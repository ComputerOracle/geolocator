use crate::errors::AppError;
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

impl Coords {
    pub fn validate(&self) -> Result<(), AppError> {
        validate_lat_lon(self.lat1, self.lon1, "Point 1")?;
        validate_lat_lon(self.lat2, self.lon2, "Point 2")?;
        Ok(())
    }
}

/// Phone-reported Bluetooth state sent as part of a proximity inquiry.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct BluetoothInfo {
    pub device_id: String,
    pub bluetooth_enabled: bool,
    pub target_detected: bool,
    pub rssi: i32,
}

impl BluetoothInfo {
    pub fn validate(&self) -> Result<(), AppError> {
        if self.device_id.trim().is_empty() {
            return Err(AppError::InvalidCoordinates("device_id cannot be empty".to_string()));
        }
        if self.rssi > 0 || self.rssi < -130 {
            return Err(AppError::InvalidRssi(format!(
                "RSSI must be between -130 and 0 dBm, received: {}",
                self.rssi
            )));
        }
        Ok(())
    }
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

impl ProximityRequest {
    pub fn validate(&self) -> Result<(), AppError> {
        validate_lat_lon(self.lat1, self.lon1, "Point 1")?;
        validate_lat_lon(self.lat2, self.lon2, "Point 2")?;
        self.bluetooth.validate()?;
        Ok(())
    }
}

/// Helper function to validate coordinate boundaries and validity.
fn validate_lat_lon(lat: f64, lon: f64, label: &str) -> Result<(), AppError> {
    if lat.is_nan() || lat.is_infinite() {
        return Err(AppError::InvalidCoordinates(format!("{label} latitude must be a finite number")));
    }
    if lon.is_nan() || lon.is_infinite() {
        return Err(AppError::InvalidCoordinates(format!("{label} longitude must be a finite number")));
    }
    if !(-90.0..=90.0).contains(&lat) {
        return Err(AppError::InvalidCoordinates(format!(
            "{label} latitude must be between -90.0 and 90.0, received: {lat}"
        )));
    }
    if !(-180.0..=180.0).contains(&lon) {
        return Err(AppError::InvalidCoordinates(format!(
            "{label} longitude must be between -180.0 and 180.0, received: {lon}"
        )));
    }
    Ok(())
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
