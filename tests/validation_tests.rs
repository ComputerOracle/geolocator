use geolocator::models::{BluetoothInfo, Coords, ProximityRequest};

#[test]
fn valid_coords_pass_validation() {
    let coords = Coords {
        lat1: 6.5244,
        lon1: 3.3792,
        lat2: 6.6018,
        lon2: 3.3515,
    };
    assert!(coords.validate().is_ok());
}

#[test]
fn out_of_range_latitude_fails() {
    let coords = Coords {
        lat1: 91.0,
        lon1: 3.3792,
        lat2: 6.6018,
        lon2: 3.3515,
    };
    let err = coords.validate().unwrap_err();
    assert!(err.to_string().contains("latitude must be between -90.0 and 90.0"));
}

#[test]
fn out_of_range_longitude_fails() {
    let coords = Coords {
        lat1: 6.5244,
        lon1: -181.0,
        lat2: 6.6018,
        lon2: 3.3515,
    };
    let err = coords.validate().unwrap_err();
    assert!(err.to_string().contains("longitude must be between -180.0 and 180.0"));
}

#[test]
fn nan_coordinate_fails() {
    let coords = Coords {
        lat1: f64::NAN,
        lon1: 3.3792,
        lat2: 6.6018,
        lon2: 3.3515,
    };
    let err = coords.validate().unwrap_err();
    assert!(err.to_string().contains("must be a finite number"));
}

#[test]
fn invalid_rssi_fails() {
    let req = ProximityRequest {
        lat1: 6.5244,
        lon1: 3.3792,
        lat2: 6.5290,
        lon2: 3.3810,
        bluetooth: BluetoothInfo {
            device_id: "phone_1".to_string(),
            bluetooth_enabled: true,
            target_detected: true,
            rssi: 10, // positive RSSI is invalid
        },
    };
    let err = req.validate().unwrap_err();
    assert!(err.to_string().contains("RSSI must be between -130 and 0 dBm"));
}

#[test]
fn geofence_validation_checks_radius() {
    use geolocator::models::GeofenceRequest;
    let req = GeofenceRequest {
        center_lat: 6.5244,
        center_lon: 3.3792,
        radius_meters: -50.0,
        target_lat: 6.5250,
        target_lon: 3.3795,
    };
    assert!(req.validate().is_err());
}
