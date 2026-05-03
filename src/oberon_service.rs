use futures_util::stream::StreamExt;
use std::io::Write;
use std::sync::Arc;
use tokio::fs::OpenOptions;
use tokio::io::AsyncWriteExt;
use tokio::sync::{Mutex, mpsc};
use tokio::task::JoinHandle;
use zbus::Connection;

use crate::upower_profiles::UPowerProfilesProxy;

pub enum OberonServiceMode {
    Auto,
    Manual,
}

pub struct OberonService {
    current_mode: OberonServiceMode,
    current_manual_clock: u32,
    connection: Connection,
    auto_task_handle: Option<JoinHandle<zbus::Result<()>>>,
    upower_profile_changed_tx: mpsc::Sender<String>,
}

// These are known good values at the cutoff frequencies
// TODO: interpolate values?
fn voltage_for_clock(clock: u32) -> u32 {
    if clock <= 1000 {
        700
    } else if clock <= 1500 {
        850
    } else if clock <= 2000 {
        1000
    } else {
        1100
    }
}

async fn set_clock(clock: u32) -> std::io::Result<()> {
    println!("Setting GPU clock to {} MHz", clock);
    let voltage = voltage_for_clock(clock);
    let mut buffer = [0u8; 30];
    let mut slice = &mut buffer[..];
    writeln!(slice, "vc 0 {clock} {voltage}")?;
    let mut file = OpenOptions::new()
        .write(true)
        .open("/sys/class/drm/card1/device/pp_od_clk_voltage")
        .await?;
    file.write_all(&buffer).await?;
    file.flush().await?;
    file.write_all(b"c\n").await?;
    file.flush().await?;
    Ok(())
}

async fn power_profile_change_listener(
    service: Arc<Mutex<OberonService>>,
    mut rx: mpsc::Receiver<String>,
) -> std::io::Result<()> {
    while let Some(new_profile) = rx.recv().await {
        println!(
            "Received power profile change notification: {}",
            new_profile
        );
        let service = service.lock().await;
        match service.current_mode {
            OberonServiceMode::Auto => match new_profile.as_str() {
                "power-saver" => {
                    set_clock(1000).await?;
                }
                "balanced" => {
                    set_clock(1500).await?;
                }
                "performance" => {
                    set_clock(2000).await?;
                }
                _ => {
                    println!("Unknown power profile: {}", new_profile);
                }
            },
            OberonServiceMode::Manual => {
                println!("In manual mode, ignoring power profile changes");
            }
        }
    }
    println!("Power profile change listener exiting");
    Ok(())
}

async fn spawn_auto_mode_task(
    connection: Connection,
    power_profile_changed_tx: mpsc::Sender<String>,
) -> JoinHandle<zbus::Result<()>> {
    tokio::spawn(async move {
        let upower_proxy = UPowerProfilesProxy::new(&connection).await?;
        let mut profile_change_stream = upower_proxy.receive_active_profile_changed().await;
        while let Some(property_changed) = profile_change_stream.next().await {
            let new_profile = property_changed.get().await?;
            println!("Detected UPower profile change: {}", new_profile);
            power_profile_changed_tx
                .send(new_profile)
                .await
                .expect("power profile channel closed")
        }
        Ok(())
    })
}

impl OberonService {
    pub async fn new(connection: Connection) -> Arc<Mutex<Self>> {
        let (tx, rx) = mpsc::channel(10);
        let s = Self {
            current_mode: OberonServiceMode::Auto,
            current_manual_clock: 1500,
            auto_task_handle: Some(spawn_auto_mode_task(connection.clone(), tx.clone()).await),
            connection,
            upower_profile_changed_tx: tx,
        };
        let service = Arc::new(Mutex::new(s));
        {
            let service = service.clone();
            tokio::spawn(async move {
                let result = power_profile_change_listener(service.clone(), rx).await;
                println!("Power profile change listener exited: {:?}", result);
            });
        }
        service
    }

    pub fn current_mode(&self) -> &OberonServiceMode {
        &self.current_mode
    }

    pub async fn set_mode(&mut self, mode: OberonServiceMode) -> std::io::Result<()> {
        match &self.current_mode {
            OberonServiceMode::Auto => {
                if let Some(handle) = self.auto_task_handle.take() {
                    handle.abort();
                    println!("Aborted auto mode task");
                }
            }
            OberonServiceMode::Manual => (),
        }
        self.current_mode = mode;
        match &self.current_mode {
            OberonServiceMode::Auto => {
                self.auto_task_handle = Some(
                    spawn_auto_mode_task(
                        self.connection.clone(),
                        self.upower_profile_changed_tx.clone(),
                    )
                    .await,
                );
                println!("Spawned auto mode task");
            }
            OberonServiceMode::Manual => set_clock(self.current_manual_clock).await?,
        }
        Ok(())
    }

    pub fn manual_clock(&self) -> u32 {
        self.current_manual_clock
    }

    pub async fn set_manual_clock(&mut self, clock: u32) -> std::io::Result<()> {
        let clock = clock.clamp(500, 2200);
        self.current_manual_clock = clock;
        if let OberonServiceMode::Manual = self.current_mode {
            set_clock(clock).await?;
        }
        Ok(())
    }
}
