pub mod bluetooth;
pub mod handlers;
pub mod math;
pub mod models;

use actix_web::{web, App, HttpServer};
use bluetooth::run_bluetooth_scanner;
use models::DeviceCache;
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

    // Launch background Bluetooth scanning supervisor
    actix_web::rt::spawn(run_bluetooth_scanner(device_cache.clone()));

    HttpServer::new(move || {
        App::new()
            .app_data(device_cache.clone())
            .service(handlers::hello)
            .service(handlers::distance)
            .service(handlers::proximity)
            .service(handlers::scan)
    })
    .bind(("0.0.0.0", port))?
    .run()
    .await
}
