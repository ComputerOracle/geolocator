use crate::bluetooth::FRESHNESS_WINDOW;
use crate::errors::AppError;
use crate::math::{
    bearing_degrees, direction_from_bearing, haversine_distance_meters, rssi_to_distance_meters,
    STEP_LENGTH_M, STEP_THRESHOLD,
};
use crate::models::{
    Coords, DeviceCache, DistanceResponse, ProximityRequest, ProximityResponse, ScanResult,
};
use actix_web::{get, post, web, HttpResponse, Responder};

/// Health check endpoint.
#[get("/")]
pub async fn hello() -> impl Responder {
    HttpResponse::Ok().body("Server is up! 🚀")
}

/// Distance calculation endpoint using GPS coordinates.
#[post("/distance")]
pub async fn distance(coords: web::Json<Coords>) -> Result<HttpResponse, AppError> {
    coords.validate()?;

    let meters = haversine_distance_meters(coords.lat1, coords.lon1, coords.lat2, coords.lon2);
    let steps = (meters / STEP_LENGTH_M).round() as u64;
    let bearing = bearing_degrees(coords.lat1, coords.lon1, coords.lat2, coords.lon2);

    Ok(HttpResponse::Ok().json(DistanceResponse {
        distance_meters: meters,
        estimated_steps: steps,
        threshold: STEP_THRESHOLD,
        within_threshold: steps <= STEP_THRESHOLD,
        bearing_degrees: bearing,
        direction: direction_from_bearing(bearing),
    }))
}

/// Combined proximity assessment using GPS distance and reported Bluetooth state.
#[post("/proximity")]
pub async fn proximity(req: web::Json<ProximityRequest>) -> Result<HttpResponse, AppError> {
    req.validate()?;

    let meters = haversine_distance_meters(req.lat1, req.lon1, req.lat2, req.lon2);
    let steps = (meters / STEP_LENGTH_M).round() as u64;
    let bearing = bearing_degrees(req.lat1, req.lon1, req.lat2, req.lon2);
    let bluetooth_detected = req.bluetooth.bluetooth_enabled && req.bluetooth.target_detected;

    Ok(HttpResponse::Ok().json(ProximityResponse {
        device_id: req.bluetooth.device_id.clone(),
        rssi: req.bluetooth.rssi,
        bluetooth_detected,
        gps_distance_meters: meters,
        estimated_steps: steps,
        threshold: STEP_THRESHOLD,
        within_threshold: steps <= STEP_THRESHOLD,
        bearing_degrees: bearing,
        direction: direction_from_bearing(bearing),
    }))
}

/// Instant nearby device scan reading from the shared Bluetooth cache.
#[get("/scan")]
pub async fn scan(cache: web::Data<DeviceCache>) -> Result<HttpResponse, AppError> {
    let seen = cache
        .lock()
        .map_err(|_| AppError::LockError("Failed to acquire lock on device cache".to_string()))?;

    let mut results: Vec<ScanResult> = seen
        .values()
        .filter(|device| device.last_seen.elapsed() < FRESHNESS_WINDOW)
        .map(|device| {
            let dist_m = rssi_to_distance_meters(device.rssi);
            ScanResult {
                name: device.name.clone(),
                rssi: device.rssi,
                estimated_distance_meters: dist_m,
                estimated_steps: (dist_m / STEP_LENGTH_M).round() as u64,
            }
        })
        .collect();

    // Sort closest device first
    results.sort_by(|a, b| a.estimated_distance_meters.total_cmp(&b.estimated_distance_meters));

    Ok(HttpResponse::Ok().json(results))
}
