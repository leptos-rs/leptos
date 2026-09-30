# Real‑time multi‑UI sync with CRDTs (crdt-kit + Leptos)

This example shows how to build a responsive, multi-user UI where any participant (server or client) can edit shared state, and everyone converges on the same result—without race conditions or “last tap wins” bugs.

We use:
- crdt-kit for Conflict‑free Replicated Data Types (CRDTs), specifically LWWRegister per field
- Leptos (WASM) on the client for a reactive UI
- Axum + WebSockets on the server for simple broadcast transport

Why CRDTs for UI?
- Local-first UX: update the UI instantly as the user drags a slider; no spinner, no blocking round-trip.
- Conflict-free: concurrent edits from different clients merge deterministically.
- Clock drift tolerant: HybridClock-based timestamps give causal, last-writer-wins ordering.
- Simple mental model: treat every field as a last-writer-wins register; whoever writes “later” (by HybridTimestamp) wins, regardless of network delays.

## What we’re syncing

A “flight controls” struct where each field is an LWWRegister:
- thrust, heading, altitude, flaps, speed, spoilers (f64)
- autopilot_on, gear_down (bool)
- mode (String)
- target_altitude (f64)

Every node (server and clients) can write to any field. All nodes converge to the same state.

## How it works (architecture)

- Server (Axum):
  - Holds the authoritative copy of `Controls`.
  - Merges any incoming client state via LWWRegister.merge.
  - Broadcasts the full, merged state to all clients over WebSocket.
  - Demonstrates “server writes” by periodically changing heading to show live updates.

- Clients (Leptos/WASM):
  - Maintain a local `Controls` using a HybridClock tied to a time source.
  - On user interaction (e.g., slider):
    1) Optimistically set the local CRDT value with the client clock.
    2) Render immediately (no lag).
    3) Serialize and send the full state to the server.
  - On server broadcasts, replace local state with the merged state.

The rule: your local edit sticks unless a higher‑timestamped write (from anyone) arrives.

## Why this feels great in UI

- Sliders and fast controls: You can emit frequent updates (on:input) without flicker or jitter.
- Multi-UI sync: Open two browsers, drag a slider in one; the other updates smoothly.
- Resilience: If two users change the same field, the one with the higher HybridTimestamp wins, consistently across all replicas.

## Run the example

Prereqs:
- Rust (stable), Node (optional for trunk), Trunk, wasm32 target

Setup:
- rustup target add wasm32-unknown-unknown
- cargo install trunk

Terminals:
1) Server
   - cd server
   - cargo run
   - Server runs at ws://127.0.0.1:3000/ws

2) Client
   - cd client
   - trunk serve
   - Open http://127.0.0.1:8080

Open two browser windows to see multi-UI sync. Watch the server randomly update heading; drag the “Thrust” slider on either client and see both UIs converge.

## Key implementation points

- CRDTs:
  - Each field is LWWRegister<T>.
  - Merge strategy: keep the value with the highest HybridTimestamp.
  - Server merges incoming client states then rebroadcasts the “winning” state.

- Clocks:
  - Server uses `HybridClock::new(0)` (node_id 0).
  - Clients use `HybridClock::with_time_source(node_id, now_u64)`, with a random node_id per tab.
  - Avoid reusing the same node_id across active clients.

- Transport:
  - We send the full `Controls` as JSON. This keeps the example simple.
  - You can optimize to deltas/ops later if state grows.

## Extending the model

To add a new field (e.g., vertical_speed: f64):
1) Add `pub vertical_speed: LWWRegister<f64>` to `Controls` on both server and client.
2) Initialize it in `Controls::new(...)` on both sides with the respective clock.
3) Add to `merge(&mut self, other: &Controls)`.
4) Wire it to the UI in Leptos (display and input).
5) The rest (serialization, transport, convergence) continues to work as-is.

Tips:
- Keep `Controls` Serde-friendly (derive Serialize/Deserialize).
- For forms/sliders, update on input for responsiveness; CRDTs handle the rest.

## When to consider CRDTs for your app

- Multiple users or devices editing the same data, even intermittently offline.
- You want instantaneous UI feedback with eventual global consistency.
- You need conflict-free merges without central locking or complex consensus.

## Notes and gotchas

- Timestamps: Use HybridClock everywhere (server and clients). Don’t manually craft timestamps.
- Node IDs: Ensure uniqueness per active device/tab. Reusing IDs can break causal ordering.
- Security/validation: This example trusts clients. In production, validate inputs and auth users.
- Bandwidth: Full-state JSON is fine for demos; consider ops or selective field updates for scale.

## References

- crdt-kit: https://docs.rs/crdt-kit/latest/crdt_kit/
- Leptos: https://leptos.dev/
- Axum: https://docs.rs/axum

Happy syncing!