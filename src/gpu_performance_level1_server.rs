use std::sync::Arc;
use tokio::sync::Mutex;
use zbus::interface;

use crate::oberon_service::{OberonService, OberonServiceMode};

#[derive(Clone)]
pub struct GpuPerformanceLevel1 {
    pub service: Arc<Mutex<OberonService>>,
}

#[interface(name = "com.steampowered.SteamOSManager1.GpuPerformanceLevel1")]
impl GpuPerformanceLevel1 {
    #[zbus(property)]
    async fn available_gpu_performance_levels(&self) -> Vec<String> {
        println!("Available levels requested");
        vec!["auto".into(), "manual".into()]
    }

    #[zbus(property)]
    async fn gpu_performance_level(&self) -> String {
        let s = self.service.lock().await;
        let performance_level = match &s.current_mode() {
            OberonServiceMode::Auto => "auto",
            OberonServiceMode::Manual => "manual",
        };
        println!("Current performance level requested: {}", performance_level);
        performance_level.into()
    }

    #[zbus(property)]
    async fn set_gpu_performance_level(&self, value: &str) -> zbus::Result<()> {
        let mut s = self.service.lock().await;
        println!("Setting performance level to: {}", value);
        match value {
            "auto" => {
                s.set_mode(OberonServiceMode::Auto).await?;
            }
            "manual" => {
                s.set_mode(OberonServiceMode::Manual).await?;
            }
            _ => {
                println!("Unknown performance level: {}", value);
            }
        }
        Ok(())
    }

    #[zbus(property)]
    async fn manual_gpu_clock(&self) -> u32 {
        let s = self.service.lock().await;
        let manual_clock = s.manual_clock();
        println!("Current manual GPU clock requested: {}", manual_clock);
        manual_clock
    }

    #[zbus(property)]
    async fn set_manual_gpu_clock(&self, value: u32) -> zbus::Result<()> {
        let mut s = self.service.lock().await;
        println!("Setting manual GPU clock to: {}", value);
        s.set_manual_clock(value).await?;
        Ok(())
    }

    #[zbus(property)]
    async fn manual_gpu_clock_max(&self) -> u32 {
        println!("Max manual GPU clock requested");
        2200
    }

    #[zbus(property)]
    async fn manual_gpu_clock_min(&self) -> u32 {
        println!("Min manual GPU clock requested");
        500
    }
}
