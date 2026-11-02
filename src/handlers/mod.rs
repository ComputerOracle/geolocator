use crate::bluetooth::FRESHNESS_WINDOW;
use crate::errors::AppError;
use crate::events::EventSender;
use crate::math::filters::calculate_trend;
use crate::math::{
    bearing_degrees, direction_from_bearing, haversine_distance_meters, rssi_to_distance_meters,
    STEP_LENGTH_M, STEP_THRESHOLD,
};
use crate::models::{
    Coords, DeviceCache, DeviceHistoryResponse, DistanceResponse, ProximityRequest,
    ProximityResponse, ScanResult,
};
use actix_web::web::Bytes;
use actix_web::{get, post, web, HttpResponse, Responder};
use futures::StreamExt as _;

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

/// Instant nearby device scan reading from the shared Bluetooth cache, enhanced with signal smoothing and trends.
#[get("/scan")]
pub async fn scan(cache: web::Data<DeviceCache>) -> Result<HttpResponse, AppError> {
    let seen = cache
        .lock()
        .map_err(|_| AppError::LockError("Failed to acquire lock on device cache".to_string()))?;

    let mut results: Vec<ScanResult> = seen
        .iter()
        .filter(|(_, device)| device.last_seen.elapsed() < FRESHNESS_WINDOW)
        .map(|(id, device)| {
            let dist_m = rssi_to_distance_meters(device.rssi);
            let readings: Vec<i16> = device.history.iter().map(|r| r.rssi).collect();
            let trend = calculate_trend(&readings);

            ScanResult {
                id: id.clone(),
                name: device.name.clone(),
                rssi: device.rssi,
                smoothed_rssi: (device.smoothed_rssi * 10.0).round() / 10.0,
                estimated_distance_meters: (dist_m * 100.0).round() / 100.0,
                estimated_steps: (dist_m / STEP_LENGTH_M).round() as u64,
                trend,
            }
        })
        .collect();

    // Sort closest device first
    results.sort_by(|a, b| a.estimated_distance_meters.total_cmp(&b.estimated_distance_meters));

    Ok(HttpResponse::Ok().json(results))
}

/// Retrieves the historical trajectory and signal logs for a specific peripheral.
#[get("/history/{device_id}")]
pub async fn device_history(
    cache: web::Data<DeviceCache>,
    path: web::Path<String>,
) -> Result<HttpResponse, AppError> {
    let dev_id = path.into_inner();
    let seen = cache
        .lock()
        .map_err(|_| AppError::LockError("Failed to acquire lock on device cache".to_string()))?;

    if let Some(dev) = seen.get(&dev_id) {
        let readings: Vec<i16> = dev.history.iter().map(|r| r.rssi).collect();
        let trend = calculate_trend(&readings);

        Ok(HttpResponse::Ok().json(DeviceHistoryResponse {
            device_id: dev_id,
            name: dev.name.clone(),
            current_rssi: dev.rssi,
            smoothed_rssi: (dev.smoothed_rssi * 10.0).round() / 10.0,
            trend,
            history: dev.history.clone(),
        }))
    } else {
        Err(AppError::DeviceNotFound(dev_id))
    }
}

/// Real-time Server-Sent Events (SSE) telemetry stream of nearby BLE device discoveries and updates.
#[get("/events")]
pub async fn events(sender: web::Data<EventSender>) -> impl Responder {
    let rx = sender.subscribe();
    let stream = tokio_stream::wrappers::BroadcastStream::new(rx).filter_map(|msg| async move {
        match msg {
            Ok(evt) => {
                let json = serde_json::to_string(&evt).ok()?;
                let formatted = format!("data: {json}\n\n");
                Some(Ok::<Bytes, actix_web::Error>(Bytes::from(formatted)))
            }
            Err(_) => None,
        }
    });

    HttpResponse::Ok()
        .append_header(("content-type", "text/event-stream"))
        .append_header(("cache-control", "no-cache"))
        .append_header(("connection", "keep-alive"))
        .streaming(stream)
}
