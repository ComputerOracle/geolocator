use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Mutex;
use std::time::Instant;

/// Request payload for POST /distance containing two GPS coordinate pairs.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Coords {
    pub lat1: f64,
    pub lon1: f64,
    pub lat2: f64,
    pub lon2: f64,
}

/// Phone-reported Bluetooth state sent as part of a proximity inquiry.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct BluetoothInfo {
    pub device_id: String,
    pub bluetooth_enabled: bool,
    pub target_detected: bool,
    pub rssi: i32,
}

/// Response payload returned by POST /distance.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DistanceResponse {
    pub distance_meters: f64,
    pub estimated_steps: u64,
    pub threshold: u64,
    pub within_threshold: bool,
    pub bearing_degrees: f64,
    pub direction: &'static str,
}

/// Request payload for POST /proximity combining GPS points and reported Bluetooth state.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ProximityRequest {
    pub lat1: f64,
    pub lon1: f64,
    pub lat2: f64,
    pub lon2: f64,
    pub bluetooth: BluetoothInfo,
}

/// Combined verdict response returned by POST /proximity.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProximityResponse {
    pub device_id: String,
    pub rssi: i32,
    pub bluetooth_detected: bool,
    pub gps_distance_meters: f64,
    pub estimated_steps: u64,
    pub threshold: u64,
    pub within_threshold: bool,
    pub bearing_degrees: f64,
    pub direction: &'static str,
}

/// Telemetry item for a nearby detected Bluetooth device returned by GET /scan.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanResult {
    pub name: String,
    pub rssi: i16,
    pub estimated_distance_meters: f64,
    pub estimated_steps: u64,
}

/// Internal state representation for a seen Bluetooth peripheral.
#[derive(Debug, Clone)]
pub struct SeenDevice {
    pub name: String,
    pub rssi: i16,
    pub last_seen: Instant,
}

/// Thread-safe in-memory cache of nearby discovered Bluetooth devices.
pub type DeviceCache = Mutex<HashMap<String, SeenDevice>>;
