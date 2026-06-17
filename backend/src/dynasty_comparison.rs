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

    #[test]
    fn test_dynasty_comparison() {
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
    }

    #[test]
    fn test_pinhole_simulation() {
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
    }

    #[test]
    fn test_virtual_experience_daytime() {
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
        assert!(result.theoretical_shadow_chi > 0.0);
        assert_eq!(result.dynasty_hint, "元代");
    }
}
