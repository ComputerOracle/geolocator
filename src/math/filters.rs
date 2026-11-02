/// 1D Kalman Filter for smoothing noisy Bluetooth RSSI signals.
#[derive(Debug, Clone)]
pub struct RssiKalmanFilter {
    process_noise: f64,   // Q: process noise covariance
    measurement_noise: f64, // R: measurement noise covariance
    estimated_value: f64, // X: state estimate
    error_covariance: f64, // P: estimate error covariance
    initialized: bool,
}

impl Default for RssiKalmanFilter {
    fn default() -> Self {
        Self {
            process_noise: 0.125,
            measurement_noise: 0.8,
            estimated_value: -70.0,
            error_covariance: 1.0,
            initialized: false,
        }
    }
}

impl RssiKalmanFilter {
    pub fn new(process_noise: f64, measurement_noise: f64) -> Self {
        Self {
            process_noise,
            measurement_noise,
            estimated_value: -70.0,
            error_covariance: 1.0,
            initialized: false,
        }
    }

    /// Feeds a new raw RSSI observation into the filter and returns the smoothed estimate.
    pub fn update(&mut self, measurement: f64) -> f64 {
        if !self.initialized {
            self.estimated_value = measurement;
            self.initialized = true;
            return measurement;
        }

        // Prediction update
        let prior_covariance = self.error_covariance + self.process_noise;

        // Measurement update (Kalman Gain)
        let kalman_gain = prior_covariance / (prior_covariance + self.measurement_noise);
        self.estimated_value += kalman_gain * (measurement - self.estimated_value);
        self.error_covariance = (1.0 - kalman_gain) * prior_covariance;

        self.estimated_value
    }

    pub fn current_estimate(&self) -> f64 {
        self.estimated_value
    }
}

/// Computes the proximity trend (approaching, receding, or stationary)
/// based on the trajectory of smoothed RSSI readings.
pub fn calculate_trend(readings: &[i16]) -> &'static str {
    if readings.len() < 3 {
        return "stationary";
    }

    let n = readings.len();
    let recent_avg: f64 = readings[n - 2..].iter().map(|&r| r as f64).sum::<f64>() / 2.0;
    let earlier_avg: f64 = readings[..2].iter().map(|&r| r as f64).sum::<f64>() / 2.0;

    let delta = recent_avg - earlier_avg;

    // In RSSI, higher (closer to 0) means closer physically.
    if delta > 3.0 {
        "approaching"
    } else if delta < -3.0 {
        "receding"
    } else {
        "stationary"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kalman_filter_smooths_noisy_rssi() {
        let mut filter = RssiKalmanFilter::default();
        let noisy_sequence = [-60.0, -85.0, -58.0, -90.0, -62.0, -61.0];
        let mut smoothed = 0.0;
        for &val in &noisy_sequence {
            smoothed = filter.update(val);
        }

        // After multiple updates around -60, estimate should hover stably near -65..-60
        assert!(smoothed > -75.0 && smoothed < -55.0);
    }

    #[test]
    fn trend_detects_approaching_signal() {
        let approaching_rssi = vec![-85, -80, -72, -65, -55];
        assert_eq!(calculate_trend(&approaching_rssi), "approaching");
    }

    #[test]
    fn trend_detects_receding_signal() {
        let receding_rssi = vec![-50, -60, -70, -80, -90];
        assert_eq!(calculate_trend(&receding_rssi), "receding");
    }
}
