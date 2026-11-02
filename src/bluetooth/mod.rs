use crate::models::{DeviceCache, SeenDevice};
use actix_web::web;
use btleplug::api::{Central, CentralEvent, Manager as _, Peripheral as _, ScanFilter};
use btleplug::platform::Manager;
use futures::stream::StreamExt;
use std::time::{Duration, Instant};
use tokio::time::sleep;

/// Threshold window for considering a detected device "fresh" and actively in-range (40 seconds).
pub const FRESHNESS_WINDOW: Duration = Duration::from_secs(40);

/// Long-running background supervisor task ensuring Bluetooth scanning runs continuously
/// and re-establishes scanning if the adapter drops or resets.
pub async fn run_bluetooth_scanner(cache: web::Data<DeviceCache>) {
    loop {
        if let Err(e) = scan_until_disconnected(&cache).await {
            eprintln!("[bluetooth] scanner error: {e} - retrying in 3s");
        } else {
            eprintln!("[bluetooth] connection dropped - retrying in 3s");
        }
        sleep(Duration::from_secs(3)).await;
    }
}

/// Discovers the primary system Bluetooth adapter and listens for BLE advertisement events.
async fn scan_until_disconnected(
    cache: &web::Data<DeviceCache>,
) -> Result<(), Box<dyn std::error::Error>> {
    let manager = Manager::new().await?;
    let adapters = manager.adapters().await?;
    let central = adapters
        .into_iter()
        .next()
        .ok_or("no bluetooth adapter found")?;

    central.start_scan(ScanFilter::default()).await?;

    let mut events = central.events().await?;
    while let Some(event) = events.next().await {
        let id = match event {
            CentralEvent::RssiUpdate { id, .. } => id,
            CentralEvent::DeviceDiscovered(id) => id,
            CentralEvent::DeviceUpdated(id) => id,
            _ => continue,
        };

        let Ok(peripheral) = central.peripheral(&id).await else {
            continue;
        };
        let Ok(Some(props)) = peripheral.properties().await else {
            continue;
        };
        let Some(rssi) = props.rssi else {
            continue;
        };

        let name = props.local_name.unwrap_or_else(|| format!("{id:?}"));
        if let Ok(mut guard) = cache.lock() {
            guard.insert(
                format!("{id:?}"),
                SeenDevice {
                    name,
                    rssi,
                    last_seen: Instant::now(),
                },
            );
        }
    }

    Ok(())
}
