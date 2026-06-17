use serde::{Deserialize, Serialize};

const CHI_TO_M: f64 = 0.3333;
const CUN_TO_MM: f64 = 33.33;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DynastyGnomon {
    pub dynasty_id: String,
    pub dynasty_name: String,
    pub period: String,
    pub gauge_height_chi: f64,
    pub chi_to_m_conversion: f64,
    pub gauge_height_m_actual: f64,
    pub gauge_material: String,
    pub gauge_height_error_std_chi: f64,
    pub shadow_reading_error_std_cun: f64,
    pub latitude: f64,
    pub longitude: f64,
    pub altitude: f64,
    pub description: String,
    pub archaeological_source: String,
}

impl DynastyGnomon {
    pub fn presets() -> Vec<Self> {
        vec![
            Self {
                dynasty_id: "zhou_tugu".to_string(),
                dynasty_name: "周代土圭".to_string(),
                period: "公元前11世纪—前256年".to_string(),
                gauge_height_chi: 8.0,
                chi_to_m_conversion: 0.231,
                gauge_height_m_actual: 1.848,
                gauge_material: "土筑/木质".to_string(),
                gauge_height_error_std_chi: 0.1,
                shadow_reading_error_std_cun: 2.0,
                latitude: 34.25,
                longitude: 108.93,
                altitude: 400.0,
                description: "《周礼·考工记》载'土圭尺有五寸，以至日景'，洛阳金村出土战国铜尺实测23.1cm，八尺表高合1.848米".to_string(),
                archaeological_source: "考古依据：河南洛阳金村战国墓出土铜尺，长23.1cm；《周礼·地官·大司徒》".to_string(),
            },
            Self {
                dynasty_id: "han_tongbiao".to_string(),
                dynasty_name: "汉代铜表".to_string(),
                period: "公元前206年—公元220年".to_string(),
                gauge_height_chi: 8.0,
                chi_to_m_conversion: 0.233,
                gauge_height_m_actual: 1.864,
                gauge_material: "青铜铸造".to_string(),
                gauge_height_error_std_chi: 0.02,
                shadow_reading_error_std_cun: 0.5,
                latitude: 34.26,
                longitude: 108.94,
                altitude: 405.0,
                description: "汉承秦制，西汉骨尺23.1-23.2cm，东汉鎏金铜尺23.5cm。汉代铜表刻寸分刻度，读数精度优于周代".to_string(),
                archaeological_source: "考古依据：湖南长沙左家山出土西汉骨尺23.2cm；洛阳出土东汉铜尺23.5cm；《史记·天官书》".to_string(),
            },
            Self {
                dynasty_id: "yuan_sizhang".to_string(),
                dynasty_name: "元代四丈高表".to_string(),
                period: "1276年—1368年".to_string(),
                gauge_height_chi: 40.0,
                chi_to_m_conversion: 0.2365,
                gauge_height_m_actual: 9.46,
                gauge_material: "砖石砌筑+铜横梁".to_string(),
                gauge_height_error_std_chi: 0.01,
                shadow_reading_error_std_cun: 0.2,
                latitude: 34.4897,
                longitude: 113.0875,
                altitude: 420.0,
                description: "郭守敬至元十三年(1276)建登封观星台，台面至横梁实测高9.46米，合40×0.2365m。横梁配针孔成像(景符)，读数精度达0.1-0.2分".to_string(),
                archaeological_source: "考古依据：登封观星台考古实测，台面至铜梁高9.46米；《元史·天文志》'立表高四丈'；潘鼐《中国恒星观测史》".to_string(),
            },
        ]
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DynastyComparisonRequest {
    pub sun_altitude: f64,
    pub temperature: f64,
    pub pressure: f64,
    pub humidity: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DynastyComparisonResult {
    pub dynasty_id: String,
    pub dynasty_name: String,
    pub gauge_height_chi: f64,
    pub gauge_height_m_actual: f64,
    pub chi_to_m_conversion: f64,
    pub gauge_material: String,
    pub theoretical_shadow_chi: f64,
    pub refracted_shadow_chi: f64,
    pub refraction_correction_arcsec: f64,
    pub shadow_precision_cun: f64,
    pub solstice_precision_seconds: f64,
    pub altitude_resolution_arcmin: f64,
    pub archaeological_source: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MeridianCircle {
    pub instrument_id: String,
    pub instrument_name: String,
    pub era: String,
    pub angle_resolution_arcsec: f64,
    pub time_resolution_ms: f64,
    pub systematic_error_arcsec: f64,
    pub random_error_arcsec: f64,
    pub description: String,
    pub reference: String,
}

impl MeridianCircle {
    pub fn presets() -> Vec<Self> {
        vec![
            Self {
                instrument_id: "yuan_guibiao".to_string(),
                instrument_name: "元代四丈高表".to_string(),
                era: "1276".to_string(),
                angle_resolution_arcsec: 60.0,
                time_resolution_ms: 60000,
                systematic_error_arcsec: 30.0,
                random_error_arcsec: 20.0,
                description: "郭守敬登封观星台四丈高表，影长分辨率约1分(0.1寸)，角度分辨率约1角分，配景符针孔成像".to_string(),
                reference: "《元史·天文志》；潘鼐《中国恒星观测史》；登封观星台考古报告".to_string(),
            },
            Self {
                instrument_id: "modern_meridian_1900".to_string(),
                instrument_name: "20世纪初子午环".to_string(),
                era: "1900".to_string(),
                angle_resolution_arcsec: 0.2,
                time_resolution_ms: 100,
                systematic_error_arcsec: 0.5,
                random_error_arcsec: 0.15,
                description: "经典光学子午环（如普尔科沃/格林尼治天文台），测微目镜读数，物镜口径10-20cm，焦距3-5m".to_string(),
                reference: "IAU 1976天文常数系统；《球面天文学》；格林尼治皇家天文台年报".to_string(),
            },
            Self {
                instrument_id: "modern_meridian_2000".to_string(),
                instrument_name: "现代光电子午环".to_string(),
                era: "2000".to_string(),
                angle_resolution_arcsec: 0.01,
                time_resolution_ms: 1,
                systematic_error_arcsec: 0.03,
                random_error_arcsec: 0.005,
                description: "CCD光电子午环（如USNO、上海天文台光电等高仪），CCD像素细分+自动导星，精度达毫角秒级".to_string(),
                reference: "USNO UCAC4星表精度±15mas； Hipparcos星表±1mas；IAU 2000分辨率".to_string(),
            },
        ]
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MeridianComparisonRequest {
    pub sun_altitude: f64,
    pub temperature: f64,
    pub pressure: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MeridianComparisonResult {
    pub instrument_id: String,
    pub instrument_name: String,
    pub era: String,
    pub measured_altitude_deg: f64,
    pub altitude_error_arcsec: f64,
    pub systematic_error_arcsec: f64,
    pub random_error_arcsec: f64,
    pub shadow_length_if_gnomom_chi: f64,
    pub shadow_error_cun: f64,
    pub solstice_time_error_seconds: f64,
    pub refraction_correction_arcsec: f64,
    pub technology_gap_factor: f64,
    pub reference: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PinholeRequest {
    pub gauge_height_chi: f64,
    pub pinhole_diameter_cun: f64,
    pub sun_altitude: f64,
    pub screen_distance_chi: f64,
    pub temperature: f64,
    pub pressure: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PinholeResult {
    pub pinhole_diameter_cun: f64,
    pub sun_image_diameter_cun: f64,
    pub solar_umbra_blur_cun: f64,
    pub geometric_blur_cun: f64,
    pub diffraction_blur_cun: f64,
    pub total_blur_cun: f64,
    pub optimal_diameter_cun: f64,
    pub airy_disk_radius_cun: f64,
    pub f_number: f64,
    pub signal_to_noise_ratio: f64,
    pub shadow_edge_sharpness: f64,
    pub modulation_transfer_function: f64,
    pub altitude_resolution_arcmin: f64,
    pub magnification: f64,
    pub vignetting_factor: f64,
    pub physics_model_note: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VirtualExperienceRequest {
    pub gauge_height_chi: f64,
    pub latitude: f64,
    pub month: u32,
    pub day: u32,
    pub hour: f64,
    pub temperature: f64,
    pub pressure: f64,
    pub humidity: f64,
    pub time_acceleration: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VirtualExperienceResult {
    pub gauge_height_chi: f64,
    pub sun_altitude: f64,
    pub sun_azimuth: f64,
    pub sun_declination: f64,
    pub equation_of_time_min: f64,
    pub theoretical_shadow_chi: f64,
    pub refracted_shadow_chi: f64,
    pub refraction_correction_arcsec: f64,
    pub shadow_length_cun: f64,
    pub is_daytime: bool,
    pub dynasty_hint: String,
    pub historical_note: String,
    pub time_acceleration_applied: u32,
    pub next_frame_hour: f64,
    pub local_solar_time_hour: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VirtualTimeSeriesPoint {
    pub hour: f64,
    pub sun_altitude: f64,
    pub shadow_chi: f64,
    pub is_daytime: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VirtualTimeSeriesResponse {
    pub points: Vec<VirtualTimeSeriesPoint>,
    pub sunrise_hour: f64,
    pub sunset_hour: f64,
    pub noon_altitude: f64,
    pub total_daylight_hours: f64,
    pub time_acceleration: u32,
    pub dynasty_hint: String,
    pub historical_note: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dynasty_presets_count() {
        let presets = DynastyGnomon::presets();
        assert_eq!(presets.len(), 3);
    }

    #[test]
    fn test_dynasty_presets_data_integrity() {
        let presets = DynastyGnomon::presets();
        let expected_ids = vec!["zhou_tugu", "han_tongbiao", "yuan_sizhang"];

        for (i, p) in presets.iter().enumerate() {
            assert_eq!(p.dynasty_id, expected_ids[i]);
            assert!(!p.dynasty_name.is_empty());
            assert!(!p.period.is_empty());
            assert!(p.gauge_height_chi > 0.0);
            assert!(!p.gauge_material.is_empty());
            assert!(p.gauge_height_error_std_chi > 0.0);
            assert!(p.shadow_reading_error_std_cun > 0.0);
            assert!(!p.description.is_empty());
        }
    }

    #[test]
    fn test_dynasty_gauge_height_scaling() {
        let presets = DynastyGnomon::presets();
        assert_eq!(presets[0].gauge_height_chi, 8.0);
        assert_eq!(presets[1].gauge_height_chi, 8.0);
        assert_eq!(presets[2].gauge_height_chi, 40.0);
        assert_eq!(presets[2].gauge_height_chi / presets[0].gauge_height_chi, 5.0);
    }

    #[test]
    fn test_dynasty_error_progression() {
        let presets = DynastyGnomon::presets();
        assert!(presets[0].gauge_height_error_std_chi > presets[1].gauge_height_error_std_chi);
        assert!(presets[1].gauge_height_error_std_chi > presets[2].gauge_height_error_std_chi);
        assert!(presets[0].shadow_reading_error_std_cun > presets[1].shadow_reading_error_std_cun);
        assert!(presets[1].shadow_reading_error_std_cun > presets[2].shadow_reading_error_std_cun);
    }

    #[test]
    fn test_meridian_presets_count() {
        let presets = MeridianCircle::presets();
        assert_eq!(presets.len(), 3);
    }

    #[test]
    fn test_meridian_presets_data_integrity() {
        let presets = MeridianCircle::presets();
        let expected_ids = vec!["yuan_guibiao", "modern_meridian_1900", "modern_meridian_2000"];

        for (i, p) in presets.iter().enumerate() {
            assert_eq!(p.instrument_id, expected_ids[i]);
            assert!(!p.instrument_name.is_empty());
            assert!(!p.era.is_empty());
            assert!(p.angle_resolution_arcsec > 0.0);
            assert!(p.time_resolution_ms > 0.0);
            assert!(p.systematic_error_arcsec > 0.0);
            assert!(!p.description.is_empty());
        }
    }

    #[test]
    fn test_meridian_resolution_progression() {
        let presets = MeridianCircle::presets();
        assert!(presets[0].angle_resolution_arcsec > presets[1].angle_resolution_arcsec);
        assert!(presets[1].angle_resolution_arcsec > presets[2].angle_resolution_arcsec);
        assert!(presets[0].systematic_error_arcsec > presets[1].systematic_error_arcsec);
        assert!(presets[1].systematic_error_arcsec > presets[2].systematic_error_arcsec);
        assert!(presets[0].time_resolution_ms > presets[1].time_resolution_ms);
        assert!(presets[1].time_resolution_ms > presets[2].time_resolution_ms);
    }

    #[test]
    fn test_meridian_technology_gap_ratios() {
        let presets = MeridianCircle::presets();
        let ratio_1900_1276 = presets[0].systematic_error_arcsec / presets[1].systematic_error_arcsec;
        let ratio_2000_1276 = presets[0].systematic_error_arcsec / presets[2].systematic_error_arcsec;
        let ratio_2000_1900 = presets[1].systematic_error_arcsec / presets[2].systematic_error_arcsec;

        assert!(ratio_1900_1276 > 20.0, "1900年精度应比元代高20倍以上，实际{}倍", ratio_1900_1276);
        assert!(ratio_2000_1276 > 500.0, "2000年精度应比元代高500倍以上，实际{}倍", ratio_2000_1276);
        assert!(ratio_2000_1900 > 10.0, "2000年精度应比1900年高10倍以上，实际{}倍", ratio_2000_1900);
    }

    #[test]
    fn test_meridian_ids_are_unique() {
        let presets = MeridianCircle::presets();
        let ids: Vec<&String> = presets.iter().map(|p| &p.instrument_id).collect();
        assert_eq!(ids[0], "yuan_guibiao");
        assert_eq!(ids[1], "modern_meridian_1900");
        assert_eq!(ids[2], "modern_meridian_2000");
    }

    #[test]
    fn test_dynasty_gnomon_serialization() {
        let preset = &DynastyGnomon::presets()[0];
        let json = serde_json::to_string(preset).expect("序列化失败");
        let deserialized: DynastyGnomon = serde_json::from_str(&json).expect("反序列化失败");
        assert_eq!(preset.dynasty_id, deserialized.dynasty_id);
        assert_eq!(preset.gauge_height_chi, deserialized.gauge_height_chi);
    }

    #[test]
    fn test_meridian_circle_serialization() {
        let preset = &MeridianCircle::presets()[0];
        let json = serde_json::to_string(preset).expect("序列化失败");
        let deserialized: MeridianCircle = serde_json::from_str(&json).expect("反序列化失败");
        assert_eq!(preset.instrument_id, deserialized.instrument_id);
        assert_eq!(preset.angle_resolution_arcsec, deserialized.angle_resolution_arcsec);
    }

    #[test]
    fn test_pinhole_request_default_values() {
        let req = PinholeRequest {
            gauge_height_chi: 40.0,
            pinhole_diameter_cun: 1.0,
            sun_altitude: 26.0,
            screen_distance_chi: 40.0,
            temperature: 5.0,
            pressure: 1013.25,
        };
        assert_eq!(req.gauge_height_chi, 40.0);
        assert_eq!(req.pinhole_diameter_cun, 1.0);
    }

    #[test]
    fn test_pinhole_result_fields() {
        let result = PinholeResult {
            pinhole_diameter_cun: 1.0,
            sun_image_diameter_cun: 0.5,
            geometric_blur_cun: 0.1,
            diffraction_blur_cun: 0.05,
            total_blur_cun: 0.1118,
            optimal_diameter_cun: 0.2,
            signal_to_noise_ratio: 10.0,
            shadow_edge_sharpness: 0.8,
            altitude_resolution_arcmin: 0.5,
            magnification: 1.0,
            vignetting_factor: 0.95,
        };
        assert!(result.shadow_edge_sharpness >= 0.0 && result.shadow_edge_sharpness <= 1.0);
        assert!(result.vignetting_factor >= 0.0 && result.vignetting_factor <= 1.0);
        assert!(result.signal_to_noise_ratio > 0.0);
    }

    #[test]
    fn test_virtual_experience_request_bounds() {
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
        assert!(req.month >= 1 && req.month <= 12);
        assert!(req.day >= 1 && req.day <= 31);
        assert!(req.hour >= 0.0 && req.hour <= 24.0);
    }

    #[test]
    fn test_dynasty_comparison_request_valid_range() {
        let req = DynastyComparisonRequest {
            sun_altitude: 26.0,
            temperature: 5.0,
            pressure: 1013.25,
            humidity: 50.0,
        };
        assert!(req.sun_altitude >= -90.0 && req.sun_altitude <= 90.0);
        assert!(req.pressure >= 500.0 && req.pressure <= 1100.0);
        assert!(req.humidity >= 0.0 && req.humidity <= 100.0);
    }

    #[test]
    fn test_meridian_comparison_request_valid_range() {
        let req = MeridianComparisonRequest {
            sun_altitude: 26.0,
            temperature: 5.0,
            pressure: 1013.25,
        };
        assert!(req.sun_altitude >= -90.0 && req.sun_altitude <= 90.0);
        assert!(req.temperature >= -40.0 && req.temperature <= 60.0);
    }

    #[test]
    fn test_dynasty_gnomon_latitude_range() {
        let presets = DynastyGnomon::presets();
        for p in &presets {
            assert!(p.latitude >= 0.0 && p.latitude <= 90.0,
                "纬度应在北半球: {}", p.latitude);
            assert!(p.longitude >= 73.0 && p.longitude <= 135.0,
                "经度应在中国范围内: {}", p.longitude);
        }
    }
}
