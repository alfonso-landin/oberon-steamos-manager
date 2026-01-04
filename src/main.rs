use tokio::select;
use tokio::signal::unix::{SignalKind, signal};

use oberon_steamos_manager::{OberonService, GpuPerformanceLevel1};

#[tokio::main(flavor = "current_thread")]
async fn main() -> zbus::Result<()> {
    println!("Initializing ...");
    let connection = zbus::Connection::system().await?;
    let service = OberonService::new(connection.clone()).await;

    connection
        .request_name("dev.landin.SteamOSManager1")
        .await?;
    connection
        .object_server()
        .at(
            "/dev/landin/SteamOSManager1",
            GpuPerformanceLevel1 { service },
        )
        .await?;

    let mut sig_int = signal(SignalKind::interrupt())?;
    let mut sig_term = signal(SignalKind::terminate())?;
    let mut sig_hup = signal(SignalKind::hangup())?;

    println!("Listening");
    select! {
        _ = sig_int.recv() => {}
        _ = sig_term.recv() => {}
        _ = sig_hup.recv() => {}
    }
    println!("Exiting");
    Ok(())
}
