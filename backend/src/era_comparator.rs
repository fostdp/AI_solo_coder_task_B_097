use crate::constants::*;
use crate::dynasty_models::*;
use crate::optics::OpticalSimulator;

pub struct EraComparator;

impl EraComparator {
    pub fn compare(request: &MeridianComparisonRequest) -> Vec<MeridianComparisonResult> {
        let presets = MeridianCircle::presets();
        let mut results = Vec::new();

        for instrument in &presets {
            let sim = OpticalSimulator::new(
                DEFAULT_STATION_LAT,
                DEFAULT_STATION_LON,
                DEFAULT_STATION_ALT,
            );

            let refraction_arcsec = sim.calculate_refraction_arcsec(
                request.sun_altitude,
                request.temperature,
                request.pressure,
            );

            let total_altitude_error = (
                instrument.systematic_error_arcsec.powi(2)
                + instrument.random_error_arcsec.powi(2)
            ).sqrt();

            let measured_alt = request.sun_altitude + total_altitude_error * ARCSEC_TO_DEG;

            let shadow_length = if instrument.era == "1276" {
                sim.shadow_length_from_altitude(40.0, measured_alt)
            } else {
                sim.shadow_length_from_altitude(40.0, request.sun_altitude)
            };

            let shadow_error_cun = total_altitude_error * ARCSEC_TO_DEG
                * 40.0 / (request.sun_altitude.max(1.0) * DEG_TO_RAD).sin().powi(2)
                * CHI_TO_CUN;

            let solstice_error = total_altitude_error * ARCSEC_TO_DEG
                * 86400.0 / (2.0 * std::f64::consts::PI / 365.25)
                / (request.sun_altitude.max(1.0) * DEG_TO_RAD).tan();

            let gap_factor = if instrument.era == "1276" {
                1.0
            } else {
                let baseline = presets.iter()
                    .find(|p| p.era == "1276")
                    .map(|p| (p.systematic_error_arcsec.powi(2) + p.random_error_arcsec.powi(2)).sqrt())
                    .unwrap_or(60.0);
                baseline / total_altitude_error
            };

            results.push(MeridianComparisonResult {
                instrument_id: instrument.instrument_id.clone(),
                instrument_name: instrument.instrument_name.clone(),
                era: instrument.era.clone(),
                measured_altitude_deg: measured_alt,
                altitude_error_arcsec: total_altitude_error,
                systematic_error_arcsec: instrument.systematic_error_arcsec,
                random_error_arcsec: instrument.random_error_arcsec,
                shadow_length_if_gnomom_chi: shadow_length,
                shadow_error_cun,
                solstice_time_error_seconds: solstice_error,
                refraction_correction_arcsec: refraction_arcsec,
                technology_gap_factor: gap_factor,
                reference: instrument.reference.clone(),
            });
        }

        results
    }

    pub fn calculate_technology_gap(baseline_error: f64, target_error: f64) -> f64 {
        if target_error <= 0.0 {
            f64::INFINITY
        } else {
            baseline_error / target_error
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_era_comparison_normal() {
        let req = MeridianComparisonRequest {
            sun_altitude: 26.0,
            temperature: 5.0,
            pressure: 1013.25,
        };
        let results = EraComparator::compare(&req);
        assert_eq!(results.len(), 3);
        assert_eq!(results[0].era, "1276");
        assert_eq!(results[1].era, "1900");
        assert_eq!(results[2].era, "2000");
    }

    #[test]
    fn test_era_comparison_technology_progress() {
        let req = MeridianComparisonRequest {
            sun_altitude: 26.0,
            temperature: 5.0,
            pressure: 1013.25,
        };
        let results = EraComparator::compare(&req);
        let err_yuan = results[0].altitude_error_arcsec;
        let err_1900 = results[1].altitude_error_arcsec;
        let err_2000 = results[2].altitude_error_arcsec;
        assert!(err_yuan > err_1900 && err_1900 > err_2000);
        assert!(results[2].technology_gap_factor > 500.0);
    }

    #[test]
    fn test_calculate_technology_gap() {
        let gap = EraComparator::calculate_technology_gap(60.0, 0.03);
        assert!(gap > 1000.0);
    }

    #[test]
    fn test_calculate_technology_gap_zero_error() {
        let gap = EraComparator::calculate_technology_gap(60.0, 0.0);
        assert!(gap.is_infinite());
    }
}
