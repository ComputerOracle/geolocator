use serde::Serialize;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;

/// Central thread-safe in-memory metrics and telemetry registry.
pub struct TelemetryRegistry {
    pub start_time: Instant,
    pub total_distance_requests: AtomicU64,
    pub total_proximity_requests: AtomicU64,
    pub total_geofence_checks: AtomicU64,
    pub total_scan_requests: AtomicU64,
    pub total_ble_events_dispatched: AtomicU64,
}

impl Default for TelemetryRegistry {
    fn default() -> Self {
        Self {
            start_time: Instant::now(),
            total_distance_requests: AtomicU64::new(0),
            total_proximity_requests: AtomicU64::new(0),
            total_geofence_checks: AtomicU64::new(0),
            total_scan_requests: AtomicU64::new(0),
            total_ble_events_dispatched: AtomicU64::new(0),
        }
    }
}

impl TelemetryRegistry {
    pub fn record_distance(&self) {
        self.total_distance_requests.fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_proximity(&self) {
        self.total_proximity_requests.fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_geofence(&self) {
        self.total_geofence_checks.fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_scan(&self) {
        self.total_scan_requests.fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_ble_event(&self) {
        self.total_ble_events_dispatched.fetch_add(1, Ordering::Relaxed);
    }

    pub fn snapshot(&self, active_devices: usize) -> MetricsSnapshot {
        let uptime_secs = self.start_time.elapsed().as_secs();
        let dist = self.total_distance_requests.load(Ordering::Relaxed);
        let prox = self.total_proximity_requests.load(Ordering::Relaxed);
        let geo = self.total_geofence_checks.load(Ordering::Relaxed);
        let scan = self.total_scan_requests.load(Ordering::Relaxed);
        let ble = self.total_ble_events_dispatched.load(Ordering::Relaxed);
        let total = dist + prox + geo + scan;

        MetricsSnapshot {
            status: "healthy",
            version: env!("CARGO_PKG_VERSION"),
            uptime_seconds: uptime_secs,
            total_api_requests: total,
            distance_queries: dist,
            proximity_queries: prox,
            geofence_checks: geo,
            scan_queries: scan,
            ble_events_dispatched: ble,
            currently_tracked_devices: active_devices,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct MetricsSnapshot {
    pub status: &'static str,
    pub version: &'static str,
    pub uptime_seconds: u64,
    pub total_api_requests: u64,
    pub distance_queries: u64,
    pub proximity_queries: u64,
    pub geofence_checks: u64,
    pub scan_queries: u64,
    pub ble_events_dispatched: u64,
    pub currently_tracked_devices: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn telemetry_records_counters_accurately() {
        let registry = TelemetryRegistry::default();
        registry.record_distance();
        registry.record_distance();
        registry.record_proximity();
        registry.record_geofence();
        registry.record_scan();
        registry.record_ble_event();

        let snap = registry.snapshot(5);
        assert_eq!(snap.distance_queries, 2);
        assert_eq!(snap.proximity_queries, 1);
        assert_eq!(snap.geofence_checks, 1);
        assert_eq!(snap.scan_queries, 1);
        assert_eq!(snap.total_api_requests, 5);
        assert_eq!(snap.ble_events_dispatched, 1);
        assert_eq!(snap.currently_tracked_devices, 5);
        assert_eq!(snap.status, "healthy");
    }
}
