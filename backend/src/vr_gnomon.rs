use crate::constants::*;
use crate::dynasty_models::*;
use crate::optics::OpticalSimulator;

pub struct VrGnomon;

impl VrGnomon {
    pub fn simulate(request: &VirtualExperienceRequest) -> VirtualExperienceResult {
        let sim = OpticalSimulator::new(
            request.latitude,
            DEFAULT_STATION_LON,
            DEFAULT_STATION_ALT,
        );

        let time_accel = request.time_acceleration.unwrap_or(1);
        let hour_step = (time_accel as f64) * 0.05;

        let year = 2024;
        let day_of_year = Self::month_day_to_doy(request.month, request.day, year);
        let gamma = 2.0 * std::f64::consts::PI * (day_of_year - 1) / 365.0;
        let declination = 23.45 * (gamma + 0.0733 - 0.0068).sin();

        let b = 2.0 * std::f64::consts::PI * (day_of_year as f64 - 81.0) / 365.0;
        let eot = 9.87 * (2.0 * b).sin() - 7.53 * b.cos() - 1.5 * b.sin();

        let lat_rad = request.latitude * DEG_TO_RAD;
        let decl_rad = declination * DEG_TO_RAD;
        let lstm = 15.0 * (DEFAULT_STATION_LON / 15.0).round();
        let tc = 4.0 * (DEFAULT_STATION_LON - lstm) + eot;
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

        let mut next_hour = request.hour + hour_step;
        if next_hour >= 24.0 {
            next_hour -= 24.0;
        }

        let local_solar_time = if lst >= 0.0 { lst % 24.0 } else { lst + 24.0 };

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
            time_acceleration_applied: time_accel,
            next_frame_hour: next_hour,
            local_solar_time_hour: local_solar_time,
        }
    }

    pub fn simulate_time_series(request: &VirtualExperienceRequest) -> VirtualTimeSeriesResponse {
        let time_accel = request.time_acceleration.unwrap_or(60);
        let (dynasty_hint, historical_note) = Self::identify_dynasty(request.gauge_height_chi);

        let step_minutes = match time_accel {
            1..=5 => 1.0,
            6..=30 => 5.0,
            31..=120 => 10.0,
            121..=600 => 15.0,
            _ => 30.0,
        };
        let step_hours = step_minutes / 60.0;

        let mut points = Vec::new();
        let mut sunrise = 99.0;
        let mut sunset = -1.0;
        let mut noon_alt = -90.0;

        let mut t = 0.0_f64;
        while t < 24.0 {
            let hour = request.hour + t;
            let wrapped_hour = if hour >= 24.0 { hour - 24.0 } else { hour };

            let sub_req = VirtualExperienceRequest {
                gauge_height_chi: request.gauge_height_chi,
                latitude: request.latitude,
                month: request.month,
                day: request.day,
                hour: wrapped_hour,
                temperature: request.temperature,
                pressure: request.pressure,
                humidity: request.humidity,
                time_acceleration: Some(1),
            };
            let point_result = Self::simulate(&sub_req);

            if point_result.is_daytime {
                if wrapped_hour < sunrise { sunrise = wrapped_hour; }
                if wrapped_hour > sunset { sunset = wrapped_hour; }
                if point_result.sun_altitude > noon_alt { noon_alt = point_result.sun_altitude; }
            }

            points.push(VirtualTimeSeriesPoint {
                hour: wrapped_hour,
                sun_altitude: point_result.sun_altitude,
                shadow_chi: point_result.refracted_shadow_chi,
                is_daytime: point_result.is_daytime,
            });

            t += step_hours;
        }

        points.sort_by(|a, b| a.hour.partial_cmp(&b.hour).unwrap());

        let daylight = if sunset >= sunrise && sunset >= 0.0 && sunrise <= 24.0 {
            sunset - sunrise
        } else {
            0.0
        };

        VirtualTimeSeriesResponse {
            points,
            sunrise_hour: if sunrise < 24.0 { sunrise } else { 6.0 },
            sunset_hour: if sunset >= 0.0 { sunset } else { 18.0 },
            noon_altitude: if noon_alt > 0.0 { noon_alt } else { 30.0 },
            total_daylight_hours: daylight,
            time_acceleration: time_accel,
            dynasty_hint,
            historical_note,
        }
    }

    pub fn identify_dynasty(gauge_height_chi: f64) -> (String, String) {
        if gauge_height_chi <= 10.0 {
            ("周代/汉代".to_string(),
             "周汉时期表高八尺，为历代基本制度。《周礼·考工记》载'土圭尺有五寸，以至日景'，以洛阳为地中。洛阳金村出土战国铜尺实测23.1cm，八尺合1.848米".to_string())
        } else if gauge_height_chi <= 20.0 {
            ("南北朝/唐代".to_string(),
             "南朝何承天制新历，唐代一行组织全国大地测量（开元十二年），使用八尺圭表测量北极高度。南宫说在河南实测子午线一度长351里80步".to_string())
        } else if gauge_height_chi <= 30.0 {
            ("宋代".to_string(),
             "宋代沈括《景表议》改进测影方法，指出蒙气差对测影的影响，提出'景符'概念的雏形。宋代圭表仍以八尺为主，精度较前代提升".to_string())
        } else {
            ("元代".to_string(),
             "郭守敬至元十三年(1276)创四丈高表，登封观星台实测台面至铜梁高9.46米(40×0.2365m)。配'景符'针孔成像，读数精度达0.1-0.2分，为古代圭表巅峰，《授时历》精度领先世界300年".to_string())
        }
    }

    pub fn month_day_to_doy(month: u32, day: u32, year: i32) -> f64 {
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
    fn test_vr_gnomon_simulate_winter_solstice() {
        let req = VirtualExperienceRequest {
            gauge_height_chi: 40.0,
            latitude: 34.49,
            month: 12,
            day: 22,
            hour: 12.0,
            temperature: 0.0,
            pressure: 1013.0,
            humidity: 40.0,
            time_acceleration: Some(1),
        };
        let result = VrGnomon::simulate(&req);
        assert!(result.is_daytime);
        assert_eq!(result.dynasty_hint, "元代");
        assert!(result.historical_note.contains("郭守敬"));
    }

    #[test]
    fn test_vr_gnomon_simulate_time_series() {
        let req = VirtualExperienceRequest {
            gauge_height_chi: 8.0,
            latitude: 34.49,
            month: 6,
            day: 22,
            hour: 12.0,
            temperature: 30.0,
            pressure: 1005.0,
            humidity: 60.0,
            time_acceleration: Some(60),
        };
        let result = VrGnomon::simulate_time_series(&req);
        assert!(result.points.len() > 0);
        assert!(result.sunrise_hour < 12.0);
        assert!(result.sunset_hour > 12.0);
        assert!(result.total_daylight_hours > 10.0);
    }

    #[test]
    fn test_identify_dynasty_boundaries() {
        assert_eq!(VrGnomon::identify_dynasty(8.0).0, "周代/汉代");
        assert_eq!(VrGnomon::identify_dynasty(15.0).0, "南北朝/唐代");
        assert_eq!(VrGnomon::identify_dynasty(25.0).0, "宋代");
        assert_eq!(VrGnomon::identify_dynasty(40.0).0, "元代");
    }

    #[test]
    fn test_month_day_to_doy_leap_year() {
        assert_eq!(VrGnomon::month_day_to_doy(3, 1, 2024), 61.0);
        assert_eq!(VrGnomon::month_day_to_doy(3, 1, 2023), 60.0);
    }

    #[test]
    fn test_vr_gnomon_southern_hemisphere() {
        let req = VirtualExperienceRequest {
            gauge_height_chi: 8.0,
            latitude: -34.49,
            month: 6,
            day: 22,
            hour: 12.0,
            temperature: 10.0,
            pressure: 1013.0,
            humidity: 50.0,
            time_acceleration: Some(1),
        };
        let result = VrGnomon::simulate(&req);
        assert!(result.is_daytime);
    }
}
