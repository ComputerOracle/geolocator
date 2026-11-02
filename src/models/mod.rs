use crate::errors::AppError;
use crate::math::filters::RssiKalmanFilter;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Mutex;
use std::time::Instant;

/// Individual historical observation recorded in a device's telemetry log.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReadingRecord {
    pub rssi: i16,
    pub distance_meters: f64,
    pub timestamp_ms: u64,
}

/// Optional query parameters accepted by navigation endpoints.
#[derive(Debug, Clone, Deserialize, Default)]
pub struct NavQuery {
    pub unit: Option<String>,
    pub step_threshold: Option<u64>,
    pub stride_m: Option<f64>,
}

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

/// Response payload returned by POST /distance with dynamic unit support.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DistanceResponse {
    pub distance: f64,
    pub distance_meters: f64,
    pub unit: String,
    pub estimated_steps: u64,
    pub threshold: u64,
    pub within_threshold: bool,
    pub bearing_degrees: f64,
    pub direction: String,
    pub proximity_zone: String,
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

/// Request payload for circular geofence containment verification.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct GeofenceRequest {
    pub center_lat: f64,
    pub center_lon: f64,
    pub radius_meters: f64,
    pub target_lat: f64,
    pub target_lon: f64,
}

impl GeofenceRequest {
    pub fn validate(&self) -> Result<(), AppError> {
        validate_lat_lon(self.center_lat, self.center_lon, "Center")?;
        validate_lat_lon(self.target_lat, self.target_lon, "Target")?;
        if self.radius_meters <= 0.0 || self.radius_meters.is_nan() || self.radius_meters.is_infinite() {
            return Err(AppError::InvalidCoordinates(
                "radius_meters must be a positive finite number".to_string(),
            ));
        }
        Ok(())
    }
}

/// Response payload returned by POST /geofence.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeofenceResponse {
    pub is_inside: bool,
    pub distance_from_center_meters: f64,
    pub radius_meters: f64,
    pub margin_meters: f64,
    pub status: String,
    pub proximity_zone: String,
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
    pub distance: f64,
    pub gps_distance_meters: f64,
    pub unit: String,
    pub estimated_steps: u64,
    pub threshold: u64,
    pub within_threshold: bool,
    pub bearing_degrees: f64,
    pub direction: String,
    pub proximity_zone: String,
}

/// Telemetry item for a nearby detected Bluetooth device returned by GET /scan.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanResult {
    pub id: String,
    pub name: String,
    pub rssi: i16,
    pub smoothed_rssi: f64,
    pub estimated_distance_meters: f64,
    pub estimated_steps: u64,
    pub trend: String,
}

/// Full historical report for a specific peripheral returned by GET /history/{id}.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceHistoryResponse {
    pub device_id: String,
    pub name: String,
    pub current_rssi: i16,
    pub smoothed_rssi: f64,
    pub trend: String,
    pub history: Vec<ReadingRecord>,
}

/// Internal state representation for a seen Bluetooth peripheral with signal filter & history buffer.
#[derive(Debug, Clone)]
pub struct SeenDevice {
    pub name: String,
    pub rssi: i16,
    pub smoothed_rssi: f64,
    pub last_seen: Instant,
    pub kalman: RssiKalmanFilter,
    pub history: Vec<ReadingRecord>,
}

impl SeenDevice {
    pub fn new(name: String, initial_rssi: i16, dist_m: f64) -> Self {
        let mut kalman = RssiKalmanFilter::default();
        let smoothed = kalman.update(initial_rssi as f64);
        let timestamp_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0);

        Self {
            name,
            rssi: initial_rssi,
            smoothed_rssi: smoothed,
            last_seen: Instant::now(),
            kalman,
            history: vec![ReadingRecord {
                rssi: initial_rssi,
                distance_meters: dist_m,
                timestamp_ms,
            }],
        }
    }

    pub fn record_reading(&mut self, new_rssi: i16, dist_m: f64) {
        self.rssi = new_rssi;
        self.smoothed_rssi = self.kalman.update(new_rssi as f64);
        self.last_seen = Instant::now();

        let timestamp_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0);

        self.history.push(ReadingRecord {
            rssi: new_rssi,
            distance_meters: dist_m,
            timestamp_ms,
        });

        if self.history.len() > 25 {
            self.history.remove(0);
        }
    }
}

pub type DeviceCache = Mutex<HashMap<String, SeenDevice>>;
