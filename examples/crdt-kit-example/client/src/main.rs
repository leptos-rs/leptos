use crdt_kit::{clock::HybridClock, prelude::*};
use futures_channel::mpsc;
use futures_util::{SinkExt, StreamExt};
use gloo_net::websocket::{futures::WebSocket, Message};
use gloo_timers::future::sleep;
use leptos::prelude::*;
use std::time::Duration;
use wasm_bindgen_futures::spawn_local;
use web_sys::console;

pub fn now_u64() -> u64 {
    let now = chrono::Local::now().timestamp_millis() as u64;
    now
}

pub fn get_hybrid_clock(node_id: u64) -> HybridClock {
    HybridClock::with_time_source(node_id, now_u64)
    //HybridClock::new(1)
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Controls {
    pub thrust: LWWRegister<f64>,
    pub heading: LWWRegister<f64>,
    pub altitude: LWWRegister<f64>,
    pub flaps: LWWRegister<f64>,
    pub speed: LWWRegister<f64>,
    pub autopilot_on: LWWRegister<bool>,
    pub mode: LWWRegister<String>,
    pub target_altitude: LWWRegister<f64>,
    pub gear_down: LWWRegister<bool>,
    pub spoilers: LWWRegister<f64>,
}

impl Controls {
    pub fn new(mut clock: HybridClock) -> Self {
        Self {
            thrust: LWWRegister::new(0.0, &mut clock),
            heading: LWWRegister::new(0.0, &mut clock),
            altitude: LWWRegister::new(0.0, &mut clock),
            flaps: LWWRegister::new(0.0, &mut clock),
            speed: LWWRegister::new(0.0, &mut clock),
            autopilot_on: LWWRegister::new(false, &mut clock),
            mode: LWWRegister::new("manual".to_string(), &mut clock),
            target_altitude: LWWRegister::new(0.0, &mut clock),
            gear_down: LWWRegister::new(true, &mut clock),
            spoilers: LWWRegister::new(0.0, &mut clock),
        }
    }
}

#[component]
pub fn App() -> impl IntoView {
    //let node_id = 1;
    let node_id: u64 = rand::random();
    web_sys::console::log_1(&format!("node_id: {}", node_id).into());

    let clock = RwSignal::new(get_hybrid_clock(node_id));
    let controls = RwSignal::new(Controls::new(clock.get()));

    // Hold a sender that UI code can use to send messages to the active socket.
    let ws_tx = RwSignal::new(None::<mpsc::UnboundedSender<Message>>);

    spawn_local({
        let controls = controls.clone();
        let ws_tx = ws_tx.clone();
        async move {
            loop {
                let url = "ws://127.0.0.1:3000/ws";
                console::log_1(&format!("Connecting to {}", url).into());

                match WebSocket::open(url) {
                    Ok(ws) => {
                        let (mut write, mut read) = ws.split();

                        // Create a channel -> writer task pumps messages to the socket.
                        let (tx, mut rx) = mpsc::unbounded::<Message>();
                        ws_tx.set(Some(tx.clone()));
                        drop(tx); // keep only the copy stored in ws_tx

                        // Background writer: forwards messages from UI to the socket.
                        spawn_local(async move {
                            while let Some(msg) = rx.next().await {
                                if let Err(e) = write.send(msg).await {
                                    console::error_1(&format!("Write error: {:?}", e).into());
                                    break;
                                }
                            }
                            console::log_1(&"📝 Writer task ended".into());
                        });

                        while let Some(msg) = read.next().await {
                            match msg {
                                Ok(Message::Text(text)) => {
                                    if let Ok(new_state) = serde_json::from_str::<Controls>(&text) {
                                        controls.set(new_state);
                                        //console::log_1(&"✅ Updated from server".into());
                                        console::log_1(
                                            &format!("✅ Updated from server {}", text).into(),
                                        );
                                    }
                                }
                                Ok(Message::Bytes(bin)) => {
                                    if let Ok(text) = std::str::from_utf8(&bin) {
                                        if let Ok(new_state) =
                                            serde_json::from_str::<Controls>(text)
                                        {
                                            controls.set(new_state);
                                        }
                                    }
                                }
                                Err(e) => {
                                    console::error_1(&format!("Read error: {:?}", e).into());
                                    break;
                                }
                            }
                        }

                        // Drop sender to stop writer task cleanly before reconnecting.
                        ws_tx.set(None);
                        console::log_1(&"🔌 Disconnected, retrying…".into());
                    }
                    Err(e) => {
                        console::error_1(&format!("Open failed: {:?}", e).into());
                        ws_tx.set(None);
                    }
                }

                sleep(Duration::from_secs(2)).await;
            }
        }
    });

    // fn print_controls(controls: &Controls) {
    //     match serde_json::to_string(controls) {
    //         Ok(json) => {
    //             console::log_1(&format!("Controls XXX print: {:?}", Message::Text(json)).into());
    //         }
    //         Err(e) => console::error_1(&format!("Controls XXX print serialize error: {}", e).into()),
    //     }
    // }

    let send_thrust = {
        let ws_tx = ws_tx.clone();
        let controls = controls.clone();
        move |new_thrust: f64| {
            let mut local = controls.get_untracked();

            // Quick sanity log to verify the local clone changed
            // console::log_1(
            //     &format!(
            //         "post-set thrust value = {:.2}, ts = {:?}",
            //         local.thrust.value(),
            //         local.thrust.timestamp()
            //     ).into()
            // );

            //print_controls(&local);
            let mut clock = clock.get();
            //console::log_1(&format!("clock.now() {:#?}", clock.now()).into());
            local.thrust.set(new_thrust, &mut clock);
            controls.set(local.clone());

            //console::log_1(&format!("Updated local thrust to {:.2}", new_thrust).into());

            if let Some(tx) = ws_tx.get() {
                match serde_json::to_string(&local) {
                    Ok(json) => {
                        let _ = tx.unbounded_send(Message::Text(json.clone()));
                        //console::log_1(&format!("➡️ Sent full state: {:?}", Message::Text(json)).into());
                        console::log_1(&format!("➡️ Sent updates to clients").into());
                    }
                    Err(e) => console::error_1(&format!("Serialize error: {}", e).into()),
                }
            }
        }
    };

    view! {
        <div style="padding: 2rem; font-family: system-ui; max-width: 800px; margin: 0 auto;">
            <h1>"Flight Simulator - Leptos 0.8 Client"</h1>

            <p><strong>Thrust:</strong> {move || format!("{:.2}", controls.get().thrust.value())}</p>
            <p><strong>Heading:</strong> {move || format!("{:.1}°", controls.get().heading.value())}</p>
            <p><strong>Altitude:</strong> {move || format!("{:.0} ft", controls.get().altitude.value())}</p>
            <p><strong>Flaps:</strong> {move || format!("{:.1}", controls.get().flaps.value())}</p>
            <p><strong>Speed:</strong> {move || format!("{:.1} kts", controls.get().speed.value())}</p>
            <p><strong>Autopilot:</strong> {move || controls.get().autopilot_on.value().to_string()}</p>
            <p><strong>Mode:</strong> {move || controls.get().mode.value().clone()}</p>

            // ... existing code ...

            // New: Thrust slider (0.00–1.00), updates as you move it
            <div style="margin-top: 0.75rem;">
                <label for="thrust-slider"><strong>Set Thrust:</strong></label>
                <input
                    id="thrust-slider"
                    type="range"
                    min="0"
                    max="1"
                    step="0.01"
                    // keep the slider in sync with the current thrust
                    prop:value=move || controls.get().thrust.value().to_string()
                    // send updates while sliding
                    on:input=move |ev| {
                        if let Ok(v) = event_target_value(&ev).parse::<f64>() {
                            send_thrust(v);
                        }
                    }
                />
            </div>

            <div style="margin-top: 1rem;">
                <button on:click=move |_| send_thrust(0.75)>"Thrust → 0.75"</button>
                <button on:click=move |_| send_thrust(0.92)>"Thrust → 0.92"</button>
            </div>
        </div>
    }
}

fn main() {
    mount_to_body(App)
}
