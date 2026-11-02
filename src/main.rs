use actix_web::{web, App, HttpServer};
use geolocator::bluetooth::run_bluetooth_scanner;
use geolocator::events::{init_event_bus, EventSender};
use geolocator::handlers;
use geolocator::models::DeviceCache;
use std::collections::HashMap;
use std::sync::Mutex;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let port: u16 = std::env::var("PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(8080);
    println!("Listening on http://0.0.0.0:{port} ...");

    let device_cache: web::Data<DeviceCache> = web::Data::new(Mutex::new(HashMap::new()));
    let event_sender: web::Data<EventSender> = web::Data::new(init_event_bus());

    actix_web::rt::spawn(run_bluetooth_scanner(device_cache.clone(), event_sender.clone()));

    HttpServer::new(move || {
        App::new()
            .app_data(device_cache.clone())
            .app_data(event_sender.clone())
            .service(handlers::hello)
            .service(handlers::distance)
            .service(handlers::proximity)
            .service(handlers::scan)
            .service(handlers::events)
    })
    .bind(("0.0.0.0", port))?
    .run()
    .await
}
