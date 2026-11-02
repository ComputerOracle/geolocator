use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::broadcast;

/// Telemetry event emitted live to SSE streams whenever a BLE peripheral is seen or updated.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceEvent {
    pub event_type: String,
    pub id: String,
    pub name: String,
    pub rssi: i16,
    pub estimated_distance_meters: f64,
    pub timestamp_ms: u64,
}

impl DeviceEvent {
    pub fn new(event_type: &str, id: String, name: String, rssi: i16, distance_meters: f64) -> Self {
        let timestamp_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0);

        Self {
            event_type: event_type.to_string(),
            id,
            name,
            rssi,
            estimated_distance_meters: distance_meters,
            timestamp_ms,
        }
    }
}

pub type EventSender = broadcast::Sender<DeviceEvent>;

pub fn init_event_bus() -> EventSender {
    let (tx, _rx) = broadcast::channel(128);
    tx
}
