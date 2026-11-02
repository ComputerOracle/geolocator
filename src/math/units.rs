use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DistanceUnit {
    Meters,
    Kilometers,
    Miles,
    Feet,
}

impl Default for DistanceUnit {
    fn default() -> Self {
        DistanceUnit::Meters
    }
}

impl DistanceUnit {
    pub fn from_str_opt(s: Option<&str>) -> Self {
        match s.map(|v| v.to_ascii_lowercase()).as_deref() {
            Some("km") | Some("kilometers") => DistanceUnit::Kilometers,
            Some("mi") | Some("miles") => DistanceUnit::Miles,
            Some("ft") | Some("feet") => DistanceUnit::Feet,
            _ => DistanceUnit::Meters,
        }
    }

    pub fn convert_from_meters(&self, meters: f64) -> f64 {
        match self {
            DistanceUnit::Meters => meters,
            DistanceUnit::Kilometers => meters / 1000.0,
            DistanceUnit::Miles => meters / 1609.344,
            DistanceUnit::Feet => meters * 3.28084,
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            DistanceUnit::Meters => "m",
            DistanceUnit::Kilometers => "km",
            DistanceUnit::Miles => "mi",
            DistanceUnit::Feet => "ft",
        }
    }
}

/// Computes the proximity zone based on metric distance.
pub fn classify_proximity_zone(meters: f64) -> &'static str {
    if meters <= 2.0 {
        "immediate"
    } else if meters <= 10.0 {
        "near"
    } else if meters <= 50.0 {
        "vicinity"
    } else {
        "distant"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_unit_conversions() {
        let meters = 1609.344;
        let miles = DistanceUnit::Miles.convert_from_meters(meters);
        assert!((miles - 1.0).abs() < 1e-4);

        let km = DistanceUnit::Kilometers.convert_from_meters(1000.0);
        assert!((km - 1.0).abs() < 1e-4);

        let feet = DistanceUnit::Feet.convert_from_meters(1.0);
        assert!((feet - 3.28084).abs() < 1e-4);
    }

    #[test]
    fn test_proximity_zones() {
        assert_eq!(classify_proximity_zone(1.2), "immediate");
        assert_eq!(classify_proximity_zone(7.5), "near");
        assert_eq!(classify_proximity_zone(25.0), "vicinity");
        assert_eq!(classify_proximity_zone(150.0), "distant");
    }
}
