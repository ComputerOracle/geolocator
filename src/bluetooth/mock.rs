use crate::events::{DeviceEvent, EventSender};
use crate::math::rssi_to_distance_meters;
use crate::models::{DeviceCache, SeenDevice};
use crate::services::metrics::TelemetryRegistry;
use actix_web::web;
use std::time::Duration;
use tokio::time::sleep;

/// Simulated background BLE beacon generator for headless CI and cloud deployments
/// where no physical Bluetooth controller is present.
pub async fn run_mock_bluetooth_scanner(
    cache: web::Data<DeviceCache>,
    event_sender: web::Data<EventSender>,
    registry: web::Data<TelemetryRegistry>,
) {
    let mock_beacons = [
        ("beacon_lab_01", "Lab Environment Sensor", -52),
        ("beacon_gateway_02", "Geo Gateway Alpha", -68),
        ("beacon_asset_03", "Asset Tracker Tag", -79),
    ];

    let mut tick: u64 = 0;
    loop {
        tick = tick.wrapping_add(1);

        for (id, name, base_rssi) in &mock_beacons {
            // Add subtle sine-wave signal drift (-4 to +4 dBm)
            let drift = ((tick as f64 * 0.2 + id.len() as f64).sin() * 4.0).round() as i16;
            let current_rssi = base_rssi + drift;
            let dist_m = rssi_to_distance_meters(current_rssi);

            if let Ok(mut guard) = cache.lock() {
                guard
                    .entry(id.to_string())
                    .and_modify(|d| {
                        d.record_reading(current_rssi, dist_m);
                    })
                    .or_insert_with(|| SeenDevice::new(name.to_string(), current_rssi, dist_m));
            }

            registry.record_ble_event();
            let evt = DeviceEvent::new("rssi_update", id.to_string(), name.to_string(), current_rssi, dist_m);
            let _ = event_sender.send(evt);
        }

        sleep(Duration::from_secs(3)).await;
    }
}
