use actix_web::{get, post, web, App, HttpServer, Responder};
use btleplug::api::{Central, CentralEvent, Manager as _, Peripheral as _, ScanFilter};
use btleplug::platform::Manager;
use futures::stream::StreamExt;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};
use tokio::time::sleep;

// A "route": when someone visits "/", run this function.
#[get("/")]
async fn hello() -> impl Responder {
    "Server is up! 🚀"
}

// This shape matches the JSON we expect in the POST /distance request body.
#[derive(Deserialize)]
struct Coords {
    lat1: f64,
    lon1: f64,
    lat2: f64,
    lon2: f64,
}

// Stage 7: the Bluetooth info a phone would send along with its GPS coords.
// (We're not scanning Bluetooth ourselves - a phone would fill this in and send it to us.)
#[derive(Deserialize)]
struct BluetoothInfo {
    device_id: String,
    bluetooth_enabled: bool,
    target_detected: bool,
    rssi: i32, // signal strength in dBm, e.g. -55. Closer to 0 = stronger signal.
}

// This shape is what we send back.
#[derive(Serialize)]
struct DistanceResponse {
    distance_meters: f64,
    estimated_steps: u64,
    threshold: u64,
    within_threshold: bool,
    bearing_degrees: f64,
    direction: &'static str,
}

// Average human step length in meters.
const STEP_LENGTH_M: f64 = 0.75;

// The max steps we consider "close enough" / within range.
const STEP_THRESHOLD: u64 = 10_000;

// The Haversine formula: distance between two lat/lon points on a sphere (Earth).
fn haversine_distance_meters(lat1: f64, lon1: f64, lat2: f64, lon2: f64) -> f64 {
    let earth_radius_m = 6_371_000.0;

    let lat1_rad = lat1.to_radians();
    let lat2_rad = lat2.to_radians();
    let delta_lat = (lat2 - lat1).to_radians();
    let delta_lon = (lon2 - lon1).to_radians();

    let a = (delta_lat / 2.0).sin().powi(2)
        + lat1_rad.cos() * lat2_rad.cos() * (delta_lon / 2.0).sin().powi(2);
    let c = 2.0 * a.sqrt().atan2((1.0 - a).sqrt());

    earth_radius_m * c
}

// Which way to walk from point A to point B, in degrees (0 = North, 90 = East...).
fn bearing_degrees(lat1: f64, lon1: f64, lat2: f64, lon2: f64) -> f64 {
    let lat1_rad = lat1.to_radians();
    let lat2_rad = lat2.to_radians();
    let delta_lon = (lon2 - lon1).to_radians();

    let x = delta_lon.sin() * lat2_rad.cos();
    let y = lat1_rad.cos() * lat2_rad.sin() - lat1_rad.sin() * lat2_rad.cos() * delta_lon.cos();

    let bearing = x.atan2(y).to_degrees();
    (bearing + 360.0) % 360.0 // keep it in the 0-360 range
}

// Turn a bearing in degrees into a plain compass direction word.
fn direction_from_bearing(bearing: f64) -> &'static str {
    let directions = [
        "North",
        "Northeast",
        "East",
        "Southeast",
        "South",
        "Southwest",
        "West",
        "Northwest",
    ];
    let index = (((bearing + 22.5) / 45.0) as usize) % 8;
    directions[index]
}

// Stage 8: the request body for /proximity - two points plus the Bluetooth info.
#[derive(Deserialize)]
struct ProximityRequest {
    lat1: f64,
    lon1: f64,
    lat2: f64,
    lon2: f64,
    bluetooth: BluetoothInfo,
}

// What we send back for /proximity: GPS math + the Bluetooth verdict together.
#[derive(Serialize)]
struct ProximityResponse {
    device_id: String,
    rssi: i32,
    bluetooth_detected: bool,
    gps_distance_meters: f64,
    estimated_steps: u64,
    threshold: u64,
    within_threshold: bool,
    bearing_degrees: f64,
    direction: &'static str,
}

#[post("/proximity")]
async fn proximity(req: web::Json<ProximityRequest>) -> impl Responder {
    let meters = haversine_distance_meters(req.lat1, req.lon1, req.lat2, req.lon2);
    let steps = (meters / STEP_LENGTH_M).round() as u64;
    let bearing = bearing_degrees(req.lat1, req.lon1, req.lat2, req.lon2);

    // The "connected" verdict: Bluetooth has to be on AND actually see the other phone.
    let bluetooth_detected = req.bluetooth.bluetooth_enabled && req.bluetooth.target_detected;

    web::Json(ProximityResponse {
        device_id: req.bluetooth.device_id.clone(),
        rssi: req.bluetooth.rssi,
        bluetooth_detected,
        gps_distance_meters: meters,
        estimated_steps: steps,
        threshold: STEP_THRESHOLD,
        within_threshold: steps <= STEP_THRESHOLD,
        bearing_degrees: bearing,
        direction: direction_from_bearing(bearing),
    })
}

// What one nearby Bluetooth device looks like in the /scan response.
#[derive(Serialize)]
struct ScanResult {
    name: String,
    rssi: i16,
    estimated_distance_meters: f64,
    estimated_steps: u64,
}

// Turns Bluetooth signal strength (RSSI, in dBm) into an estimated distance in
// meters. This is the standard "log-distance path loss" formula. It's rough -
// real signal strength bounces around a lot depending on walls, obstacles, and
// how the phone is being held - so treat the result as an estimate, not GPS-grade.
fn rssi_to_distance_meters(rssi: i16) -> f64 {
    let measured_power = -59.0; // typical RSSI at 1 meter for a BLE device
    let path_loss_exponent = 2.0; // 2 = open air, higher (3-4) = indoors/obstacles
    10f64.powf((measured_power - rssi as f64) / (10.0 * path_loss_exponent))
}

// One device we've heard a signal from recently.
struct SeenDevice {
    name: String,
    rssi: i16,
    last_seen: Instant,
}

// Shared between the background scanner task and the /scan route. Keyed by device id.
type DeviceCache = Mutex<HashMap<String, SeenDevice>>;

// Ignore devices we haven't heard a fresh signal from in this long (they're
// probably out of range / no longer nearby).
const FRESHNESS_WINDOW: Duration = Duration::from_secs(40);

// Runs for the whole life of the server: keeps Bluetooth discovery on and
// updates `cache` the instant a nearby device's signal strength changes.
// This means /scan never has to wait - it just reads whatever's in the cache.
//
// The Bluetooth connection can silently drop (e.g. the system's Bluetooth
// service hiccups) - if that ever happens we'd otherwise go quiet forever,
// so this keeps retrying instead of giving up after one attempt.
async fn run_bluetooth_scanner(cache: web::Data<DeviceCache>) {
    loop {
        if let Err(e) = scan_until_disconnected(&cache).await {
            eprintln!("[bluetooth] scanner error: {e} - retrying in 3s");
        } else {
            eprintln!("[bluetooth] connection dropped - retrying in 3s");
        }
        sleep(Duration::from_secs(3)).await;
    }
}

async fn scan_until_disconnected(
    cache: &web::Data<DeviceCache>,
) -> Result<(), Box<dyn std::error::Error>> {
    let manager = Manager::new().await?;
    let adapters = manager.adapters().await?;
    let central = adapters
        .into_iter()
        .next()
        .ok_or("no bluetooth adapter found")?;

    central.start_scan(ScanFilter::default()).await?;

    let mut events = central.events().await?;
    while let Some(event) = events.next().await {
        // A device's signal can arrive as "this changed" (RssiUpdate) or "this is
        // new to us" (DeviceDiscovered/DeviceUpdated) - we want all three, otherwise
        // we miss every device we haven't already seen before.
        let id = match event {
            CentralEvent::RssiUpdate { id, .. } => id,
            CentralEvent::DeviceDiscovered(id) => id,
            CentralEvent::DeviceUpdated(id) => id,
            _ => continue,
        };

        let Ok(peripheral) = central.peripheral(&id).await else {
            continue;
        };
        let Ok(Some(props)) = peripheral.properties().await else {
            continue;
        };
        let Some(rssi) = props.rssi else {
            continue;
        };

        let name = props.local_name.unwrap_or_else(|| format!("{id:?}"));
        cache.lock().unwrap().insert(
            format!("{id:?}"),
            SeenDevice {
                name,
                rssi,
                last_seen: Instant::now(),
            },
        );
    }

    Ok(()) // the event stream ended on its own - scan_bluetooth_scanner will retry
}

// Reports every nearby device we've recently heard a real signal from, as an
// estimated distance/steps. Answers instantly from the shared cache.
#[get("/scan")]
async fn scan(cache: web::Data<DeviceCache>) -> impl Responder {
    let seen = cache.lock().unwrap();
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

    // Closest device first - the nearest signal is usually what you actually care about.
    results.sort_by(|a, b| a.estimated_distance_meters.total_cmp(&b.estimated_distance_meters));

    web::Json(results)
}

// Stage 3: calculate the real distance and return it as JSON.
#[post("/distance")]
async fn distance(coords: web::Json<Coords>) -> impl Responder {
    let meters = haversine_distance_meters(coords.lat1, coords.lon1, coords.lat2, coords.lon2);
    let steps = (meters / STEP_LENGTH_M).round() as u64;
    let bearing = bearing_degrees(coords.lat1, coords.lon1, coords.lat2, coords.lon2);
    web::Json(DistanceResponse {
        distance_meters: meters,
        estimated_steps: steps,
        threshold: STEP_THRESHOLD,
        within_threshold: steps <= STEP_THRESHOLD,
        bearing_degrees: bearing,
        direction: direction_from_bearing(bearing),
    })
}

// The entry point. actix_web::main turns main() into an async function for us.
#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // Render (and most cloud hosts) assign their own port via $PORT - fall back
    // to 8080 for running it locally, where that variable won't be set.
    let port: u16 = std::env::var("PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(8080);
    println!("Listening on http://0.0.0.0:{port} ...");

    let device_cache: web::Data<DeviceCache> = web::Data::new(Mutex::new(HashMap::new()));

    // Start the Bluetooth scanner once, in the background, for as long as the server runs.
    actix_web::rt::spawn(run_bluetooth_scanner(device_cache.clone()));

    HttpServer::new(move || {
        App::new()
            .app_data(device_cache.clone())
            .service(hello)
            .service(distance)
            .service(proximity)
            .service(scan)
    })
        .bind(("0.0.0.0", port))?
        .run()
        .await
}

// Stage 9: a couple of quick tests for the plain math functions (no server needed).
// Run these with `cargo test`.
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn haversine_same_point_is_zero() {
        let dist = haversine_distance_meters(6.5244, 3.3792, 6.5244, 3.3792);
        assert!(dist < 0.001);
    }

    #[test]
    fn direction_matches_known_bearings() {
        assert_eq!(direction_from_bearing(0.0), "North");
        assert_eq!(direction_from_bearing(90.0), "East");
        assert_eq!(direction_from_bearing(180.0), "South");
        assert_eq!(direction_from_bearing(270.0), "West");
    }
}
