use std::sync::Arc;

use axum::{
    Router,
    extract::{
        State,
        ws::{Message, WebSocket, WebSocketUpgrade},
    },
    response::{Html, Response},
    routing::get,
};
use futures_util::{SinkExt, StreamExt};
use tokio::net::TcpListener;
use tokio::sync::{Mutex, broadcast};

use crate::control::RTK_CONTROL_PAGE_HTML;
use crate::rtk_base_emulator_state::{RtkBaseEmulatorState, RtkEmulatorMode};

#[derive(Clone)]
pub struct ControlShared {
    pub state: Arc<Mutex<RtkBaseEmulatorState>>,
    pub panel_tx: broadcast::Sender<String>,
}

/// HTTP/WebSocket-сервер веб-панели эмулятора RTK-базы.
pub struct ControlServer {
    shared: ControlShared,
    bind: std::net::SocketAddr,
    shutdown_tx: Option<tokio::sync::oneshot::Sender<()>>,
    join: Option<tokio::task::JoinHandle<()>>,
}

impl ControlServer {
    pub fn new(shared: ControlShared, bind: std::net::SocketAddr) -> Self {
        Self {
            shared,
            bind,
            shutdown_tx: None,
            join: None,
        }
    }

    pub fn url(&self) -> String {
        let host = match self.bind.ip() {
            std::net::IpAddr::V4(v) if v.is_unspecified() => "127.0.0.1".to_string(),
            ip => ip.to_string(),
        };
        format!("http://{}:{}", host, self.bind.port())
    }

    pub async fn start(&mut self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let listener = TcpListener::bind(self.bind).await?;
        let app = Router::new()
            .route("/", get(html_handler))
            .route("/ws", get(ws_handler))
            .with_state(self.shared.clone());

        let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel::<()>();
        self.shutdown_tx = Some(shutdown_tx);

        self.join = Some(tokio::spawn(async move {
            let _ = axum::serve(listener, app)
                .with_graceful_shutdown(async {
                    let _ = shutdown_rx.await;
                })
                .await;
        }));

        Ok(())
    }

    pub async fn stop(&mut self) {
        if let Some(tx) = self.shutdown_tx.take() {
            let _ = tx.send(());
        }
        if let Some(join) = self.join.take() {
            let _ = join.await;
        }
    }

    pub fn broadcast_state(&self) {
        let Ok(state) = self.shared.state.try_lock() else {
            return;
        };
        let _ = self.shared.panel_tx.send(state.to_panel_json_string());
    }
}

async fn html_handler() -> Html<&'static str> {
    Html(RTK_CONTROL_PAGE_HTML)
}

async fn ws_handler(ws: WebSocketUpgrade, State(shared): State<ControlShared>) -> Response {
    ws.on_upgrade(move |socket| handle_socket(socket, shared))
}

async fn handle_socket(socket: WebSocket, shared: ControlShared) {
    let (mut sink, mut stream) = socket.split();
    let mut rx = shared.panel_tx.subscribe();

    {
        let state = shared.state.lock().await;
        let _ = sink
            .send(Message::Text(state.to_panel_json_string().into()))
            .await;
    }

    let writer = tokio::spawn(async move {
        loop {
            match rx.recv().await {
                Ok(msg) => {
                    if sink.send(Message::Text(msg.into())).await.is_err() {
                        break;
                    }
                }
                Err(broadcast::error::RecvError::Lagged(_)) => continue,
                Err(broadcast::error::RecvError::Closed) => break,
            }
        }
    });

    while let Some(Ok(msg)) = stream.next().await {
        let Message::Text(text) = msg else {
            continue;
        };
        handle_client_message(&shared, &text).await;
    }

    writer.abort();
}

async fn handle_client_message(shared: &ControlShared, message: &str) {
    let Ok(value) = serde_json::from_str::<serde_json::Value>(message) else {
        return;
    };
    let Some(msg_type) = value.get("type").and_then(|v| v.as_str()) else {
        return;
    };

    let mut state = shared.state.lock().await;
    match msg_type {
        "set" => apply_set(&mut state, &value),
        "cmd" => apply_cmd(&mut state, value.get("cmd").and_then(|v| v.as_str())),
        _ => {}
    }
    let payload = state.to_panel_json_string();
    let _ = shared.panel_tx.send(payload);
}

fn apply_set(state: &mut RtkBaseEmulatorState, json: &serde_json::Value) {
    if let Some(v) = json.get("latitude").and_then(|v| v.as_f64()) {
        state.latitude = v;
    }
    if let Some(v) = json.get("longitude").and_then(|v| v.as_f64()) {
        state.longitude = v;
    }
    if let Some(v) = json.get("heightMsl").and_then(|v| v.as_f64()) {
        state.height_msl = v;
    }
    if let Some(v) = json.get("surveyQuality").and_then(|v| v.as_f64()) {
        state.survey_quality = v.clamp(0.0, 1.0);
    }
    if let Some(v) = json.get("meanAccOverride") {
        if v.is_null() {
            state.mean_acc_override = None;
        } else if let Some(n) = v.as_f64() {
            state.mean_acc_override = Some(n);
            state.mean_acc_meters = n;
        }
    }
}

fn apply_cmd(state: &mut RtkBaseEmulatorState, cmd: Option<&str>) {
    match cmd {
        Some("forceValid") => {
            state.force_fail = false;
            state.force_valid = true;
            state.survey_valid = true;
            state.survey_active = false;
            if state.mode == RtkEmulatorMode::Disabled {
                state.mode = RtkEmulatorMode::SurveyIn;
            }
            state.add_log("Force valid с панели");
        }
        Some("forceFail") => {
            state.force_valid = false;
            state.force_fail = true;
            state.survey_valid = false;
            state.add_log("Force fail с панели (RTCM остановлен)");
        }
        Some("clearForce") => {
            state.force_valid = false;
            state.force_fail = false;
            state.add_log("Force-флаги сняты");
        }
        Some("reset") => {
            state.reset_survey();
        }
        _ => {}
    }
}
