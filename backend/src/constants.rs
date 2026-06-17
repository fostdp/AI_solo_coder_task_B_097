pub const ARCSEC_TO_DEG: f64 = 1.0 / 3600.0;
pub const DEG_TO_RAD: f64 = std::f64::consts::PI / 180.0;
pub const CHI_TO_CUN: f64 = 10.0;
pub const CHI_TO_M: f64 = 0.3333;
pub const CUN_TO_M: f64 = CHI_TO_M / CHI_TO_CUN;
pub const WAVELENGTH_NM: f64 = 550.0;
pub const SPEED_OF_LIGHT_M_S: f64 = 299792458.0;

pub const BOOTSTRAP_RESAMPLES: usize = 2000;
pub const JACKKNIFE_MIN: usize = 10;

pub const DEFAULT_STATION_LAT: f64 = 34.4897;
pub const DEFAULT_STATION_LON: f64 = 113.0875;
pub const DEFAULT_STATION_ALT: f64 = 420.0;

pub const SOLAR_ANGULAR_DIAMETER_RAD: f64 = 0.00930;

pub const MONTE_CARLO_THREAD_POOL_SIZE: usize = 4;
