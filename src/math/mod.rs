pub mod filters;
pub mod units;

/// Standard average human step length in meters.
pub const STEP_LENGTH_M: f64 = 0.75;

/// Default maximum walking steps considered within proximate range (10,000 steps).
pub const STEP_THRESHOLD: u64 = 10_000;

/// Average human walking speed in meters per second (~5.0 km/h or 1.39 m/s).
pub const WALKING_SPEED_MPS: f64 = 1.39;

/// Computes the estimated time in seconds to cover a given distance in meters based on a walking speed.
pub fn time_to_cover_seconds(distance_meters: f64, speed_mps: f64) -> f64 {
    let speed = if speed_mps <= 0.0 || speed_mps.is_nan() {
        WALKING_SPEED_MPS
    } else {
        speed_mps
    };
    distance_meters.max(0.0) / speed
}

/// The Haversine formula: calculates the great-circle distance between two GPS coordinates on Earth in meters.
pub fn haversine_distance_meters(lat1: f64, lon1: f64, lat2: f64, lon2: f64) -> f64 {
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

/// Computes the compass bearing in degrees (0..360) from point A to point B.
/// 0° = North, 90° = East, 180° = South, 270° = West.
pub fn bearing_degrees(lat1: f64, lon1: f64, lat2: f64, lon2: f64) -> f64 {
    let lat1_rad = lat1.to_radians();
    let lat2_rad = lat2.to_radians();
    let delta_lon = (lon2 - lon1).to_radians();

    let x = delta_lon.sin() * lat2_rad.cos();
    let y = lat1_rad.cos() * lat2_rad.sin() - lat1_rad.sin() * lat2_rad.cos() * delta_lon.cos();

    let bearing = x.atan2(y).to_degrees();
    (bearing + 360.0) % 360.0
}

/// Converts a compass bearing in degrees to an 8-point cardinal direction string.
pub fn direction_from_bearing(bearing: f64) -> &'static str {
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

/// Converts Bluetooth signal strength (RSSI in dBm) into an estimated physical distance in meters
/// using the log-distance path loss model.
pub fn rssi_to_distance_meters(rssi: i16) -> f64 {
    let measured_power = -59.0; // typical RSSI at 1 meter for standard BLE device
    let path_loss_exponent = 2.0; // 2.0 = free space, 3.0-4.0 = obstacles/indoors
    10f64.powf((measured_power - rssi as f64) / (10.0 * path_loss_exponent))
}

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

    #[test]
    fn rssi_at_measured_power_is_one_meter() {
        let dist = rssi_to_distance_meters(-59);
        assert!((dist - 1.0).abs() < 0.001);
    }

    #[test]
    fn time_to_cover_calculates_accurately() {
        let time = time_to_cover_seconds(139.0, 1.39);
        assert!((time - 100.0).abs() < 0.001);

        // Fallback on invalid or zero speed
        let fallback_time = time_to_cover_seconds(139.0, 0.0);
        assert!((fallback_time - 100.0).abs() < 0.001);
    }
}
