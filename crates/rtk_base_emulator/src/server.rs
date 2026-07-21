use std::net::SocketAddr;
use std::sync::Arc;

use tokio::sync::{Mutex, broadcast, mpsc};
use tokio::task::JoinHandle;

use crate::control::{ControlServer, ControlShared};
use crate::rtcm::RtcmFrameBuilder;
use crate::rtk_base_emulator_state::{RtkBaseEmulatorState, RtkEmulatorMode};
use crate::ubx::{CfgHandler, NavEncoder, StreamParser};
use chrono::Utc;
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

pub const RTK_BASE_EMULATOR_PORT: u16 = 5782;

/// TCP-сервер эмулятора RTK-базы: UBX + RTCM.
pub struct RtkBaseEmulatorServer {
    data_bind: SocketAddr,
    control_bind: SocketAddr,
    state: Arc<Mutex<RtkBaseEmulatorState>>,
    panel_tx: broadcast::Sender<String>,
    client_out: Arc<Mutex<Option<mpsc::UnboundedSender<Vec<u8>>>>>,
    control: Option<ControlServer>,
    tasks: Vec<JoinHandle<()>>,
    shutdown_tx: Option<tokio::sync::watch::Sender<bool>>,
}

impl RtkBaseEmulatorServer {
    pub fn new(data_bind: SocketAddr, control_bind: SocketAddr) -> Self {
        let (panel_tx, _) = broadcast::channel(64);
        Self {
            data_bind,
            control_bind,
            state: Arc::new(Mutex::new(RtkBaseEmulatorState::new())),
            panel_tx,
            client_out: Arc::new(Mutex::new(None)),
            control: None,
            tasks: Vec::new(),
            shutdown_tx: None,
        }
    }

    pub fn control_panel_url(&self) -> String {
        self.control.as_ref().map(|c| c.url()).unwrap_or_else(|| {
            let host = match self.control_bind.ip() {
                std::net::IpAddr::V4(v) if v.is_unspecified() => "127.0.0.1".to_string(),
                ip => ip.to_string(),
            };
            format!("http://{}:{}", host, self.control_bind.port())
        })
    }

    pub async fn state_snapshot(&self) -> RtkBaseEmulatorState {
        self.state.lock().await.clone()
    }

    pub async fn with_state_mut<R>(&self, f: impl FnOnce(&mut RtkBaseEmulatorState) -> R) -> R {
        let mut state = self.state.lock().await;
        let result = f(&mut state);
        let payload = state.to_panel_json_string();
        let _ = self.panel_tx.send(payload);
        result
    }

    pub async fn start(&mut self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let shared = ControlShared {
            state: Arc::clone(&self.state),
            panel_tx: self.panel_tx.clone(),
        };
        let mut control = ControlServer::new(shared, self.control_bind);
        control.start().await?;
        self.control = Some(control);

        let (shutdown_tx, shutdown_rx) = tokio::sync::watch::channel(false);
        self.shutdown_tx = Some(shutdown_tx);

        let data_listener = TcpListener::bind(self.data_bind).await?;
        let state = Arc::clone(&self.state);
        let panel_tx = self.panel_tx.clone();
        let client_out = Arc::clone(&self.client_out);
        let mut data_shutdown = shutdown_rx.clone();
        self.tasks.push(tokio::spawn(async move {
            let mut client_task: Option<JoinHandle<()>> = None;
            loop {
                tokio::select! {
                    _ = data_shutdown.changed() => {
                        if *data_shutdown.borrow() {
                            if let Some(t) = client_task.take() {
                                t.abort();
                            }
                            break;
                        }
                    }
                    accept = data_listener.accept() => {
                        let Ok((stream, _)) = accept else { continue };
                        if let Some(t) = client_task.take() {
                            t.abort();
                            let _ = t.await;
                        }
                        client_task = Some(tokio::spawn(handle_client(
                            stream,
                            Arc::clone(&state),
                            panel_tx.clone(),
                            Arc::clone(&client_out),
                            data_shutdown.clone(),
                        )));
                    }
                }
            }
        }));

        let state = Arc::clone(&self.state);
        let panel_tx = self.panel_tx.clone();
        let client_out = Arc::clone(&self.client_out);
        let mut tick_shutdown = shutdown_rx;
        let nav = NavEncoder::new();
        let rtcm = RtcmFrameBuilder::default();
        self.tasks.push(tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(1));
            loop {
                tokio::select! {
                    _ = tick_shutdown.changed() => {
                        if *tick_shutdown.borrow() {
                            break;
                        }
                    }
                    _ = interval.tick() => {
                        on_tick(&state, &panel_tx, &client_out, &nav, &rtcm).await;
                    }
                }
            }
        }));

        {
            let mut state = self.state.lock().await;
            state.add_log(&format!(
                "Эмулятор RTK-базы запущен на :{}",
                self.data_bind.port()
            ));
        }
        self.broadcast_panel();

        Ok(())
    }

    pub async fn stop(&mut self) {
        if let Some(tx) = self.shutdown_tx.take() {
            let _ = tx.send(true);
        }
        for task in self.tasks.drain(..) {
            task.abort();
            let _ = task.await;
        }
        if let Some(mut control) = self.control.take() {
            control.stop().await;
        }
        *self.client_out.lock().await = None;
    }

    fn broadcast_panel(&self) {
        if let Some(control) = &self.control {
            control.broadcast_state();
        }
    }
}

async fn on_tick(
    state: &Arc<Mutex<RtkBaseEmulatorState>>,
    panel_tx: &broadcast::Sender<String>,
    client_out: &Arc<Mutex<Option<mpsc::UnboundedSender<Vec<u8>>>>>,
    nav: &NavEncoder,
    rtcm: &RtcmFrameBuilder,
) {
    let utc = Utc::now();
    let mut packets: Vec<Vec<u8>> = Vec::new();
    let payload: String;

    {
        let mut st = state.lock().await;
        st.tick_survey();

        let has_client = client_out.lock().await.is_some();
        if has_client {
            packets.push(nav.pack_nav_pvt(&st, utc));

            if st.mode == RtkEmulatorMode::SurveyIn
                || st.mode == RtkEmulatorMode::Fixed
                || st.survey_active
                || st.survey_valid
            {
                packets.push(nav.pack_nav_svin(&st, utc));
            }

            if st.should_emit_rtcm() {
                packets.extend(rtcm.build_cadence(st.latitude, st.longitude, st.height_msl, utc));
            }
        }

        payload = st.to_panel_json_string();
    }

    if let Some(tx) = client_out.lock().await.as_ref() {
        for packet in packets {
            let _ = tx.send(packet);
        }
    }

    let _ = panel_tx.send(payload);
}

async fn handle_client(
    stream: TcpStream,
    state: Arc<Mutex<RtkBaseEmulatorState>>,
    panel_tx: broadcast::Sender<String>,
    client_out: Arc<Mutex<Option<mpsc::UnboundedSender<Vec<u8>>>>>,
    mut shutdown: tokio::sync::watch::Receiver<bool>,
) {
    let (mut reader, mut writer) = stream.into_split();
    let (tx, mut rx) = mpsc::unbounded_channel::<Vec<u8>>();
    *client_out.lock().await = Some(tx);

    {
        let mut st = state.lock().await;
        st.client_connected = true;
        st.add_log("Клиент подключён");
        let _ = panel_tx.send(st.to_panel_json_string());
    }

    let write_task = tokio::spawn(async move {
        while let Some(bytes) = rx.recv().await {
            if writer.write_all(&bytes).await.is_err() {
                break;
            }
        }
    });

    let mut parser = StreamParser::new();
    let mut buf = vec![0u8; 4096];
    loop {
        tokio::select! {
            _ = shutdown.changed() => {
                if *shutdown.borrow() {
                    break;
                }
            }
            read = reader.read(&mut buf) => {
                match read {
                    Ok(0) => break,
                    Ok(n) => {
                        let frames = parser.add_data(&buf[..n]);
                        for frame in frames {
                            let out = Arc::clone(&client_out);
                            let mut handler = CfgHandler::new(
                                move |bytes: &[u8]| {
                                    if let Ok(guard) = out.try_lock() {
                                        if let Some(tx) = guard.as_ref() {
                                            let _ = tx.send(bytes.to_vec());
                                        }
                                    }
                                },
                                None::<fn()>,
                            );
                            let mut st = state.lock().await;
                            handler.handle_frame(&mut st, &frame);
                            let _ = panel_tx.send(st.to_panel_json_string());
                        }
                    }
                    Err(_) => break,
                }
            }
        }
    }

    write_task.abort();
    let _ = write_task.await;
    *client_out.lock().await = None;
    parser.clear();

    {
        let mut st = state.lock().await;
        st.client_connected = false;
        st.add_log("Клиент отключён");
        let _ = panel_tx.send(st.to_panel_json_string());
    }
}
