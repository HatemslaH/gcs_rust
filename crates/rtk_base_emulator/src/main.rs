use std::net::{IpAddr, Ipv4Addr};

use rtk_base_emulator::{Emulator, EmulatorConfig, RTK_BASE_EMULATOR_PORT, RTK_CONTROL_PORT};

#[tokio::main]
async fn main() -> rtk_base_emulator::Result<()> {
    let config = EmulatorConfig {
        bind: IpAddr::V4(Ipv4Addr::LOCALHOST),
        data_port: RTK_BASE_EMULATOR_PORT,
        control_port: RTK_CONTROL_PORT,
    };

    let mut emulator = Emulator::new(config);
    emulator.start().await?;

    println!("Emulator is running: {}", emulator.is_running());
    println!("Data TCP: {}", emulator.data_endpoint());
    println!("Control panel: {}", emulator.control_panel_url());

    tokio::signal::ctrl_c().await?;

    emulator.stop().await?;
    Ok(())
}
