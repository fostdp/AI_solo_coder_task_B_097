use std::sync::Arc;
use uuid::Uuid;

use guibiao_backend::dynasty_models::*;
use guibiao_backend::dynasty_comparison::*;
use guibiao_backend::models::*;
use guibiao_backend::dtu_receiver::*;

// =========================================================================
// 集成测试：精度对比验证测量误差
// =========================================================================

#[tokio::test]
async fn test_dynasty_comparison_integration_full_pipeline() {
    let req = DynastyComparisonRequest {
        sun_altitude: 26.0,
        temperature: 5.0,
        pressure: 1013.25,
        humidity: 50.0,
    };

    let results = DynastyComparator::compare(&req);

    assert_eq!(results.len(), 3);

    for (i, r) in results.iter().enumerate() {
        assert!(!r.dynasty_id.is_empty());
        assert!(!r.dynasty_name.is_empty());
        assert!(r.gauge_height_chi > 0.0);
        assert!(r.refraction_correction_arcsec > 0.0);
        assert!(r.shadow_precision_cun > 0.0);
        assert!(r.solstice_precision_seconds > 0.0);
        assert!(r.altitude_resolution_arcmin > 0.0);

        if i == 0 {
            assert_eq!(r.dynasty_id, "zhou_tugu");
            assert!(r.theoretical_shadow_chi > 15.0);
        }
        if i == 2 {
            assert_eq!(r.dynasty_id, "yuan_sizhang");
            assert!(r.theoretical_shadow_chi > 75.0);
        }
    }

    let zhou_shadow = results[0].theoretical_shadow_chi;
    let yuan_shadow = results[2].theoretical_shadow_chi;
    assert!((yuan_shadow / zhou_shadow - 5.0).abs() < 0.01,
        "元代表高是周代5倍，影长也应约为5倍，实际比例: {}", yuan_shadow / zhou_shadow);

    assert!(results[2].solstice_precision_seconds < results[0].solstice_precision_seconds,
        "元代冬至精度应高于周代: 元代={}s, 周代={}s",
        results[2].solstice_precision_seconds, results[0].solstice_precision_seconds);
}

#[tokio::test]
async fn test_dynasty_comparison_with_various_conditions() {
    let conditions = vec![
        (26.0, 5.0, 1013.25, "冬至正午"),
        (80.0, 30.0, 1013.25, "夏至正午"),
        (5.0, -10.0, 1030.0, "冬季日出"),
        (45.0, 20.0, 1013.25, "春秋分正午"),
        (15.0, 0.0, 1000.0, "高海拔地区"),
    ];

    for (alt, temp, pressure, desc) in conditions {
        let req = DynastyComparisonRequest {
            sun_altitude: alt,
            temperature: temp,
            pressure,
            humidity: 50.0,
        };

        let results = DynastyComparator::compare(&req);
        assert_eq!(results.len(), 3, "{}: 应返回3个朝代结果", desc);

        for r in &results {
            assert!(!r.theoretical_shadow_chi.is_nan(), "{}: {} 影长不应为NaN", desc, r.dynasty_name);
            assert!(!r.refraction_correction_arcsec.is_nan(), "{}: {} 蒙气差不应为NaN", desc, r.dynasty_name);
        }

        let scale_ratio = results[2].theoretical_shadow_chi / results[0].theoretical_shadow_chi;
        if alt > 0.0 {
            assert!((scale_ratio - 5.0).abs() < 0.1,
                "{}: 表高5倍影长比例应约5倍，实际: {}", desc, scale_ratio);
        }
    }
}

// =========================================================================
// 集成测试：跨时代对比验证技术进步
// =========================================================================

#[tokio::test]
async fn test_meridian_comparison_full_technology_evolution() {
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

    let err_yuan = results[0].altitude_error_arcsec;
    let err_1900 = results[1].altitude_error_arcsec;
    let err_2000 = results[2].altitude_error_arcsec;

    assert!(err_yuan > err_1900,
        "元代误差应大于1900年: 元代={}, 1900年={}", err_yuan, err_1900);
    assert!(err_1900 > err_2000,
        "1900年误差应大于2000年: 1900年={}, 2000年={}", err_1900, err_2000);

    assert_eq!(results[0].technology_gap_factor, 1.0);
    assert!(results[1].technology_gap_factor > 1.0);
    assert!(results[2].technology_gap_factor > results[1].technology_gap_factor);

    let gap_yuan_to_2000 = err_yuan / err_2000;
    assert!(gap_yuan_to_2000 > 500.0,
        "元代到2000年精度提升应超过500倍，实际: {}倍", gap_yuan_to_2000);

    let shadow_err_yuan = results[0].shadow_error_cun;
    let shadow_err_2000 = results[2].shadow_error_cun;
    assert!(shadow_err_yuan > shadow_err_2000 * 100.0,
        "元代影长误差应大于2000年100倍: 元代={}寸, 2000年={}寸",
        shadow_err_yuan, shadow_err_2000);
}

#[tokio::test]
async fn test_meridian_comparison_integration_solstice_context() {
    let req = MeridianComparisonRequest {
        sun_altitude: 31.5,
        temperature: 0.0,
        pressure: 1015.0,
    };

    let results = MeridianComparator::compare(&req);

    assert!(results[0].solstice_time_error_seconds > 60.0,
        "元代冬至时刻误差应大于1分钟，实际: {}s", results[0].solstice_time_error_seconds);
    assert!(results[2].solstice_time_error_seconds < 1.0,
        "2000年冬至时刻误差应小于1秒，实际: {}s", results[2].solstice_time_error_seconds);

    let improvement_factor = results[0].solstice_time_error_seconds / results[2].solstice_time_error_seconds;
    assert!(improvement_factor > 100.0,
        "冬至时刻精度提升应超过100倍，实际: {}倍", improvement_factor);
}

// =========================================================================
// 集成测试：针孔成像验证影长清晰度
// =========================================================================

#[tokio::test]
async fn test_pinhole_simulation_full_pipeline() {
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
    assert!(result.geometric_blur_cun > 0.0);
    assert!(result.diffraction_blur_cun > 0.0);
    assert!(result.total_blur_cun > 0.0);
    assert!(result.optimal_diameter_cun > 0.0);
    assert!(result.signal_to_noise_ratio > 0.0);
    assert!(result.shadow_edge_sharpness > 0.0 && result.shadow_edge_sharpness <= 1.0);
    assert!(result.altitude_resolution_arcmin > 0.0);
    assert!(result.magnification > 0.0);
    assert!(result.vignetting_factor >= 0.0 && result.vignetting_factor <= 1.0);

    assert!((result.total_blur_cun.powi(2)
        - result.geometric_blur_cun.powi(2)
        - result.diffraction_blur_cun.powi(2)).abs() < 1e-6,
        "总模糊的平方应等于几何模糊平方加衍射模糊平方");

    assert!(result.optimal_diameter_cun > 0.0 && result.optimal_diameter_cun < 1.0,
        "最优孔径应在合理范围内: {}", result.optimal_diameter_cun);
}

#[tokio::test]
async fn test_pinhole_simulation_blur_tradeoff() {
    let diameters = vec![0.05, 0.1, 0.2, 0.5, 1.0, 2.0, 5.0, 10.0];
    let mut geometric_trend = Vec::new();
    let mut diffraction_trend = Vec::new();
    let mut total_blur_values = Vec::new();

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
        geometric_trend.push(result.geometric_blur_cun);
        diffraction_trend.push(result.diffraction_blur_cun);
        total_blur_values.push(result.total_blur_cun);
    }

    for i in 1..geometric_trend.len() {
        assert!(geometric_trend[i] >= geometric_trend[i-1],
            "几何模糊应随孔径增大而递增: {} -> {}", geometric_trend[i-1], geometric_trend[i]);
    }

    for i in 1..diffraction_trend.len() {
        assert!(diffraction_trend[i] <= diffraction_trend[i-1],
            "衍射模糊应随孔径增大而递减: {} -> {}", diffraction_trend[i-1], diffraction_trend[i]);
    }

    let min_blur_idx = total_blur_values.iter()
        .enumerate()
        .min_by(|a, b| a.1.partial_cmp(b.1).unwrap())
        .map(|(i, _)| i)
        .unwrap();

    assert!(min_blur_idx > 0 && min_blur_idx < diameters.len() - 1,
        "最优孔径应在中间位置，实际在索引{}", min_blur_idx);

    let optimal_found = diameters[min_blur_idx];
    assert!(optimal_found > 0.1 && optimal_found < 2.0,
        "最优孔径应在0.1到2.0寸之间，实际: {}寸", optimal_found);
}

#[tokio::test]
async fn test_pinhole_simulation_edge_cases() {
    let test_cases = vec![
        (0.01, "极小孔径", true),
        (0.1, "小孔径", false),
        (1.0, "中等孔径", false),
        (10.0, "大孔径", false),
        (100.0, "极大孔径", false),
    ];

    for (diameter, desc, expect_diffraction_dominant) in test_cases {
        let req = PinholeRequest {
            gauge_height_chi: 40.0,
            pinhole_diameter_cun: diameter,
            sun_altitude: 26.0,
            screen_distance_chi: 40.0,
            temperature: 5.0,
            pressure: 1013.25,
        };
        let result = PinholeSimulator::simulate(&req);

        assert!(!result.total_blur_cun.is_nan(), "{}: 总模糊不应为NaN", desc);
        assert!(!result.shadow_edge_sharpness.is_nan(), "{}: 锐度不应为NaN", desc);
        assert!(result.shadow_edge_sharpness >= 0.0 && result.shadow_edge_sharpness <= 1.0,
            "{}: 锐度应在[0,1]范围内，实际: {}", desc, result.shadow_edge_sharpness);

        if expect_diffraction_dominant {
            assert!(result.diffraction_blur_cun > result.geometric_blur_cun,
                "{}: 衍射应主导: 衍射={}, 几何={}",
                desc, result.diffraction_blur_cun, result.geometric_blur_cun);
        }

        assert!(result.signal_to_noise_ratio > 0.0,
            "{}: 信噪比应大于0，实际: {}", desc, result.signal_to_noise_ratio);
    }
}

#[tokio::test]
async fn test_pinhole_with_different_gauge_heights() {
    let heights = vec![8.0, 16.0, 32.0, 40.0];

    for h in &heights {
        let req = PinholeRequest {
            gauge_height_chi: *h,
            pinhole_diameter_cun: 1.0,
            sun_altitude: 26.0,
            screen_distance_chi: *h,
            temperature: 5.0,
            pressure: 1013.25,
        };
        let result = PinholeSimulator::simulate(&req);

        assert!(!result.optimal_diameter_cun.is_nan(),
            "表高{}尺: 最优孔径不应为NaN", h);
        assert!(result.optimal_diameter_cun > 0.0,
            "表高{}尺: 最优孔径应大于0，实际: {}", h, result.optimal_diameter_cun);

        let expected_optimal = (1.22 * 550e-9 * (*h * 0.3333)).sqrt() / 0.03333;
        assert!((result.optimal_diameter_cun - expected_optimal).abs() / expected_optimal < 0.1,
            "表高{}尺: 最优孔径应接近理论值: 实际={}, 理论={}",
            h, result.optimal_diameter_cun, expected_optimal);
    }
}

// =========================================================================
// 集成测试：虚拟体验测试交互教育性
// =========================================================================

#[tokio::test]
async fn test_virtual_experience_full_interaction() {
    let test_scenarios = vec![
        (8.0, 12, 22, 12.0, "周代/汉代", "冬至正午"),
        (40.0, 12, 22, 12.0, "元代", "元代冬至正午"),
        (8.0, 6, 22, 12.0, "周代/汉代", "夏至正午"),
        (8.0, 3, 21, 12.0, "周代/汉代", "春分正午"),
        (15.0, 12, 22, 12.0, "南北朝/唐代", "特殊表高"),
        (25.0, 12, 22, 12.0, "宋代", "宋代表高"),
    ];

    for (gauge, month, day, hour, expected_dynasty, desc) in test_scenarios {
        let req = VirtualExperienceRequest {
            gauge_height_chi: gauge,
            latitude: 34.49,
            month,
            day,
            hour,
            temperature: 5.0,
            pressure: 1013.0,
            humidity: 50.0,
        };
        let result = VirtualExperienceSimulator::simulate(&req);

        assert_eq!(result.dynasty_hint, expected_dynasty,
            "{}: 表高{}尺应归属{}，实际: {}", desc, gauge, expected_dynasty, result.dynasty_hint);
        assert!(!result.historical_note.is_empty(),
            "{}: 历史说明不应为空", desc);
        assert!(result.historical_note.chars().count() > 10,
            "{}: 历史说明应包含足够教育内容", desc);
        assert!(result.sun_altitude > 0.0,
            "{}: 白天太阳高度角应大于0，实际: {}", desc, result.sun_altitude);
        assert!(result.is_daytime,
            "{}: 正午应为白天", desc);
        assert!(result.theoretical_shadow_chi > 0.0,
            "{}: 白天影长应大于0，实际: {}", desc, result.theoretical_shadow_chi);
    }
}

#[tokio::test]
async fn test_virtual_experience_educational_content() {
    let heights = vec![4.0, 8.0, 12.0, 18.0, 25.0, 35.0, 40.0, 50.0];
    let mut seen_dynasties = std::collections::HashSet::new();

    for h in &heights {
        let req = VirtualExperienceRequest {
            gauge_height_chi: *h,
            latitude: 34.49,
            month: 12,
            day: 22,
            hour: 12.0,
            temperature: 0.0,
            pressure: 1013.0,
            humidity: 50.0,
        };
        let result = VirtualExperienceSimulator::simulate(&req);

        seen_dynasties.insert(result.dynasty_hint.clone());

        assert!(!result.dynasty_hint.is_empty());
        assert!(!result.historical_note.is_empty());
        assert!(result.historical_note.contains("圭表") ||
            result.historical_note.contains("土圭") ||
            result.historical_note.contains("郭守敬") ||
            result.historical_note.contains("《周礼》") ||
            result.historical_note.contains("《景表议》") ||
            result.historical_note.contains("沈括") ||
            result.historical_note.contains("一行") ||
            result.historical_note.contains("何承天"),
            "历史说明应包含历史关键词: {}", result.historical_note);
    }

    assert!(seen_dynasties.len() >= 4,
        "应覆盖4个以上朝代提示，实际: {:?}", seen_dynasties);
}

#[tokio::test]
async fn test_virtual_experience_daytime_cycle() {
    let hours = vec![5.0, 7.0, 9.0, 12.0, 15.0, 17.0, 19.0, 21.0];
    let mut altitudes = Vec::new();
    let mut daytimes = Vec::new();

    for h in &hours {
        let req = VirtualExperienceRequest {
            gauge_height_chi: 40.0,
            latitude: 34.49,
            month: 6,
            day: 22,
            hour: *h,
            temperature: 20.0,
            pressure: 1013.0,
            humidity: 50.0,
        };
        let result = VirtualExperienceSimulator::simulate(&req);
        altitudes.push(result.sun_altitude);
        daytimes.push(result.is_daytime);
    }

    let noon_idx = hours.iter().position(|&h| h == 12.0).unwrap();
    let max_alt = altitudes.iter().fold(f64::NEG_INFINITY, |a, &b| a.max(b));
    assert!((altitudes[noon_idx] - max_alt).abs() < 0.1,
        "正午太阳应达到最高高度: 正午={}, 最高={}", altitudes[noon_idx], max_alt);

    let morning_alts: Vec<f64> = altitudes[..=noon_idx].to_vec();
    for i in 1..morning_alts.len() {
        assert!(morning_alts[i] >= morning_alts[i-1] - 0.5,
            "上午太阳高度应递增: {}点={} -> {}点={}",
            hours[i-1], morning_alts[i-1], hours[i], morning_alts[i]);
    }

    let afternoon_alts: Vec<f64> = altitudes[noon_idx..].to_vec();
    for i in 1..afternoon_alts.len() {
        assert!(afternoon_alts[i] <= afternoon_alts[i-1] + 0.5,
            "下午太阳高度应递减: {}点={} -> {}点={}",
            hours[noon_idx + i - 1], afternoon_alts[i-1], hours[noon_idx + i], afternoon_alts[i]);
    }

    assert_eq!(daytimes[0], false, "5点应为夜间");
    assert_eq!(daytimes[3], true, "12点应为白天");
    assert_eq!(daytimes[7], false, "21点应为夜间");
}

#[tokio::test]
async fn test_virtual_experience_shadow_scaling() {
    let heights = vec![8.0, 16.0, 24.0, 32.0, 40.0];
    let mut results = Vec::new();

    for h in &heights {
        let req = VirtualExperienceRequest {
            gauge_height_chi: *h,
            latitude: 34.49,
            month: 12,
            day: 22,
            hour: 12.0,
            temperature: 0.0,
            pressure: 1013.0,
            humidity: 50.0,
        };
        results.push((*h, VirtualExperienceSimulator::simulate(&req)));
    }

    for i in 0..results.len() {
        for j in i+1..results.len() {
            let (h1, r1) = &results[i];
            let (h2, r2) = &results[j];
            let ratio_h = h2 / h1;
            let ratio_shadow = r2.theoretical_shadow_chi / r1.theoretical_shadow_chi;

            assert!((ratio_h - ratio_shadow).abs() < 0.01,
                "表高{}倍，影长应约{}倍，实际: 表高{}倍 -> 影长{}倍",
                ratio_h, ratio_h, ratio_h, ratio_shadow);
        }
    }
}

// =========================================================================
// 集成测试：边界和异常输入处理
// =========================================================================

#[tokio::test]
async fn test_extreme_inputs_handled_gracefully() {
    let extreme_cases = vec![
        ("极高高度角", DynastyComparisonRequest {
            sun_altitude: 89.9, temperature: 30.0, pressure: 1013.25, humidity: 50.0,
        }),
        ("极低高度角", DynastyComparisonRequest {
            sun_altitude: 0.1, temperature: -20.0, pressure: 1030.0, humidity: 30.0,
        }),
        ("极高温", DynastyComparisonRequest {
            sun_altitude: 45.0, temperature: 60.0, pressure: 980.0, humidity: 10.0,
        }),
        ("极低温", DynastyComparisonRequest {
            sun_altitude: 45.0, temperature: -60.0, pressure: 1040.0, humidity: 80.0,
        }),
        ("极低气压", DynastyComparisonRequest {
            sun_altitude: 45.0, temperature: 15.0, pressure: 500.0, humidity: 50.0,
        }),
    ];

    for (desc, req) in extreme_cases {
        let results = DynastyComparator::compare(&req);
        assert_eq!(results.len(), 3, "{}: 应返回3个结果", desc);
        for r in &results {
            assert!(!r.theoretical_shadow_chi.is_nan(), "{}: {} 影长不应为NaN", desc, r.dynasty_name);
            assert!(!r.refraction_correction_arcsec.is_nan(), "{}: {} 蒙气差不应为NaN", desc, r.dynasty_name);
            assert!(!r.solstice_precision_seconds.is_nan() && r.solstice_precision_seconds.is_finite(),
                "{}: {} 冬至精度应为有限值", desc, r.dynasty_name);
        }
    }
}

#[tokio::test]
async fn test_invalid_virtual_experience_inputs() {
    let invalid_cases = vec![
        ("月份15", VirtualExperienceRequest {
            gauge_height_chi: 8.0, latitude: 34.49, month: 15, day: 22, hour: 12.0,
            temperature: 5.0, pressure: 1013.0, humidity: 50.0,
        }),
        ("日期32", VirtualExperienceRequest {
            gauge_height_chi: 8.0, latitude: 34.49, month: 1, day: 32, hour: 12.0,
            temperature: 5.0, pressure: 1013.0, humidity: 50.0,
        }),
        ("小时25", VirtualExperienceRequest {
            gauge_height_chi: 8.0, latitude: 34.49, month: 6, day: 22, hour: 25.0,
            temperature: 25.0, pressure: 1013.0, humidity: 50.0,
        }),
        ("负表高", VirtualExperienceRequest {
            gauge_height_chi: -10.0, latitude: 34.49, month: 6, day: 22, hour: 12.0,
            temperature: 25.0, pressure: 1013.0, humidity: 50.0,
        }),
        ("南半球", VirtualExperienceRequest {
            gauge_height_chi: 8.0, latitude: -34.49, month: 6, day: 22, hour: 12.0,
            temperature: 10.0, pressure: 1013.0, humidity: 50.0,
        }),
        ("北极", VirtualExperienceRequest {
            gauge_height_chi: 8.0, latitude: 85.0, month: 6, day: 22, hour: 12.0,
            temperature: 0.0, pressure: 1013.0, humidity: 80.0,
        }),
    ];

    for (desc, req) in invalid_cases {
        let result = VirtualExperienceSimulator::simulate(&req);
        assert!(!result.sun_altitude.is_nan(), "{}: 太阳高度角不应为NaN", desc);
        assert!(!result.sun_declination.is_nan(), "{}: 赤纬不应为NaN", desc);
        assert!(!result.theoretical_shadow_chi.is_nan(), "{}: 影长不应为NaN", desc);
        assert!(!result.dynasty_hint.is_empty(), "{}: 朝代提示不应为空", desc);
        assert!(!result.historical_note.is_empty(), "{}: 历史说明不应为空", desc);
    }
}

// =========================================================================
// 集成测试：与现有模块的兼容性
// =========================================================================

#[tokio::test]
async fn test_dtu_receiver_still_works_with_new_features() {
    let config = DtuValidationConfig::default();
    let dtu = DtuReceiver::new(config);

    let valid_measurement = SensorMeasurement {
        id: Uuid::new_v4(),
        station_id: "dengfeng_001".to_string(),
        station_name: "登封观星台".to_string(),
        measurement_time: chrono::Utc::now(),
        gauge_height: 40.0,
        shadow_length: 78.5,
        sun_altitude: 26.0,
        sun_azimuth: 180.0,
        atmospheric_refraction: 0.00029,
        temperature: 5.0,
        pressure: 1013.25,
        humidity: 50.0,
        is_solstice: 0,
    };

    assert!(dtu.validate(&valid_measurement).is_ok(),
        "DTU接收器应继续接受有效测量数据");

    let dynasty_req = DynastyComparisonRequest {
        sun_altitude: 26.0,
        temperature: 5.0,
        pressure: 1013.25,
        humidity: 50.0,
    };
    let dynasty_results = DynastyComparator::compare(&dynasty_req);
    assert_eq!(dynasty_results.len(), 3,
        "朝代对比功能应独立于DTU正常工作");

    let virtual_req = VirtualExperienceRequest {
        gauge_height_chi: 40.0,
        latitude: 34.49,
        month: 12,
        day: 22,
        hour: 12.0,
        temperature: 5.0,
        pressure: 1013.25,
        humidity: 50.0,
    };
    let virtual_result = VirtualExperienceSimulator::simulate(&virtual_req);
    assert!(virtual_result.is_daytime,
        "虚拟体验功能应独立正常工作");

    let pinhole_req = PinholeRequest {
        gauge_height_chi: 40.0,
        pinhole_diameter_cun: 1.0,
        sun_altitude: 26.0,
        screen_distance_chi: 40.0,
        temperature: 5.0,
        pressure: 1013.25,
    };
    let pinhole_result = PinholeSimulator::simulate(&pinhole_req);
    assert!(pinhole_result.total_blur_cun > 0.0,
        "针孔成像功能应独立正常工作");

    let meridian_req = MeridianComparisonRequest {
        sun_altitude: 26.0,
        temperature: 5.0,
        pressure: 1013.25,
    };
    let meridian_results = MeridianComparator::compare(&meridian_req);
    assert_eq!(meridian_results.len(), 3,
        "跨时代对比功能应独立正常工作");
}

#[tokio::test]
async fn test_all_features_use_same_optical_model() {
    let altitude = 26.0;
    let temperature = 5.0;
    let pressure = 1013.25;
    let gauge_height = 40.0;

    use guibiao_backend::optics::OpticalSimulator;
    let sim = OpticalSimulator::new(34.49, 113.0875, 420.0);
    let refraction = sim.calculate_refraction_arcsec(altitude, temperature, pressure);
    let true_alt = altitude - refraction / 3600.0;
    let expected_shadow = gauge_height / (true_alt * std::f64::consts::PI / 180.0).tan();

    let dynasty_req = DynastyComparisonRequest {
        sun_altitude: altitude, temperature, pressure, humidity: 50.0,
    };
    let dynasty_results = DynastyComparator::compare(&dynasty_req);
    let dynasty_shadow = dynasty_results[2].theoretical_shadow_chi;

    assert!((dynasty_shadow - expected_shadow).abs() < 0.01,
        "朝代对比模块应使用相同光学模型: 期望={}, 实际={}", expected_shadow, dynasty_shadow);

    let virtual_req = VirtualExperienceRequest {
        gauge_height_chi: gauge_height,
        latitude: 34.49,
        month: 12,
        day: 22,
        hour: 12.0,
        temperature,
        pressure,
        humidity: 50.0,
    };
    let virtual_result = VirtualExperienceSimulator::simulate(&virtual_req);
    assert!((virtual_result.refraction_correction_arcsec - refraction).abs() < 0.1,
        "虚拟体验模块应使用相同蒙气差模型: 期望={}, 实际={}",
        refraction, virtual_result.refraction_correction_arcsec);

    let meridian_req = MeridianComparisonRequest {
        sun_altitude: altitude, temperature, pressure,
    };
    let meridian_results = MeridianComparator::compare(&meridian_req);
    assert!((meridian_results[0].refraction_correction_arcsec - refraction).abs() < 0.1,
        "跨时代对比模块应使用相同蒙气差模型");
}

#[tokio::test]
async fn test_api_response_models_consistency() {
    use serde_json;

    let dynasty_req = DynastyComparisonRequest {
        sun_altitude: 26.0,
        temperature: 5.0,
        pressure: 1013.25,
        humidity: 50.0,
    };
    let results = DynastyComparator::compare(&dynasty_req);
    let json = serde_json::to_string(&results).expect("序列化失败");
    let deserialized: Vec<DynastyComparisonResult> = serde_json::from_str(&json).expect("反序列化失败");
    assert_eq!(results.len(), deserialized.len());
    assert_eq!(results[0].dynasty_id, deserialized[0].dynasty_id);
    assert!((results[0].theoretical_shadow_chi - deserialized[0].theoretical_shadow_chi).abs() < 1e-10);

    let pinhole_req = PinholeRequest {
        gauge_height_chi: 40.0,
        pinhole_diameter_cun: 1.0,
        sun_altitude: 26.0,
        screen_distance_chi: 40.0,
        temperature: 5.0,
        pressure: 1013.25,
    };
    let pinhole_result = PinholeSimulator::simulate(&pinhole_req);
    let json = serde_json::to_string(&pinhole_result).expect("序列化失败");
    let deserialized: PinholeResult = serde_json::from_str(&json).expect("反序列化失败");
    assert!((pinhole_result.total_blur_cun - deserialized.total_blur_cun).abs() < 1e-10);
    assert!((pinhole_result.optimal_diameter_cun - deserialized.optimal_diameter_cun).abs() < 1e-10);

    let virtual_req = VirtualExperienceRequest {
        gauge_height_chi: 40.0,
        latitude: 34.49,
        month: 12,
        day: 22,
        hour: 12.0,
        temperature: 5.0,
        pressure: 1013.0,
        humidity: 50.0,
    };
    let virtual_result = VirtualExperienceSimulator::simulate(&virtual_req);
    let json = serde_json::to_string(&virtual_result).expect("序列化失败");
    let deserialized: VirtualExperienceResult = serde_json::from_str(&json).expect("反序列化失败");
    assert_eq!(virtual_result.dynasty_hint, deserialized.dynasty_hint);
    assert_eq!(virtual_result.is_daytime, deserialized.is_daytime);
}

#[tokio::test]
async fn test_stress_large_number_of_calls() {
    let mut total_shadows = 0.0;
    let iterations = 100;

    for i in 0..iterations {
        let req = DynastyComparisonRequest {
            sun_altitude: 20.0 + (i as f64) * 0.1,
            temperature: 10.0,
            pressure: 1013.25,
            humidity: 50.0,
        };
        let results = DynastyComparator::compare(&req);
        total_shadows += results[2].theoretical_shadow_chi;
    }

    assert!(total_shadows > 0.0);
    assert!(!total_shadows.is_nan());
    assert!(total_shadows.is_finite());
}
