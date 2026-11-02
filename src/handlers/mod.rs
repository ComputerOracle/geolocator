use crate::bluetooth::FRESHNESS_WINDOW;
use crate::errors::AppError;
use crate::events::EventSender;
use crate::math::filters::calculate_trend;
use crate::math::units::{classify_proximity_zone, DistanceUnit};
use crate::math::{
    bearing_degrees, direction_from_bearing, haversine_distance_meters, rssi_to_distance_meters,
    STEP_LENGTH_M, STEP_THRESHOLD,
};
use crate::models::{
    Coords, DeviceCache, DeviceHistoryResponse, DistanceResponse, GeofenceRequest,
    GeofenceResponse, NavQuery, ProximityRequest, ProximityResponse, ScanResult,
};
use crate::services::metrics::TelemetryRegistry;
use actix_web::web::Bytes;
use actix_web::{get, post, web, HttpResponse, Responder};
use futures::StreamExt as _;

/// Root ping endpoint.
#[get("/")]
pub async fn hello() -> impl Responder {
    HttpResponse::Ok().body("Server is up! 🚀")
}

/// System health check endpoint providing service uptime and basic operational flags.
#[get("/health")]
pub async fn health(
    registry: web::Data<TelemetryRegistry>,
    cache: web::Data<DeviceCache>,
) -> impl Responder {
    let active_count = cache
        .lock()
        .map(|c| c.values().filter(|d| d.last_seen.elapsed() < FRESHNESS_WINDOW).count())
        .unwrap_or(0);

    HttpResponse::Ok().json(registry.snapshot(active_count))
}

/// Operational telemetry metrics report.
#[get("/metrics")]
pub async fn metrics(
    registry: web::Data<TelemetryRegistry>,
    cache: web::Data<DeviceCache>,
) -> impl Responder {
    let active_count = cache
        .lock()
        .map(|c| c.values().filter(|d| d.last_seen.elapsed() < FRESHNESS_WINDOW).count())
        .unwrap_or(0);

    HttpResponse::Ok().json(registry.snapshot(active_count))
}

/// Distance calculation endpoint with multi-unit support, dynamic thresholds, and step estimations.
#[post("/distance")]
pub async fn distance(
    coords: web::Json<Coords>,
    query: web::Query<NavQuery>,
    registry: web::Data<TelemetryRegistry>,
) -> Result<HttpResponse, AppError> {
    registry.record_distance();
    coords.validate()?;

    let unit = DistanceUnit::from_str_opt(query.unit.as_deref());
    let threshold = query.step_threshold.unwrap_or(STEP_THRESHOLD);
    let stride = query.stride_m.unwrap_or(STEP_LENGTH_M).max(0.2);

    let meters = haversine_distance_meters(coords.lat1, coords.lon1, coords.lat2, coords.lon2);
    let converted_dist = unit.convert_from_meters(meters);
    let steps = (meters / stride).round() as u64;
    let bearing = bearing_degrees(coords.lat1, coords.lon1, coords.lat2, coords.lon2);
    let zone = classify_proximity_zone(meters);

    Ok(HttpResponse::Ok().json(DistanceResponse {
        distance: (converted_dist * 100.0).round() / 100.0,
        distance_meters: (meters * 100.0).round() / 100.0,
        unit: unit.label().to_string(),
        estimated_steps: steps,
        threshold,
        within_threshold: steps <= threshold,
        bearing_degrees: (bearing * 10.0).round() / 10.0,
        direction: direction_from_bearing(bearing).to_string(),
        proximity_zone: zone.to_string(),
    }))
}

/// Proximity verdict evaluating GPS points and device-reported Bluetooth telemetry.
#[post("/proximity")]
pub async fn proximity(
    req: web::Json<ProximityRequest>,
    query: web::Query<NavQuery>,
    registry: web::Data<TelemetryRegistry>,
) -> Result<HttpResponse, AppError> {
    registry.record_proximity();
    req.validate()?;

    let unit = DistanceUnit::from_str_opt(query.unit.as_deref());
    let threshold = query.step_threshold.unwrap_or(STEP_THRESHOLD);
    let stride = query.stride_m.unwrap_or(STEP_LENGTH_M).max(0.2);

    let meters = haversine_distance_meters(req.lat1, req.lon1, req.lat2, req.lon2);
    let converted_dist = unit.convert_from_meters(meters);
    let steps = (meters / stride).round() as u64;
    let bearing = bearing_degrees(req.lat1, req.lon1, req.lat2, req.lon2);
    let bluetooth_detected = req.bluetooth.bluetooth_enabled && req.bluetooth.target_detected;
    let zone = classify_proximity_zone(meters);

    Ok(HttpResponse::Ok().json(ProximityResponse {
        device_id: req.bluetooth.device_id.clone(),
        rssi: req.bluetooth.rssi,
        bluetooth_detected,
        distance: (converted_dist * 100.0).round() / 100.0,
        gps_distance_meters: (meters * 100.0).round() / 100.0,
        unit: unit.label().to_string(),
        estimated_steps: steps,
        threshold,
        within_threshold: steps <= threshold,
        bearing_degrees: (bearing * 10.0).round() / 10.0,
        direction: direction_from_bearing(bearing).to_string(),
        proximity_zone: zone.to_string(),
    }))
}

/// Evaluates whether a coordinate target is within a defined circular geofence boundary.
#[post("/geofence")]
pub async fn geofence(
    req: web::Json<GeofenceRequest>,
    registry: web::Data<TelemetryRegistry>,
) -> Result<HttpResponse, AppError> {
    registry.record_geofence();
    req.validate()?;

    let dist = haversine_distance_meters(
        req.center_lat,
        req.center_lon,
        req.target_lat,
        req.target_lon,
    );
    let margin = req.radius_meters - dist;
    let is_inside = dist <= req.radius_meters;
    let status = if (dist - req.radius_meters).abs() < 1.0 {
        "perimeter"
    } else if is_inside {
        "inside"
    } else {
        "outside"
    };

    Ok(HttpResponse::Ok().json(GeofenceResponse {
        is_inside,
        distance_from_center_meters: (dist * 100.0).round() / 100.0,
        radius_meters: req.radius_meters,
        margin_meters: (margin * 100.0).round() / 100.0,
        status: status.to_string(),
        proximity_zone: classify_proximity_zone(dist).to_string(),
    }))
}

/// Instant nearby device scan reading from the shared Bluetooth cache.
#[get("/scan")]
pub async fn scan(
    cache: web::Data<DeviceCache>,
    registry: web::Data<TelemetryRegistry>,
) -> Result<HttpResponse, AppError> {
    registry.record_scan();
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
                trend: trend.to_string(),
            }
        })
        .collect();

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
            trend: trend.to_string(),
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
