use crate::events::{DeviceEvent, EventSender};
use crate::math::rssi_to_distance_meters;
use crate::models::{DeviceCache, SeenDevice};
use actix_web::web;
use btleplug::api::{Central, CentralEvent, Manager as _, Peripheral as _, ScanFilter};
use btleplug::platform::Manager;
use futures::stream::StreamExt;
use std::time::Duration;
use tokio::time::sleep;

/// Threshold window for considering a detected device "fresh" and actively in-range (40 seconds).
pub const FRESHNESS_WINDOW: Duration = Duration::from_secs(40);

/// Long-running background supervisor task ensuring Bluetooth scanning runs continuously.
pub async fn run_bluetooth_scanner(cache: web::Data<DeviceCache>, event_sender: web::Data<EventSender>) {
    loop {
        if let Err(e) = scan_until_disconnected(&cache, &event_sender).await {
            eprintln!("[bluetooth] scanner error: {e} - retrying in 3s");
        } else {
            eprintln!("[bluetooth] connection dropped - retrying in 3s");
        }
        sleep(Duration::from_secs(3)).await;
    }
}

/// Discovers the primary system Bluetooth adapter, listens for BLE events, updates the cache with smoothing,
/// and broadcasts real-time telemetry events.
async fn scan_until_disconnected(
    cache: &web::Data<DeviceCache>,
    event_sender: &web::Data<EventSender>,
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
        let (id, event_kind) = match event {
            CentralEvent::RssiUpdate { id, .. } => (id, "rssi_update"),
            CentralEvent::DeviceDiscovered(id) => (id, "discovered"),
            CentralEvent::DeviceUpdated(id) => (id, "updated"),
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

        let dev_id_str = format!("{id:?}");
        let name = props.local_name.unwrap_or_else(|| dev_id_str.clone());
        let dist = rssi_to_distance_meters(rssi);

        if let Ok(mut guard) = cache.lock() {
            guard
                .entry(dev_id_str.clone())
                .and_modify(|d| {
                    d.name = name.clone();
                    d.record_reading(rssi, dist);
                })
                .or_insert_with(|| SeenDevice::new(name.clone(), rssi, dist));
        }

        // Broadcast to SSE stream subscribers
        let event_payload = DeviceEvent::new(event_kind, dev_id_str, name, rssi, dist);
        let _ = event_sender.send(event_payload);
    }

    Ok(())
}
