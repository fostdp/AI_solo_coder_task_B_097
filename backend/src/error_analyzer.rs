use std::sync::Arc;
use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};

use crate::models::{MonteCarloConfig, MonteCarloResult, SensorMeasurement, Station};
use crate::monte_carlo::MonteCarloAnalyzer;
use crate::optics::OpticalSimulator;
use crate::storage::SharedStore;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalyzerConfig {
    pub default_simulation_count: u32,
    pub default_gauge_height_error_std: f64,
    pub default_refraction_error_std: f64,
    pub default_confidence_level: f64,
    pub expanded_uncertainty_k: f64,
    pub spawn_blocking_threshold: u32,
}

impl Default for AnalyzerConfig {
    fn default() -> Self {
        Self {
            default_simulation_count: 10000,
            default_gauge_height_error_std: 0.01,
            default_refraction_error_std: 5.0,
            default_confidence_level: 0.95,
            expanded_uncertainty_k: 2.0,
            spawn_blocking_threshold: 20000,
        }
    }
}

impl AnalyzerConfig {
    pub fn from_json_file(path: &std::path::Path) -> Result<Self> {
        if path.exists() {
            let text = std::fs::read_to_string(path)?;
            let val: serde_json::Value = serde_json::from_str(&text)?;
            if let Some(mc) = val.get("monte_carlo") {
                Ok(Self {
                    default_simulation_count: mc
                        .get("default_simulation_count")
                        .and_then(|v| v.as_u64())
                        .unwrap_or(10000) as u32,
                    default_gauge_height_error_std: mc
                        .get("default_gauge_height_error_std_chi")
                        .and_then(|v| v.as_f64())
                        .unwrap_or(0.01),
                    default_refraction_error_std: mc
                        .get("default_refraction_error_std_arcsec")
                        .and_then(|v| v.as_f64())
                        .unwrap_or(5.0),
                    default_confidence_level: mc
                        .get("default_confidence_level")
                        .and_then(|v| v.as_f64())
                        .unwrap_or(0.95),
                    expanded_uncertainty_k: mc
                        .get("expanded_uncertainty_k_factor")
                        .and_then(|v| v.as_f64())
                        .unwrap_or(2.0),
                    spawn_blocking_threshold: 20000,
                })
            } else {
                Ok(Self::default())
            }
        } else {
            Ok(Self::default())
        }
    }

    pub fn apply_defaults(&self, config: &mut MonteCarloConfig) {
        if config.simulation_count == 0 {
            config.simulation_count = self.default_simulation_count;
        }
        if config.gauge_height_error_std <= 0.0 {
            config.gauge_height_error_std = self.default_gauge_height_error_std;
        }
        if config.refraction_error_std <= 0.0 {
            config.refraction_error_std = self.default_refraction_error_std;
        }
        if config.confidence_level <= 0.0 || config.confidence_level >= 1.0 {
            config.confidence_level = self.default_confidence_level;
        }
    }
}

pub struct ErrorAnalyzerService {
    store: SharedStore,
    analyzer_config: AnalyzerConfig,
}

impl ErrorAnalyzerService {
    pub fn new(store: SharedStore, analyzer_config: AnalyzerConfig) -> Self {
        Self { store, analyzer_config }
    }

    pub fn config(&self) -> &AnalyzerConfig {
        &self.analyzer_config
    }

    pub async fn analyze_with_station(
        &self,
        measurement: &SensorMeasurement,
        station: &Station,
        mut config: MonteCarloConfig,
    ) -> Result<MonteCarloResult> {
        self.analyzer_config.apply_defaults(&mut config);

        if config.simulation_count == 0 {
            return Err(anyhow!("simulation_count 必须为正数"));
        }
        if config.gauge_height_error_std <= 0.0 || config.refraction_error_std <= 0.0 {
            return Err(anyhow!("误差标准差必须为正数"));
        }
        if config.confidence_level <= 0.0 || config.confidence_level >= 1.0 {
            return Err(anyhow!("置信水平必须在 (0, 1) 之间"));
        }

        let simulator = OpticalSimulator::new(
            station.latitude,
            station.longitude,
            station.altitude,
        );
        let analyzer = MonteCarloAnalyzer::new(simulator);

        let count = config.simulation_count;
        let threshold = self.analyzer_config.spawn_blocking_threshold;
        let result = if count > threshold {
            let measurement = measurement.clone();
            let config = config.clone();
            let (tx, rx) = tokio::sync::oneshot::channel();
            std::thread::spawn(move || {
                let r = analyzer.analyze(&measurement, &config);
                let _ = tx.send(r);
            });
            rx.await.map_err(|e| anyhow!("分析线程异常退出: {}", e))?
        } else {
            analyzer.analyze(measurement, &config)
        };

        if let Err(e) = self.store.insert_monte_carlo(&result).await {
            tracing::error!("蒙特卡洛结果入库失败: station={} err={}", measurement.station_id, e);
        }

        Ok(result)
    }

    pub async fn analyze_from_store(
        &self,
        config: MonteCarloConfig,
    ) -> Result<MonteCarloResult> {
        let latest = self.store
            .get_latest_measurements(1)
            .await
            .map_err(|e| anyhow!("获取最新测量失败: {}", e))?;
        let measurement = latest
            .into_iter()
            .next()
            .ok_or_else(|| anyhow!("暂无测量数据可供分析"))?;

        let station = self.store
            .get_station(&measurement.station_id)
            .await
            .map_err(|e| anyhow!("查询台站失败: {}", e))?
            .ok_or_else(|| anyhow!("台站不存在: {}", measurement.station_id))?;

        self.analyze_with_station(&measurement, &station, config).await
    }

    pub fn uncertainty_budget(
        &self,
        measurement: &SensorMeasurement,
        config: &MonteCarloConfig,
        station: &Station,
    ) -> Vec<(String, f64, f64)> {
        let simulator = OpticalSimulator::new(
            station.latitude,
            station.longitude,
            station.altitude,
        );
        let analyzer = MonteCarloAnalyzer::new(simulator);
        analyzer.uncertainty_budget(measurement, config)
    }
}

pub type SharedErrorAnalyzer = Arc<ErrorAnalyzerService>;
