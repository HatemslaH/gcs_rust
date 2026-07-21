use std::net::TcpListener;
use std::time::Duration;

use rtk_base_emulator::{Emulator, EmulatorConfig, EmulatorError};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::time::timeout;

fn free_port() -> u16 {
    TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port()
}

#[tokio::test]
async fn public_api_start_stop_and_snapshot() {
    let data_port = free_port();
    let control_port = free_port();
    let mut emu = Emulator::new(EmulatorConfig::localhost(data_port, control_port));

    assert!(!emu.is_running());
    emu.start().await.unwrap();
    assert!(emu.is_running());

    let snap = emu.snapshot().await;
    assert!(snap.is_running);
    assert!(snap.control_panel_url.contains(&control_port.to_string()));
    assert_eq!(snap.mode, "disabled");

    emu.set_position(55.0, 37.0, 100.0).await;
    emu.start_survey_in().await;
    let snap = emu.snapshot().await;
    assert_eq!(snap.mode, "surveyIn");
    assert!((snap.latitude - 55.0).abs() < 1e-9);

    emu.stop().await.unwrap();
    assert!(!emu.is_running());
}

#[tokio::test]
async fn start_twice_returns_already_running() {
    let mut emu = Emulator::new(EmulatorConfig::localhost(free_port(), free_port()));
    emu.start().await.unwrap();
    let err = emu.start().await.unwrap_err();
    assert!(matches!(err, EmulatorError::AlreadyRunning));
    emu.stop().await.unwrap();
}

#[tokio::test]
async fn data_port_accepts_tcp_client() {
    let data_port = free_port();
    let mut emu = Emulator::new(EmulatorConfig::localhost(data_port, free_port()));
    emu.start().await.unwrap();

    let mut stream = timeout(
        Duration::from_secs(2),
        TcpStream::connect(("127.0.0.1", data_port)),
    )
    .await
    .unwrap()
    .unwrap();

    // После подключения тик шлёт NAV-PVT (sync B5 62).
    let mut buf = [0u8; 8];
    let n = timeout(Duration::from_secs(3), stream.read(&mut buf))
        .await
        .unwrap()
        .unwrap();
    assert!(n >= 2);
    assert_eq!(buf[0], 0xb5);
    assert_eq!(buf[1], 0x62);

    let _ = stream.write_all(&[0]).await;
    emu.stop().await.unwrap();
}
