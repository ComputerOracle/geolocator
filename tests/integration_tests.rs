use actix_web::{test, web, App};
use geolocator::handlers;
use geolocator::models::{
    Coords, DeviceCache, DistanceResponse, GeofenceRequest, GeofenceResponse,
    NearestDeviceResponse, ScanResult, SeenDevice,
};
use geolocator::services::metrics::TelemetryRegistry;
use std::collections::HashMap;
use std::sync::Mutex;

#[actix_web::test]
async fn test_root_hello() {
    let app = test::init_service(App::new().service(handlers::hello)).await;
    let req = test::TestRequest::get().uri("/").to_request();
    let resp = test::call_service(&app, req).await;
    assert!(resp.status().is_success());
}

#[actix_web::test]
async fn test_health_and_metrics() {
    let cache: web::Data<DeviceCache> = web::Data::new(Mutex::new(HashMap::new()));
    let registry: web::Data<TelemetryRegistry> = web::Data::new(TelemetryRegistry::default());

    let app = test::init_service(
        App::new()
            .app_data(cache.clone())
            .app_data(registry.clone())
            .service(handlers::health)
            .service(handlers::metrics),
    )
    .await;

    let req = test::TestRequest::get().uri("/health").to_request();
    let resp = test::call_service(&app, req).await;
    assert!(resp.status().is_success());

    let req_m = test::TestRequest::get().uri("/metrics").to_request();
    let resp_m = test::call_service(&app, req_m).await;
    assert!(resp_m.status().is_success());
}

#[actix_web::test]
async fn test_distance_endpoint() {
    let cache: web::Data<DeviceCache> = web::Data::new(Mutex::new(HashMap::new()));
    let registry: web::Data<TelemetryRegistry> = web::Data::new(TelemetryRegistry::default());

    let app = test::init_service(
        App::new()
            .app_data(cache.clone())
            .app_data(registry.clone())
            .service(handlers::distance),
    )
    .await;

    let payload = Coords {
        lat1: 6.5244,
        lon1: 3.3792,
        lat2: 6.6018,
        lon2: 3.3515,
    };

    let req = test::TestRequest::post()
        .uri("/distance?unit=km")
        .set_json(&payload)
        .to_request();

    let resp: DistanceResponse = test::call_and_read_body_json(&app, req).await;
    assert!(resp.distance > 8.0 && resp.distance < 12.0);
    assert_eq!(resp.unit, "km");
    assert!(resp.estimated_steps > 10_000);
}

#[actix_web::test]
async fn test_geofence_endpoint() {
    let registry: web::Data<TelemetryRegistry> = web::Data::new(TelemetryRegistry::default());

    let app = test::init_service(
        App::new()
            .app_data(registry.clone())
            .service(handlers::geofence),
    )
    .await;

    let payload = GeofenceRequest {
        center_lat: 6.5244,
        center_lon: 3.3792,
        radius_meters: 5000.0,
        target_lat: 6.5250,
        target_lon: 3.3795,
    };

    let req = test::TestRequest::post()
        .uri("/geofence")
        .set_json(&payload)
        .to_request();

    let resp: GeofenceResponse = test::call_and_read_body_json(&app, req).await;
    assert!(resp.is_inside);
    assert_eq!(resp.status, "inside");
    assert!(resp.margin_meters > 0.0);
}

#[actix_web::test]
async fn test_bearing_math_edge_cases() {
    use geolocator::math::{bearing_degrees, direction_from_bearing};

    // North to South
    let b1 = bearing_degrees(10.0, 0.0, 5.0, 0.0);
    assert!((b1 - 180.0).abs() < 1e-4);
    assert_eq!(direction_from_bearing(b1), "South");

    // West to East along equator
    let b2 = bearing_degrees(0.0, 0.0, 0.0, 10.0);
    assert!((b2 - 90.0).abs() < 1e-4);
    assert_eq!(direction_from_bearing(b2), "East");
}

#[actix_web::test]
async fn test_nearest_device_and_scan_time_to_cover() {
    let cache: web::Data<DeviceCache> = web::Data::new(Mutex::new(HashMap::new()));
    let registry: web::Data<TelemetryRegistry> = web::Data::new(TelemetryRegistry::default());

    // Populate mock devices in cache:
    // dev_far: -79 dBm (~10.0m)
    // dev_near: -59 dBm (~1.0m)
    {
        let mut guard = cache.lock().unwrap();
        guard.insert(
            "dev_far".to_string(),
            SeenDevice::new("Far Beacon".to_string(), -79, 10.0),
        );
        guard.insert(
            "dev_near".to_string(),
            SeenDevice::new("Near Beacon".to_string(), -59, 1.0),
        );
    }

    let app = test::init_service(
        App::new()
            .app_data(cache.clone())
            .app_data(registry.clone())
            .service(handlers::scan)
            .service(handlers::nearest_device),
    )
    .await;

    // Test GET /scan with estimated_time_to_cover_seconds
    let scan_req = test::TestRequest::get().uri("/scan").to_request();
    let scan_resp: Vec<ScanResult> = test::call_and_read_body_json(&app, scan_req).await;
    assert_eq!(scan_resp.len(), 2);
    assert_eq!(scan_resp[0].id, "dev_near");
    assert!(scan_resp[0].estimated_time_to_cover_seconds > 0.0);

    // Test GET /nearest tracking factor endpoint
    let nearest_req = test::TestRequest::get().uri("/nearest").to_request();
    let nearest_resp: NearestDeviceResponse = test::call_and_read_body_json(&app, nearest_req).await;
    assert!(nearest_resp.nearest_device.is_some());
    let nearest = nearest_resp.nearest_device.unwrap();
    assert_eq!(nearest.id, "dev_near");
    assert_eq!(nearest.name, "Near Beacon");
    assert!(nearest_resp.estimated_time_to_cover_seconds.is_some());
    assert!(!nearest_resp.estimated_time_to_cover_formatted.is_empty());
    assert_eq!(nearest_resp.active_devices_count, 2);
}
