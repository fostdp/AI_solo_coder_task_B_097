use crate::constants::*;
use crate::dynasty_models::*;
use crate::optics::OpticalSimulator;

pub struct PrecisionComparator;

impl PrecisionComparator {
    pub fn compare(request: &DynastyComparisonRequest) -> Vec<DynastyComparisonResult> {
        let presets = DynastyGnomon::presets();
        let mut results = Vec::new();

        for gnomon in &presets {
            let sim = OpticalSimulator::new(
                gnomon.latitude,
                gnomon.longitude,
                gnomon.altitude,
            );

            let refraction_arcsec = sim.calculate_refraction_arcsec(
                request.sun_altitude,
                request.temperature,
                request.pressure,
            );

            let true_alt = request.sun_altitude - refraction_arcsec * ARCSEC_TO_DEG;
            let apparent_alt = sim.correct_for_refraction(
                true_alt,
                request.temperature,
                request.pressure,
            );

            let theoretical_shadow = sim.shadow_length_from_altitude(
                gnomon.gauge_height_chi,
                true_alt,
            );
            let refracted_shadow = sim.shadow_length_from_altitude(
                gnomon.gauge_height_chi,
                apparent_alt,
            );

            let shadow_precision_cun = gnomon.shadow_reading_error_std_cun;
            let alt_resolution_rad = shadow_precision_cun / (CHI_TO_CUN * gnomon.gauge_height_chi);
            let alt_resolution_arcmin = alt_resolution_rad * (180.0 / std::f64::consts::PI) * 60.0;

            let solstice_precision_seconds = Self::estimate_solstice_precision(
                gnomon.gauge_height_chi,
                gnomon.gauge_height_error_std_chi,
                gnomon.shadow_reading_error_std_cun,
                request.sun_altitude,
            );

            results.push(DynastyComparisonResult {
                dynasty_id: gnomon.dynasty_id.clone(),
                dynasty_name: gnomon.dynasty_name.clone(),
                gauge_height_chi: gnomon.gauge_height_chi,
                gauge_height_m_actual: gnomon.gauge_height_m_actual,
                chi_to_m_conversion: gnomon.chi_to_m_conversion,
                gauge_material: gnomon.gauge_material.clone(),
                theoretical_shadow_chi: theoretical_shadow,
                refracted_shadow_chi: refracted_shadow,
                refraction_correction_arcsec: refraction_arcsec,
                shadow_precision_cun,
                solstice_precision_seconds,
                altitude_resolution_arcmin: alt_resolution_arcmin,
                archaeological_source: gnomon.archaeological_source.clone(),
            });
        }

        results
    }

    pub fn estimate_solstice_precision(
        gauge_height_chi: f64,
        gauge_height_error_chi: f64,
        shadow_reading_error_cun: f64,
        sun_altitude: f64,
    ) -> f64 {
        let alt_rad = sun_altitude.max(1.0) * DEG_TO_RAD;
        let tan_alt = alt_rad.tan();

        let dshadow_dgauge = 1.0 / tan_alt;
        let dshadow_dalt = -gauge_height_chi / (alt_rad.sin() * alt_rad.sin());

        let shadow_err_gauge = gauge_height_error_chi * dshadow_dgauge;
        let shadow_err_reading = shadow_reading_error_cun / CHI_TO_CUN;
        let shadow_err_alt = dshadow_dalt * 0.0005;

        let total_shadow_err = (shadow_err_gauge.powi(2)
            + shadow_err_reading.powi(2)
            + shadow_err_alt.powi(2)).sqrt();

        let dsolstice_dshadow = 86400.0 / (2.0 * std::f64::consts::PI / 365.25 * tan_alt);
        total_shadow_err * dsolstice_dshadow.abs()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_precision_comparison_normal() {
        let req = DynastyComparisonRequest {
            sun_altitude: 26.0,
            temperature: 5.0,
            pressure: 1013.25,
            humidity: 50.0,
        };
        let results = PrecisionComparator::compare(&req);
        assert_eq!(results.len(), 3);
        assert!(results[2].solstice_precision_seconds < results[0].solstice_precision_seconds);
    }

    #[test]
    fn test_precision_comparison_boundary_low_altitude() {
        let req = DynastyComparisonRequest {
            sun_altitude: 5.0,
            temperature: -10.0,
            pressure: 1030.0,
            humidity: 30.0,
        };
        let results = PrecisionComparator::compare(&req);
        assert_eq!(results.len(), 3);
        for r in &results {
            assert!(r.refraction_correction_arcsec > 10.0);
        }
    }

    #[test]
    fn test_precision_comparison_anomalous_nan_input() {
        let req = DynastyComparisonRequest {
            sun_altitude: f64::NAN,
            temperature: 20.0,
            pressure: 1013.25,
            humidity: 50.0,
        };
        let results = PrecisionComparator::compare(&req);
        assert_eq!(results.len(), 3);
    }

    #[test]
    fn test_estimate_solstice_precision_known_value() {
        let precision = PrecisionComparator::estimate_solstice_precision(40.0, 0.01, 0.2, 26.0);
        assert!(precision > 0.0);
        assert!(precision < 1000.0);
    }
}
