use serde::{Deserialize, Serialize};

const CHI_TO_M: f64 = 0.3333;
const CUN_TO_MM: f64 = 33.33;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DynastyGnomon {
    pub dynasty_id: String,
    pub dynasty_name: String,
    pub period: String,
    pub gauge_height_chi: f64,
    pub gauge_material: String,
    pub gauge_height_error_std_chi: f64,
    pub shadow_reading_error_std_cun: f64,
    pub latitude: f64,
    pub longitude: f64,
    pub altitude: f64,
    pub description: String,
}

impl DynastyGnomon {
    pub fn presets() -> Vec<Self> {
        vec![
            Self {
                dynasty_id: "zhou_tugu".to_string(),
                dynasty_name: "周代土圭".to_string(),
                period: "公元前11世纪—前256年".to_string(),
                gauge_height_chi: 8.0,
                gauge_material: "土筑".to_string(),
                gauge_height_error_std_chi: 0.1,
                shadow_reading_error_std_cun: 2.0,
                latitude: 34.25,
                longitude: 108.93,
                altitude: 400.0,
                description: "《周礼》载土圭之法，表高八尺，以土筑成，精度受限".to_string(),
            },
            Self {
                dynasty_id: "han_tongbiao".to_string(),
                dynasty_name: "汉代铜表".to_string(),
                period: "公元前206年—公元220年".to_string(),
                gauge_height_chi: 8.0,
                gauge_material: "青铜铸造".to_string(),
                gauge_height_error_std_chi: 0.02,
                shadow_reading_error_std_cun: 0.5,
                latitude: 34.26,
                longitude: 108.94,
                altitude: 405.0,
                description: "汉代以铜铸表，表高八尺，材质稳定，刻度精确".to_string(),
            },
            Self {
                dynasty_id: "yuan_sizhang".to_string(),
                dynasty_name: "元代四丈高表".to_string(),
                period: "1276年—1368年".to_string(),
                gauge_height_chi: 40.0,
                gauge_material: "砖石砌筑+铜横梁".to_string(),
                gauge_height_error_std_chi: 0.01,
                shadow_reading_error_std_cun: 0.2,
                latitude: 34.4897,
                longitude: 113.0875,
                altitude: 420.0,
                description: "郭守敬建登封观星台，表高四丈(40尺)，横梁针孔成像，精度达古代巅峰".to_string(),
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
    pub gauge_material: String,
    pub theoretical_shadow_chi: f64,
    pub refracted_shadow_chi: f64,
    pub refraction_correction_arcsec: f64,
    pub shadow_precision_cun: f64,
    pub solstice_precision_seconds: f64,
    pub altitude_resolution_arcmin: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MeridianCircle {
    pub instrument_id: String,
    pub instrument_name: String,
    pub era: String,
    pub angle_resolution_arcsec: f64,
    pub time_resolution_ms: f64,
    pub systematic_error_arcsec: f64,
    pub description: String,
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
                description: "郭守敬高表，影长分辨率约1分，角度分辨率约1角分".to_string(),
            },
            Self {
                instrument_id: "modern_meridian_1900".to_string(),
                instrument_name: "20世纪初子午环".to_string(),
                era: "1900".to_string(),
                angle_resolution_arcsec: 0.5,
                time_resolution_ms: 100,
                systematic_error_arcsec: 1.0,
                description: "经典光学子午环，测微显微镜读数，精度约0.5角秒".to_string(),
            },
            Self {
                instrument_id: "modern_meridian_2000".to_string(),
                instrument_name: "现代光电子午环".to_string(),
                era: "2000".to_string(),
                angle_resolution_arcsec: 0.01,
                time_resolution_ms: 1,
                systematic_error_arcsec: 0.05,
                description: "CCD光电读数子午环，精度达0.01角秒级别".to_string(),
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
    pub shadow_length_if_gnomon_chi: f64,
    pub shadow_error_cun: f64,
    pub solstice_time_error_seconds: f64,
    pub refraction_correction_arcsec: f64,
    pub technology_gap_factor: f64,
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
    pub geometric_blur_cun: f64,
    pub diffraction_blur_cun: f64,
    pub total_blur_cun: f64,
    pub optimal_diameter_cun: f64,
    pub signal_to_noise_ratio: f64,
    pub shadow_edge_sharpness: f64,
    pub altitude_resolution_arcmin: f64,
    pub magnification: f64,
    pub vignetting_factor: f64,
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
}
