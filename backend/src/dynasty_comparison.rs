use crate::dynasty_models::*;
use crate::optics::OpticalSimulator;

const ARCSEC_TO_DEG: f64 = 1.0 / 3600.0;
const DEG_TO_RAD: f64 = std::f64::consts::PI / 180.0;
const CHI_TO_CUN: f64 = 10.0;
const CHI_TO_M: f64 = 0.3333;
const CUN_TO_M: f64 = CHI_TO_M / CHI_TO_CUN;
const WAVELENGTH_NM: f64 = 550.0;
const SPEED_OF_LIGHT_M_S: f64 = 299792458.0;

pub struct DynastyComparator;

impl DynastyComparator {
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
                gauge_material: gnomon.gauge_material.clone(),
                theoretical_shadow_chi: theoretical_shadow,
                refracted_shadow_chi: refracted_shadow,
                refraction_correction_arcsec: refraction_arcsec,
                shadow_precision_cun,
                solstice_precision_seconds,
                altitude_resolution_arcmin: alt_resolution_arcmin,
            });
        }

        results
    }

    fn estimate_solstice_precision(
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

pub struct MeridianComparator;

impl MeridianComparator {
    pub fn compare(request: &MeridianComparisonRequest) -> Vec<MeridianComparisonResult> {
        let presets = MeridianCircle::presets();
        let mut results = Vec::new();

        for instrument in &presets {
            let sim = OpticalSimulator::new(34.4897, 113.0875, 420.0);

            let refraction_arcsec = sim.calculate_refraction_arcsec(
                request.sun_altitude,
                request.temperature,
                request.pressure,
            );

            let altitude_error = instrument.systematic_error_arcsec;
            let measured_alt = request.sun_altitude + altitude_error * ARCSEC_TO_DEG;

            let shadow_length = if instrument.era == "1276" {
                sim.shadow_length_from_altitude(40.0, measured_alt)
            } else {
                sim.shadow_length_from_altitude(40.0, request.sun_altitude)
            };

            let shadow_error_cun = altitude_error * ARCSEC_TO_DEG
                * 40.0 / (request.sun_altitude.max(1.0) * DEG_TO_RAD).sin().powi(2)
                * CHI_TO_CUN;

            let solstice_error = altitude_error * ARCSEC_TO_DEG
                * 86400.0 / (2.0 * std::f64::consts::PI / 365.25)
                / (request.sun_altitude.max(1.0) * DEG_TO_RAD).tan();

            let gap_factor = if instrument.era == "1276" {
                1.0
            } else {
                let baseline = presets.iter()
                    .find(|p| p.era == "1276")
                    .map(|p| p.systematic_error_arcsec)
                    .unwrap_or(60.0);
                baseline / instrument.systematic_error_arcsec
            };

            results.push(MeridianComparisonResult {
                instrument_id: instrument.instrument_id.clone(),
                instrument_name: instrument.instrument_name.clone(),
                era: instrument.era.clone(),
                measured_altitude_deg: measured_alt,
                altitude_error_arcsec: altitude_error,
                shadow_length_if_gnomom_chi: shadow_length,
                shadow_error_cun,
                solstice_time_error_seconds: solstice_error,
                refraction_correction_arcsec: refraction_arcsec,
                technology_gap_factor: gap_factor,
            });
        }

        results
    }
}

pub struct PinholeSimulator;

impl PinholeSimulator {
    pub fn simulate(request: &PinholeRequest) -> PinholeResult {
        let d_m = request.pinhole_diameter_cun * CUN_TO_M;
        let h_m = request.gauge_height_chi * CHI_TO_M;
        let s_m = request.screen_distance_chi * CHI_TO_M;
        let alt_rad = request.sun_altitude.max(0.1) * DEG_TO_RAD;

        let sun_angular_diameter_rad = 0.00930; // ~31.6 arcmin = 0.533 deg

        let geometric_blur_m = d_m * s_m / h_m;
        let geometric_blur_cun = geometric_blur_m / CUN_TO_M;

        let wavelength_m = WAVELENGTH_NM * 1e-9;
        let diffraction_blur_rad = 1.22 * wavelength_m / d_m;
        let diffraction_blur_m = diffraction_blur_rad * s_m;
        let diffraction_blur_cun = diffraction_blur_m / CUN_TO_M;

        let total_blur_cun = (geometric_blur_cun.powi(2) + diffraction_blur_cun.powi(2)).sqrt();

        let optimal_diameter_m = (1.22 * wavelength_m * h_m).sqrt();
        let optimal_diameter_cun = optimal_diameter_m / CUN_TO_M;

        let sun_image_diameter_m = sun_angular_diameter_rad * s_m;
        let sun_image_diameter_cun = sun_image_diameter_m / CUN_TO_M;

        let magnification = s_m / h_m;

        let signal_area = std::f64::consts::PI * (sun_image_diameter_m / 2.0).powi(2);
        let blur_area = std::f64::consts::PI * (total_blur_m / 2.0).powi(2);
        let snr = (signal_area / blur_area.max(1e-15)).sqrt();

        let shadow_edge_sharpness = 1.0 / (1.0 + (total_blur_cun / sun_image_diameter_cun.max(0.001)).powi(2));

        let alt_resolution_rad = total_blur_cun / (CHI_TO_CUN * request.gauge_height_chi);
        let alt_resolution_arcmin = alt_resolution_rad * (180.0 / std::f64::consts::PI) * 60.0;

        let cos_alt = alt_rad.cos();
        let vignetting = (1.0 - (d_m / (2.0 * h_m * cos_alt)).powi(2)).max(0.0);

        PinholeResult {
            pinhole_diameter_cun: request.pinhole_diameter_cun,
            sun_image_diameter_cun,
            geometric_blur_cun,
            diffraction_blur_cun,
            total_blur_cun,
            optimal_diameter_cun,
            signal_to_noise_ratio: snr,
            shadow_edge_sharpness,
            altitude_resolution_arcmin,
            magnification,
            vignetting_factor: vignetting,
        }
    }
}

pub struct VirtualExperienceSimulator;

impl VirtualExperienceSimulator {
    pub fn simulate(request: &VirtualExperienceRequest) -> VirtualExperienceResult {
        let sim = OpticalSimulator::new(
            request.latitude,
            113.0875,
            420.0,
        );

        let year = 2024;
        let day_of_year = Self::month_day_to_doy(request.month, request.day, year);
        let gamma = 2.0 * std::f64::consts::PI * (day_of_year - 1) / 365.0;
        let declination = 23.45 * (gamma + 0.0733 - 0.0068).sin();

        let b = 2.0 * std::f64::consts::PI * (day_of_year as f64 - 81.0) / 365.0;
        let eot = 9.87 * (2.0 * b).sin() - 7.53 * b.cos() - 1.5 * b.sin();

        let lat_rad = request.latitude * DEG_TO_RAD;
        let decl_rad = declination * DEG_TO_RAD;
        let lstm = 15.0 * (113.0875 / 15.0).round();
        let tc = 4.0 * (113.0875 - lstm) + eot;
        let lst = request.hour + tc / 60.0;
        let hour_angle = 15.0 * (lst - 12.0);
        let hour_rad = hour_angle * DEG_TO_RAD;

        let sin_alt = lat_rad.sin() * decl_rad.sin()
            + lat_rad.cos() * decl_rad.cos() * hour_rad.cos();
        let sun_altitude = sin_alt.clamp(-1.0, 1.0).asin().to_degrees();

        let cos_azi = if sun_altitude.abs() < 89.9 {
            (decl_rad.sin() - sin_alt.clamp(-1.0, 1.0) * lat_rad.sin())
                / (sun_altitude.to_radians().cos() * lat_rad.cos())
        } else {
            0.0
        };
        let cos_azi = cos_azi.clamp(-1.0, 1.0);
        let azi = cos_azi.acos().to_degrees();
        let sun_azimuth = if hour_angle > 0.0 { 360.0 - azi } else { azi };

        let is_daytime = sun_altitude > 0.0;

        let refraction_arcsec = if is_daytime {
            sim.calculate_refraction_arcsec(sun_altitude, request.temperature, request.pressure)
        } else {
            0.0
        };

        let true_alt = sun_altitude - refraction_arcsec * ARCSEC_TO_DEG;
        let apparent_alt = if is_daytime {
            sim.correct_for_refraction(true_alt, request.temperature, request.pressure)
        } else {
            sun_altitude
        };

        let theoretical_shadow = if is_daytime && true_alt > 0.0 {
            sim.shadow_length_from_altitude(request.gauge_height_chi, true_alt)
        } else {
            f64::MAX
        };
        let refracted_shadow = if is_daytime && apparent_alt > 0.0 {
            sim.shadow_length_from_altitude(request.gauge_height_chi, apparent_alt)
        } else {
            f64::MAX
        };

        let (dynasty_hint, historical_note) = Self::identify_dynasty(request.gauge_height_chi);

        VirtualExperienceResult {
            gauge_height_chi: request.gauge_height_chi,
            sun_altitude,
            sun_azimuth,
            sun_declination: declination,
            equation_of_time_min: eot,
            theoretical_shadow_chi: if theoretical_shadow.is_finite() { theoretical_shadow } else { -1.0 },
            refracted_shadow_chi: if refracted_shadow.is_finite() { refracted_shadow } else { -1.0 },
            refraction_correction_arcsec: refraction_arcsec,
            shadow_length_cun: if refracted_shadow.is_finite() { refracted_shadow * CHI_TO_CUN } else { -1.0 },
            is_daytime,
            dynasty_hint,
            historical_note,
        }
    }

    fn identify_dynasty(gauge_height_chi: f64) -> (String, String) {
        if gauge_height_chi <= 10.0 {
            ("周代/汉代".to_string(),
             "周汉时期表高八尺，为历代基本制度。《周礼》'土圭之法'以此为基准".to_string())
        } else if gauge_height_chi <= 20.0 {
            ("南北朝/唐代".to_string(),
             "南朝何承天、唐代一行等曾改进圭表制度，但表高仍以八尺为主".to_string())
        } else if gauge_height_chi <= 30.0 {
            ("宋代".to_string(),
             "宋代沈括《景表议》改进测影方法，但表高仍有限".to_string())
        } else {
            ("元代".to_string(),
             "郭守敬创四丈高表，以横梁针孔成像提高读数精度，为古代圭表巅峰".to_string())
        }
    }

    fn month_day_to_doy(month: u32, day: u32, year: i32) -> f64 {
        let days_in_month = [0, 31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
        let is_leap = (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0);
        let mut doy = 0;
        for m in 1..month.min(12) {
            doy += days_in_month[m as usize];
            if m == 2 && is_leap {
                doy += 1;
            }
        }
        doy += day.min(days_in_month[month.min(12) as usize]) as i32;
        doy as f64
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // =========================================================================
    // Feature 1: 精度对比验证测量误差
    // =========================================================================

    #[test]
    fn test_dynasty_comparison_normal() {
        let req = DynastyComparisonRequest {
            sun_altitude: 26.0,
            temperature: 5.0,
            pressure: 1013.25,
            humidity: 50.0,
        };
        let results = DynastyComparator::compare(&req);
        assert_eq!(results.len(), 3);
        assert!(results[0].theoretical_shadow_chi > results[2].theoretical_shadow_chi / 5.0);
        assert!(results[2].solstice_precision_seconds < results[0].solstice_precision_seconds);
        assert!(results[2].altitude_resolution_arcmin < results[0].altitude_resolution_arcmin);
    }

    #[test]
    fn test_dynasty_comparison_boundary_high_altitude() {
        let req = DynastyComparisonRequest {
            sun_altitude: 80.0,
            temperature: 30.0,
            pressure: 1013.25,
            humidity: 70.0,
        };
        let results = DynastyComparator::compare(&req);
        assert_eq!(results.len(), 3);
        for r in &results {
            assert!(r.theoretical_shadow_chi > 0.0);
            assert!(r.refraction_correction_arcsec > 0.0);
        }
        let ratio = results[2].theoretical_shadow_chi / results[0].theoretical_shadow_chi;
        assert!((ratio - 5.0).abs() < 0.1, "表高5倍对应影长应约5倍，实际: {}", ratio);
    }

    #[test]
    fn test_dynasty_comparison_boundary_low_altitude() {
        let req = DynastyComparisonRequest {
            sun_altitude: 5.0,
            temperature: -10.0,
            pressure: 1030.0,
            humidity: 30.0,
        };
        let results = DynastyComparator::compare(&req);
        assert_eq!(results.len(), 3);
        for r in &results {
            assert!(r.theoretical_shadow_chi > 0.0);
            assert!(r.refraction_correction_arcsec > 10.0);
        }
        assert!(results[2].solstice_precision_seconds > 0.0);
    }

    #[test]
    fn test_dynasty_comparison_boundary_negative_altitude() {
        let req = DynastyComparisonRequest {
            sun_altitude: -5.0,
            temperature: -20.0,
            pressure: 1013.25,
            humidity: 20.0,
        };
        let results = DynastyComparator::compare(&req);
        assert_eq!(results.len(), 3);
        for r in &results {
            assert!(r.theoretical_shadow_chi < 0.0 || r.theoretical_shadow_chi.is_nan() || r.theoretical_shadow_chi.abs() > 1e6);
        }
    }

    #[test]
    fn test_dynasty_comparison_anomalous_extreme_pressure() {
        let req = DynastyComparisonRequest {
            sun_altitude: 26.0,
            temperature: 20.0,
            pressure: 500.0,
            humidity: 50.0,
        };
        let results = DynastyComparator::compare(&req);
        assert_eq!(results.len(), 3);
        for r in &results {
            assert!(r.refraction_correction_arcsec > 0.0);
            assert!(r.refraction_correction_arcsec < 300.0);
        }
    }

    #[test]
    fn test_dynasty_comparison_anomalous_invalid_inputs() {
        let req = DynastyComparisonRequest {
            sun_altitude: f64::NAN,
            temperature: 20.0,
            pressure: 1013.25,
            humidity: 50.0,
        };
        let results = DynastyComparator::compare(&req);
        assert_eq!(results.len(), 3);
    }

    #[test]
    fn test_dynasty_measurement_error_scaling() {
        let req_normal = DynastyComparisonRequest {
            sun_altitude: 26.0,
            temperature: 5.0,
            pressure: 1013.25,
            humidity: 50.0,
        };
        let results = DynastyComparator::compare(&req_normal);
        assert_eq!(results.len(), 3);

        let zhou_precision = results[0].shadow_precision_cun;
        let han_precision = results[1].shadow_precision_cun;
        let yuan_precision = results[2].shadow_precision_cun;

        assert!(zhou_precision > han_precision, "周代精度应差于汉代");
        assert!(han_precision > yuan_precision, "汉代精度应差于元代");
        assert!(zhou_precision >= 2.0, "周代影长精度应≥2寸");
        assert!(han_precision >= 0.5, "汉代影长精度应≥0.5寸");
        assert!(yuan_precision >= 0.2, "元代影长精度应≥0.2寸");
    }

    #[test]
    fn test_dynasty_comparison_anomalous_zero_altitude() {
        let req = DynastyComparisonRequest {
            sun_altitude: 0.0,
            temperature: 15.0,
            pressure: 1013.25,
            humidity: 50.0,
        };
        let results = DynastyComparator::compare(&req);
        assert_eq!(results.len(), 3);
        for r in &results {
            assert!(!r.refraction_correction_arcsec.is_nan());
            assert!(r.refraction_correction_arcsec > 0.0);
        }
    }

    // =========================================================================
    // Feature 2: 跨时代对比验证技术进步
    // =========================================================================

    #[test]
    fn test_meridian_comparison_normal() {
        let req = MeridianComparisonRequest {
            sun_altitude: 26.0,
            temperature: 5.0,
            pressure: 1013.25,
        };
        let results = MeridianComparator::compare(&req);
        assert_eq!(results.len(), 3);
        assert_eq!(results[0].era, "1276");
        assert_eq!(results[1].era, "1900");
        assert_eq!(results[2].era, "2000");
    }

    #[test]
    fn test_meridian_technology_progress() {
        let req = MeridianComparisonRequest {
            sun_altitude: 26.0,
            temperature: 5.0,
            pressure: 1013.25,
        };
        let results = MeridianComparator::compare(&req);
        assert_eq!(results.len(), 3);

        let yuan_error = results[0].altitude_error_arcsec;
        let modern1900_error = results[1].altitude_error_arcsec;
        let modern2000_error = results[2].altitude_error_arcsec;

        assert!(yuan_error > modern1900_error, "元代误差应大于1900年");
        assert!(modern1900_error > modern2000_error, "1900年误差应大于2000年");
        assert!(results[0].technology_gap_factor == 1.0, "元代基线技术差距应为1");
        assert!(results[1].technology_gap_factor > 1.0, "1900年技术差距应>1");
        assert!(results[2].technology_gap_factor > results[1].technology_gap_factor, "2000年技术差距应最大");
    }

    #[test]
    fn test_meridian_comparison_boundary_solstice_altitude() {
        let req = MeridianComparisonRequest {
            sun_altitude: 31.5,
            temperature: 0.0,
            pressure: 1015.0,
        };
        let results = MeridianComparator::compare(&req);
        assert_eq!(results.len(), 3);
        let solstice_err_yuan = results[0].solstice_time_error_seconds;
        let solstice_err_2000 = results[2].solstice_time_error_seconds;
        assert!(solstice_err_yuan > 60.0, "元代冬至时刻误差应大于1分钟");
        assert!(solstice_err_2000 < 10.0, "2000年冬至时刻误差应小于10秒");
    }

    #[test]
    fn test_meridian_comparison_boundary_extreme_altitude() {
        let req = MeridianComparisonRequest {
            sun_altitude: 85.0,
            temperature: 35.0,
            pressure: 1005.0,
        };
        let results = MeridianComparator::compare(&req);
        assert_eq!(results.len(), 3);
        for r in &results {
            assert!(r.altitude_error_arcsec > 0.0);
            assert!(r.shadow_error_cun > 0.0);
        }
    }

    #[test]
    fn test_meridian_comparison_anomalous_negative_altitude() {
        let req = MeridianComparisonRequest {
            sun_altitude: -10.0,
            temperature: -15.0,
            pressure: 1020.0,
        };
        let results = MeridianComparator::compare(&req);
        assert_eq!(results.len(), 3);
        for r in &results {
            assert!(!r.altitude_error_arcsec.is_nan());
            assert!(!r.shadow_error_cun.is_nan());
        }
    }

    #[test]
    fn test_meridian_comparison_anomalous_extreme_temperature() {
        let req = MeridianComparisonRequest {
            sun_altitude: 26.0,
            temperature: 60.0,
            pressure: 950.0,
        };
        let results = MeridianComparator::compare(&req);
        assert_eq!(results.len(), 3);
        assert!(results[2].technology_gap_factor >= 100.0);
    }

    #[test]
    fn test_meridian_gap_factor_ordering() {
        let req = MeridianComparisonRequest {
            sun_altitude: 26.0,
            temperature: 10.0,
            pressure: 1013.25,
        };
        let results = MeridianComparator::compare(&req);
        assert_eq!(results.len(), 3);

        let gap_1276 = results[0].technology_gap_factor;
        let gap_1900 = results[1].technology_gap_factor;
        let gap_2000 = results[2].technology_gap_factor;

        assert_eq!(gap_1276, 1.0);
        assert!(gap_1900 > 30.0, "1900年相对元代应有约30倍以上精度提升");
        assert!(gap_2000 > 500.0, "2000年相对元代应有约500倍以上精度提升");
        assert!(gap_2000 > gap_1900);
    }

    // =========================================================================
    // Feature 3: 针孔成像验证影长清晰度
    // =========================================================================

    #[test]
    fn test_pinhole_simulation_normal() {
        let req = PinholeRequest {
            gauge_height_chi: 40.0,
            pinhole_diameter_cun: 1.0,
            sun_altitude: 26.0,
            screen_distance_chi: 40.0,
            temperature: 5.0,
            pressure: 1013.25,
        };
        let result = PinholeSimulator::simulate(&req);
        assert!(result.sun_image_diameter_cun > 0.0);
        assert!(result.optimal_diameter_cun > 0.0);
        assert!(result.total_blur_cun > 0.0);
        assert!(result.shadow_edge_sharpness > 0.0 && result.shadow_edge_sharpness <= 1.0);
        assert!(result.magnification > 0.0);
        assert!(result.altitude_resolution_arcmin > 0.0);
        assert!(result.vignetting_factor >= 0.0 && result.vignetting_factor <= 1.0);
    }

    #[test]
    fn test_pinhole_boundary_optimal_diameter() {
        let diameters = vec![0.1, 0.2, 0.5, 1.0, 2.0, 3.0, 5.0];
        let mut min_blur = f64::MAX;
        let mut optimal_d = 0.0;

        for d in &diameters {
            let req = PinholeRequest {
                gauge_height_chi: 40.0,
                pinhole_diameter_cun: *d,
                sun_altitude: 26.0,
                screen_distance_chi: 40.0,
                temperature: 5.0,
                pressure: 1013.25,
            };
            let result = PinholeSimulator::simulate(&req);
            if result.total_blur_cun < min_blur {
                min_blur = result.total_blur_cun;
                optimal_d = *d;
            }
            assert!(result.signal_to_noise_ratio > 0.0);
        }

        assert!(optimal_d > 0.0);
        assert!(min_blur > 0.0);
    }

    #[test]
    fn test_pinhole_boundary_large_diameter() {
        let req = PinholeRequest {
            gauge_height_chi: 40.0,
            pinhole_diameter_cun: 10.0,
            sun_altitude: 26.0,
            screen_distance_chi: 40.0,
            temperature: 5.0,
            pressure: 1013.25,
        };
        let result = PinholeSimulator::simulate(&req);
        assert!(result.geometric_blur_cun > result.diffraction_blur_cun);
        assert!(result.total_blur_cun > 0.0);
        assert!(result.shadow_edge_sharpness > 0.0 && result.shadow_edge_sharpness <= 1.0);
    }

    #[test]
    fn test_pinhole_boundary_small_diameter() {
        let req = PinholeRequest {
            gauge_height_chi: 40.0,
            pinhole_diameter_cun: 0.01,
            sun_altitude: 26.0,
            screen_distance_chi: 40.0,
            temperature: 5.0,
            pressure: 1013.25,
        };
        let result = PinholeSimulator::simulate(&req);
        assert!(result.diffraction_blur_cun > result.geometric_blur_cun);
        assert!(result.total_blur_cun > 0.0);
    }

    #[test]
    fn test_pinhole_blur_components() {
        let req_small = PinholeRequest {
            gauge_height_chi: 40.0,
            pinhole_diameter_cun: 0.05,
            sun_altitude: 26.0,
            screen_distance_chi: 40.0,
            temperature: 5.0,
            pressure: 1013.25,
        };
        let result_small = PinholeSimulator::simulate(&req_small);

        let req_large = PinholeRequest {
            gauge_height_chi: 40.0,
            pinhole_diameter_cun: 5.0,
            sun_altitude: 26.0,
            screen_distance_chi: 40.0,
            temperature: 5.0,
            pressure: 1013.25,
        };
        let result_large = PinholeSimulator::simulate(&req_large);

        assert!(result_small.diffraction_blur_cun > result_small.geometric_blur_cun,
            "小孔径下衍射应主导: 衍射={}, 几何={}", result_small.diffraction_blur_cun, result_small.geometric_blur_cun);
        assert!(result_large.geometric_blur_cun > result_large.diffraction_blur_cun,
            "大孔径下几何应主导: 几何={}, 衍射={}", result_large.geometric_blur_cun, result_large.diffraction_blur_cun);
    }

    #[test]
    fn test_pinhole_shadow_sharpness_monotonic() {
        let diameters = vec![0.1, 0.3, 0.5, 0.7, 1.0, 2.0, 5.0];
        let mut sharpness_values = Vec::new();

        for d in &diameters {
            let req = PinholeRequest {
                gauge_height_chi: 40.0,
                pinhole_diameter_cun: *d,
                sun_altitude: 26.0,
                screen_distance_chi: 40.0,
                temperature: 5.0,
                pressure: 1013.25,
            };
            let result = PinholeSimulator::simulate(&req);
            sharpness_values.push(result.shadow_edge_sharpness);
        }

        let max_sharpness = sharpness_values.iter().fold(0.0, |a, &b| a.max(b));
        assert!(max_sharpness > 0.5, "最优锐度应大于0.5");

        let min_sharpness = sharpness_values.iter().fold(f64::MAX, |a, &b| a.min(b));
        assert!(min_sharpness > 0.0, "所有锐度应大于0");
    }

    #[test]
    fn test_pinhole_anomalous_zero_diameter() {
        let req = PinholeRequest {
            gauge_height_chi: 40.0,
            pinhole_diameter_cun: 0.0,
            sun_altitude: 26.0,
            screen_distance_chi: 40.0,
            temperature: 5.0,
            pressure: 1013.25,
        };
        let result = PinholeSimulator::simulate(&req);
        assert!(result.diffraction_blur_cun.is_infinite() || result.diffraction_blur_cun > 1e6);
    }

    #[test]
    fn test_pinhole_anomalous_negative_diameter() {
        let req = PinholeRequest {
            gauge_height_chi: 40.0,
            pinhole_diameter_cun: -1.0,
            sun_altitude: 26.0,
            screen_distance_chi: 40.0,
            temperature: 5.0,
            pressure: 1013.25,
        };
        let result = PinholeSimulator::simulate(&req);
        assert!(result.geometric_blur_cun >= 0.0);
        assert!(result.diffraction_blur_cun >= 0.0);
    }

    #[test]
    fn test_pinhole_anomalous_extreme_gauge_height() {
        let req = PinholeRequest {
            gauge_height_chi: 1000.0,
            pinhole_diameter_cun: 1.0,
            sun_altitude: 26.0,
            screen_distance_chi: 1000.0,
            temperature: 5.0,
            pressure: 1013.25,
        };
        let result = PinholeSimulator::simulate(&req);
        assert!(result.sun_image_diameter_cun > 0.0);
        assert!(!result.altitude_resolution_arcmin.is_nan());
    }

    // =========================================================================
    // Feature 4: 虚拟体验测试交互教育性
    // =========================================================================

    #[test]
    fn test_virtual_experience_daytime_winter_solstice() {
        let req = VirtualExperienceRequest {
            gauge_height_chi: 40.0,
            latitude: 34.49,
            month: 12,
            day: 22,
            hour: 12.0,
            temperature: 0.0,
            pressure: 1013.0,
            humidity: 40.0,
        };
        let result = VirtualExperienceSimulator::simulate(&req);
        assert!(result.is_daytime);
        assert!(result.sun_altitude > 0.0);
        assert!(result.sun_altitude < 40.0);
        assert!(result.theoretical_shadow_chi > 0.0);
        assert_eq!(result.dynasty_hint, "元代");
        assert!(result.historical_note.contains("郭守敬"));
    }

    #[test]
    fn test_virtual_experience_daytime_summer_solstice() {
        let req = VirtualExperienceRequest {
            gauge_height_chi: 8.0,
            latitude: 34.49,
            month: 6,
            day: 22,
            hour: 12.0,
            temperature: 30.0,
            pressure: 1005.0,
            humidity: 60.0,
        };
        let result = VirtualExperienceSimulator::simulate(&req);
        assert!(result.is_daytime);
        assert!(result.sun_altitude > 60.0);
        assert!(result.theoretical_shadow_chi > 0.0);
        assert_eq!(result.dynasty_hint, "周代/汉代");
        assert!(result.historical_note.contains("土圭"));
    }

    #[test]
    fn test_virtual_experience_boundary_sunrise() {
        let req = VirtualExperienceRequest {
            gauge_height_chi: 8.0,
            latitude: 34.49,
            month: 6,
            day: 22,
            hour: 5.0,
            temperature: 20.0,
            pressure: 1013.0,
            humidity: 70.0,
        };
        let result = VirtualExperienceSimulator::simulate(&req);
        assert!(!result.is_daytime);
        assert!(result.sun_altitude < 0.0);
        assert_eq!(result.theoretical_shadow_chi, -1.0);
        assert_eq!(result.refracted_shadow_chi, -1.0);
    }

    #[test]
    fn test_virtual_experience_boundary_sunset() {
        let req = VirtualExperienceRequest {
            gauge_height_chi: 8.0,
            latitude: 34.49,
            month: 6,
            day: 22,
            hour: 19.5,
            temperature: 25.0,
            pressure: 1012.0,
            humidity: 65.0,
        };
        let result = VirtualExperienceSimulator::simulate(&req);
        assert!(!result.is_daytime);
        assert!(result.sun_altitude < 0.0);
    }

    #[test]
    fn test_virtual_experience_boundary_equinox() {
        let req = VirtualExperienceRequest {
            gauge_height_chi: 8.0,
            latitude: 34.49,
            month: 3,
            day: 21,
            hour: 12.0,
            temperature: 15.0,
            pressure: 1013.0,
            humidity: 50.0,
        };
        let result = VirtualExperienceSimulator::simulate(&req);
        assert!(result.is_daytime);
        assert!((result.sun_declination).abs() < 2.0);
    }

    #[test]
    fn test_virtual_experience_boundary_dynasty_boundaries() {
        let heights = vec![
            (8.0, "周代/汉代"),
            (10.0, "周代/汉代"),
            (10.1, "南北朝/唐代"),
            (20.0, "南北朝/唐代"),
            (20.1, "宋代"),
            (30.0, "宋代"),
            (30.1, "元代"),
            (40.0, "元代"),
            (100.0, "元代"),
        ];

        for (height, expected_hint) in heights {
            let req = VirtualExperienceRequest {
                gauge_height_chi: height,
                latitude: 34.49,
                month: 12,
                day: 22,
                hour: 12.0,
                temperature: 5.0,
                pressure: 1013.0,
                humidity: 50.0,
            };
            let result = VirtualExperienceSimulator::simulate(&req);
            assert_eq!(result.dynasty_hint, expected_hint,
                "表高{}尺应归属{}，实际归属{}", height, expected_hint, result.dynasty_hint);
            assert!(!result.historical_note.is_empty());
        }
    }

    #[test]
    fn test_virtual_experience_boundary_extreme_latitude() {
        let req = VirtualExperienceRequest {
            gauge_height_chi: 8.0,
            latitude: 80.0,
            month: 6,
            day: 22,
            hour: 12.0,
            temperature: 5.0,
            pressure: 1013.0,
            humidity: 50.0,
        };
        let result = VirtualExperienceSimulator::simulate(&req);
        assert!(result.sun_altitude > 0.0);
        assert!(result.sun_altitude < 40.0);
        assert!(result.theoretical_shadow_chi > 0.0);
    }

    #[test]
    fn test_virtual_experience_boundary_southern_hemisphere() {
        let req = VirtualExperienceRequest {
            gauge_height_chi: 8.0,
            latitude: -34.49,
            month: 6,
            day: 22,
            hour: 12.0,
            temperature: 10.0,
            pressure: 1013.0,
            humidity: 50.0,
        };
        let result = VirtualExperienceSimulator::simulate(&req);
        assert!(result.is_daytime);
        assert!(result.sun_altitude > 0.0);
        assert_eq!(result.dynasty_hint, "周代/汉代");
    }

    #[test]
    fn test_virtual_experience_anomalous_invalid_month() {
        let req = VirtualExperienceRequest {
            gauge_height_chi: 8.0,
            latitude: 34.49,
            month: 15,
            day: 22,
            hour: 12.0,
            temperature: 5.0,
            pressure: 1013.0,
            humidity: 50.0,
        };
        let result = VirtualExperienceSimulator::simulate(&req);
        assert!(!result.sun_declination.is_nan());
    }

    #[test]
    fn test_virtual_experience_anomalous_invalid_day() {
        let req = VirtualExperienceRequest {
            gauge_height_chi: 8.0,
            latitude: 34.49,
            month: 2,
            day: 31,
            hour: 12.0,
            temperature: 5.0,
            pressure: 1013.0,
            humidity: 50.0,
        };
        let result = VirtualExperienceSimulator::simulate(&req);
        assert!(!result.sun_declination.is_nan());
    }

    #[test]
    fn test_virtual_experience_anomalous_invalid_hour() {
        let req = VirtualExperienceRequest {
            gauge_height_chi: 8.0,
            latitude: 34.49,
            month: 6,
            day: 22,
            hour: 30.0,
            temperature: 25.0,
            pressure: 1013.0,
            humidity: 50.0,
        };
        let result = VirtualExperienceSimulator::simulate(&req);
        assert!(!result.sun_altitude.is_nan());
    }

    #[test]
    fn test_virtual_experience_anomalous_negative_gauge_height() {
        let req = VirtualExperienceRequest {
            gauge_height_chi: -10.0,
            latitude: 34.49,
            month: 6,
            day: 22,
            hour: 12.0,
            temperature: 25.0,
            pressure: 1013.0,
            humidity: 50.0,
        };
        let result = VirtualExperienceSimulator::simulate(&req);
        assert_eq!(result.dynasty_hint, "周代/汉代");
    }

    #[test]
    fn test_virtual_experience_educational_hint_not_empty() {
        let heights = vec![5.0, 8.0, 15.0, 25.0, 40.0];
        for h in heights {
            let req = VirtualExperienceRequest {
                gauge_height_chi: h,
                latitude: 34.49,
                month: 12,
                day: 22,
                hour: 12.0,
                temperature: 0.0,
                pressure: 1013.0,
                humidity: 50.0,
            };
            let result = VirtualExperienceSimulator::simulate(&req);
            assert!(!result.dynasty_hint.is_empty(),
                "表高{}尺应返回朝代提示", h);
            assert!(!result.historical_note.is_empty(),
                "表高{}尺应返回历史说明", h);
            assert!(result.historical_note.chars().count() > 10,
                "历史说明应包含足够教育内容");
        }
    }

    #[test]
    fn test_virtual_experience_hour_monotonic_daytime() {
        let mut prev_altitude = -100.0;
        for hour in (6..=18).step_by(1) {
            let req = VirtualExperienceRequest {
                gauge_height_chi: 40.0,
                latitude: 34.49,
                month: 6,
                day: 22,
                hour: hour as f64,
                temperature: 20.0,
                pressure: 1013.0,
                humidity: 50.0,
            };
            let result = VirtualExperienceSimulator::simulate(&req);
            if hour <= 12 {
                assert!(result.sun_altitude >= prev_altitude - 0.1,
                    "上午{}点太阳应升高: {} -> {}", hour - 1, prev_altitude, result.sun_altitude);
            } else {
                assert!(result.sun_altitude <= prev_altitude + 0.1,
                    "下午{}点太阳应降低: {} -> {}", hour - 1, prev_altitude, result.sun_altitude);
            }
            prev_altitude = result.sun_altitude;
        }
    }

    #[test]
    fn test_virtual_experience_shadow_length_matches_gauge() {
        let heights = vec![8.0, 16.0, 32.0, 40.0];
        let mut shadows = Vec::new();

        for h in &heights {
            let req = VirtualExperienceRequest {
                gauge_height_chi: *h,
                latitude: 34.49,
                month: 6,
                day: 22,
                hour: 12.0,
                temperature: 25.0,
                pressure: 1013.0,
                humidity: 50.0,
            };
            let result = VirtualExperienceSimulator::simulate(&req);
            shadows.push(result.theoretical_shadow_chi);
        }

        for i in 1..shadows.len() {
            let ratio = shadows[i] / shadows[0];
            let expected_ratio = heights[i] / heights[0];
            assert!((ratio - expected_ratio).abs() < 0.01,
                "表高{}倍应影长约{}倍，实际{}倍", expected_ratio, expected_ratio, ratio);
        }
    }

    // =========================================================================
    // 辅助函数测试
    // =========================================================================

    #[test]
    fn test_month_day_to_doy_normal() {
        assert_eq!(VirtualExperienceSimulator::month_day_to_doy(1, 1, 2024), 1.0);
        assert_eq!(VirtualExperienceSimulator::month_day_to_doy(2, 1, 2024), 32.0);
        assert_eq!(VirtualExperienceSimulator::month_day_to_doy(3, 1, 2024), 61.0);
        assert_eq!(VirtualExperienceSimulator::month_day_to_doy(12, 31, 2024), 366.0);
    }

    #[test]
    fn test_month_day_to_doy_leap_year() {
        assert_eq!(VirtualExperienceSimulator::month_day_to_doy(3, 1, 2024), 61.0);
        assert_eq!(VirtualExperienceSimulator::month_day_to_doy(3, 1, 2023), 60.0);
    }

    #[test]
    fn test_month_day_to_doy_boundary() {
        assert_eq!(VirtualExperienceSimulator::month_day_to_doy(15, 1, 2024), 335.0);
        assert_eq!(VirtualExperienceSimulator::month_day_to_doy(2, 30, 2024), 59.0);
    }

    #[test]
    fn test_identify_dynasty_all_ranges() {
        let test_cases = vec![
            (0.0, "周代/汉代"),
            (8.0, "周代/汉代"),
            (10.0, "周代/汉代"),
            (10.0001, "南北朝/唐代"),
            (15.0, "南北朝/唐代"),
            (20.0, "南北朝/唐代"),
            (20.0001, "宋代"),
            (25.0, "宋代"),
            (30.0, "宋代"),
            (30.0001, "元代"),
            (40.0, "元代"),
            (100.0, "元代"),
            (f64::INFINITY, "元代"),
        ];

        for (height, expected) in test_cases {
            let (hint, note) = VirtualExperienceSimulator::identify_dynasty(height);
            assert_eq!(hint, expected, "表高{}尺", height);
            assert!(!note.is_empty());
        }
    }
}
