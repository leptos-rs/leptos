use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        State,
    },
    response::IntoResponse,
    routing::get,
    Router,
};
use crdt_kit::{clock::HybridClock, prelude::*};
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::sync::broadcast;

// ====================== SHARED STATE ======================
#[derive(Clone, Debug, Serialize, Deserialize)]
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
    pub fn new(node_id: u64) -> Self {
        let mut clock = HybridClock::new(node_id);
        Self {
            thrust: LWWRegister::new(10.0, &mut clock),
            heading: LWWRegister::new(20.0, &mut clock),
            altitude: LWWRegister::new(10000.0, &mut clock),
            flaps: LWWRegister::new(30.0, &mut clock),
            speed: LWWRegister::new(40.0, &mut clock),
            autopilot_on: LWWRegister::new(false, &mut clock),
            mode: LWWRegister::new("manual".to_string(), &mut clock),
            target_altitude: LWWRegister::new(10000.0, &mut clock),
            gear_down: LWWRegister::new(true, &mut clock),
            spoilers: LWWRegister::new(0.0, &mut clock),
        }
    }

    pub fn merge(&mut self, other: &Controls) {
        self.thrust.merge(&other.thrust);
        self.heading.merge(&other.heading);
        self.altitude.merge(&other.altitude);
        self.flaps.merge(&other.flaps);
        self.speed.merge(&other.speed);
        self.autopilot_on.merge(&other.autopilot_on);
        self.mode.merge(&other.mode);
        self.target_altitude.merge(&other.target_altitude);
        self.gear_down.merge(&other.gear_down);
        self.spoilers.merge(&other.spoilers);
    }
}

// ====================== SERVER ======================
#[derive(Clone)]
struct AppState {
    controls: Arc<Mutex<Controls>>,
    tx: broadcast::Sender<Controls>,
    clock: Arc<Mutex<HybridClock>>,
}

#[tokio::main]
async fn main() {
    let controls = Arc::new(Mutex::new(Controls::new(0))); // Server node ID = 0
    let (tx, _) = broadcast::channel(100);
    let clock = Arc::new(Mutex::new(HybridClock::new(0)));

    let app_state = AppState {
        controls,
        tx: tx.clone(),
        clock: clock.clone(),
    };
    tokio::spawn(server_updates_loop(app_state.clone()));

    let app = Router::new()
        .route("/ws", get(ws_handler))
        .with_state(app_state);

    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000")
        .await
        .unwrap();
    println!("✅ Complete server running on ws://127.0.0.1:3000/ws");
    println!("Clients can connect and send updates. Server merges with LWW.");
    axum::serve(listener, app).await.unwrap();
}

async fn server_updates_loop(state: AppState) {
    loop {
        // sleep 2–5 seconds
        let delay_ms = rand::random_range(200..=2000);
        tokio::time::sleep(Duration::from_millis(delay_ms)).await;

        // compute a new heading (random 0..360). You could also do a small increment and wrap.
        let new_heading: f64 = rand::random_range(0.0..360.0);

        // perform CRDT write with the server clock
        {
            let mut controls = state.controls.lock().unwrap();
            let mut clock = state.clock.lock().unwrap();
            controls.heading.set(new_heading, &mut *clock);
            // after mutation, broadcast full state to clients
            let _ = state.tx.send(controls.clone());
        }
    }
}

async fn ws_handler(ws: WebSocketUpgrade, State(state): State<AppState>) -> impl IntoResponse {
    ws.on_upgrade(|socket| handle_socket(socket, state))
}

async fn handle_socket(mut socket: WebSocket, app_state: AppState) {
    let mut rx = app_state.tx.subscribe();

    // Send current state on connect
    {
        let current = app_state.controls.lock().unwrap().clone();
        let _ = socket
            .send(Message::Text(serde_json::to_string(&current).unwrap()))
            .await;
    }

    loop {
        tokio::select! {
            // Receive from client
            msg = socket.recv() => {
                // println!("WS message received");
                match msg {

                    Some(Ok(Message::Text(text))) => {
                        // println!(" -> WSmessage is text with: {}", text);
                        if let Ok(update) = serde_json::from_str::<Controls>(&text) {
                            //println!("update message (from client): {:#?}", update);
                            println!("update message (from client)");
                            let mut guard = app_state.controls.lock().unwrap();
                            guard.merge(&update);
                            // Broadcast winning state
                            let _ = app_state.tx.send(guard.clone());
                        }
                    }
                    Some(Ok(Message::Close(_))) | None => break,
                    _ => {
                        println!("message received binary/pong");
                    } // ignore binary/pong etc.
                }
            }

            // Broadcast incoming updates from other clients
            Ok(msg) = rx.recv() => {
                //println!("update message (from server): {:#?}", serde_json::to_string(&msg).unwrap());
                let _ = socket.send(Message::Text(serde_json::to_string(&msg).unwrap())).await;
            }
        }
    }
}
