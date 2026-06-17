use crate::constants::*;
use crate::dynasty_models::*;

pub struct PinholeOptimizer;

impl PinholeOptimizer {
    pub fn simulate(request: &PinholeRequest) -> PinholeResult {
        let d_m = request.pinhole_diameter_cun.abs().max(1e-9) * CUN_TO_M;
        let h_m = request.gauge_height_chi * CHI_TO_M;
        let s_m = request.screen_distance_chi * CHI_TO_M;
        let alt_rad = request.sun_altitude.max(0.1) * DEG_TO_RAD;

        let solar_umbra_m = SOLAR_ANGULAR_DIAMETER_RAD * s_m;
        let solar_umbra_cun = solar_umbra_m / CUN_TO_M;

        let geometric_blur_m = d_m * s_m / h_m;
        let geometric_blur_cun = geometric_blur_m / CUN_TO_M;

        let wavelength_m = WAVELENGTH_NM * 1e-9;
        let diffraction_blur_rad = 1.22 * wavelength_m / d_m;
        let airy_radius_m = diffraction_blur_rad * s_m;
        let diffraction_blur_m = airy_radius_m;
        let diffraction_blur_cun = diffraction_blur_m / CUN_TO_M;
        let airy_disk_radius_cun = airy_radius_m / CUN_TO_M;

        let f_number = h_m / d_m;

        let total_blur_m = (
            solar_umbra_m.powi(2)
            + geometric_blur_m.powi(2)
            + diffraction_blur_m.powi(2)
        ).sqrt();
        let total_blur_cun = total_blur_m / CUN_TO_M;

        let optimal_diameter_m = (1.22 * wavelength_m * h_m).sqrt();
        let optimal_diameter_cun = optimal_diameter_m / CUN_TO_M;

        let sun_image_diameter_m = SOLAR_ANGULAR_DIAMETER_RAD * s_m;
        let sun_image_diameter_cun = sun_image_diameter_m / CUN_TO_M;

        let magnification = s_m / h_m;

        let signal_area = std::f64::consts::PI * (sun_image_diameter_m / 2.0).powi(2);
        let blur_area = std::f64::consts::PI * (total_blur_m / 2.0).powi(2);
        let snr = (signal_area / blur_area.max(1e-15)).sqrt();

        let shadow_edge_sharpness = 1.0 / (1.0 + (total_blur_cun / sun_image_diameter_cun.max(0.001)).powi(2));

        let cutoff_freq = d_m / (wavelength_m * h_m);
        let mtf_reference_freq = 0.5 * cutoff_freq;
        let modulation_transfer_function = if mtf_reference_freq > 0.0 {
            let x = 1.22 * std::f64::consts::PI * mtf_reference_freq * wavelength_m * h_m / d_m;
            if x.abs() < 1e-6 {
                1.0
            } else {
                let j1 = x.sin() / x - x.cos();
                2.0 * j1 / x
            }
        } else {
            0.0
        }.abs().min(1.0);

        let alt_resolution_rad = total_blur_cun / (CHI_TO_CUN * request.gauge_height_chi);
        let alt_resolution_arcmin = alt_resolution_rad * (180.0 / std::f64::consts::PI) * 60.0;

        let cos_alt = alt_rad.cos();
        let vignetting = (1.0 - (d_m / (2.0 * h_m * cos_alt)).powi(2)).max(0.0);

        let physics_model_note = format!(
            "物理光学模型：太阳本影模糊(θ☉·s) + 几何模糊(d·s/h) + 夫琅禾费衍射(1.22λ/D·s)；\
             瑞利判据爱里斑；截止频率f_c=D/(λh)；F数={:.1}",
            f_number
        );

        PinholeResult {
            pinhole_diameter_cun: request.pinhole_diameter_cun,
            sun_image_diameter_cun,
            solar_umbra_blur_cun: solar_umbra_cun,
            geometric_blur_cun,
            diffraction_blur_cun,
            total_blur_cun,
            optimal_diameter_cun,
            airy_disk_radius_cun,
            f_number,
            signal_to_noise_ratio: snr,
            shadow_edge_sharpness,
            modulation_transfer_function,
            altitude_resolution_arcmin,
            magnification,
            vignetting_factor: vignetting,
            physics_model_note,
        }
    }

    pub fn scan_optimal_diameter(
        gauge_height_chi: f64,
        min_diameter_cun: f64,
        max_diameter_cun: f64,
        steps: usize,
    ) -> Vec<(f64, f64)> {
        let mut results = Vec::new();
        let step = (max_diameter_cun - min_diameter_cun) / steps.max(1) as f64;

        for i in 0..=steps {
            let d = min_diameter_cun + i as f64 * step;
            let req = PinholeRequest {
                gauge_height_chi,
                pinhole_diameter_cun: d,
                sun_altitude: 26.0,
                screen_distance_chi: gauge_height_chi,
                temperature: 5.0,
                pressure: 1013.25,
            };
            let result = Self::simulate(&req);
            results.push((d, result.total_blur_cun));
        }

        results
    }

    pub fn theoretical_optimal_diameter(gauge_height_chi: f64) -> f64 {
        let h_m = gauge_height_chi * CHI_TO_M;
        let wavelength_m = WAVELENGTH_NM * 1e-9;
        let optimal_m = (1.22 * wavelength_m * h_m).sqrt();
        optimal_m / CUN_TO_M
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pinhole_simulate_normal() {
        let req = PinholeRequest {
            gauge_height_chi: 40.0,
            pinhole_diameter_cun: 1.0,
            sun_altitude: 26.0,
            screen_distance_chi: 40.0,
            temperature: 5.0,
            pressure: 1013.25,
        };
        let result = PinholeOptimizer::simulate(&req);
        assert!(result.total_blur_cun > 0.0);
        assert!(result.shadow_edge_sharpness > 0.0 && result.shadow_edge_sharpness <= 1.0);
    }

    #[test]
    fn test_theoretical_optimal_diameter() {
        let optimal = PinholeOptimizer::theoretical_optimal_diameter(40.0);
        assert!(optimal > 0.0 && optimal < 1.0);
    }

    #[test]
    fn test_scan_optimal_diameter() {
        let scan = PinholeOptimizer::scan_optimal_diameter(40.0, 0.01, 10.0, 20);
        assert_eq!(scan.len(), 21);
        let mut min_blur = f64::MAX;
        let mut min_d = 0.0;
        for (d, blur) in &scan {
            if *blur < min_blur {
                min_blur = *blur;
                min_d = *d;
            }
        }
        assert!(min_d > 0.0 && min_d < 2.0);
    }

    #[test]
    fn test_pinhole_boundary_small_diameter_diffraction_dominant() {
        let req = PinholeRequest {
            gauge_height_chi: 40.0,
            pinhole_diameter_cun: 0.01,
            sun_altitude: 26.0,
            screen_distance_chi: 40.0,
            temperature: 5.0,
            pressure: 1013.25,
        };
        let result = PinholeOptimizer::simulate(&req);
        assert!(result.diffraction_blur_cun > result.geometric_blur_cun);
    }

    #[test]
    fn test_pinhole_boundary_large_diameter_geometric_dominant() {
        let req = PinholeRequest {
            gauge_height_chi: 40.0,
            pinhole_diameter_cun: 10.0,
            sun_altitude: 26.0,
            screen_distance_chi: 40.0,
            temperature: 5.0,
            pressure: 1013.25,
        };
        let result = PinholeOptimizer::simulate(&req);
        assert!(result.geometric_blur_cun > result.diffraction_blur_cun);
    }
}
