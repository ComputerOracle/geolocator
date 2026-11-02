use actix_web::{test, web, App};
use geolocator::handlers;
use geolocator::models::{Coords, DeviceCache, DistanceResponse, GeofenceRequest, GeofenceResponse};
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
