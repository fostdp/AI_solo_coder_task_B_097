use std::sync::Arc;
use anyhow::{anyhow, Result};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;
use uuid::Uuid;

use crate::models::{SensorMeasurement, Station};
use crate::storage::SharedStore;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DtuValidationConfig {
    pub gauge_height_min_chi: f64,
    pub gauge_height_max_chi: f64,
    pub shadow_length_min_chi: f64,
    pub shadow_length_max_chi: f64,
    pub altitude_min_deg: f64,
    pub altitude_max_deg: f64,
    pub azimuth_min_deg: f64,
    pub azimuth_max_deg: f64,
    pub temperature_min_c: f64,
    pub temperature_max_c: f64,
    pub pressure_min_hpa: f64,
    pub pressure_max_hpa: f64,
    pub humidity_min_pct: f64,
    pub humidity_max_pct: f64,
    pub max_clock_skew_seconds: i64,
}

impl Default for DtuValidationConfig {
    fn default() -> Self {
        Self {
            gauge_height_min_chi: 10.0,
            gauge_height_max_chi: 60.0,
            shadow_length_min_chi: 0.0,
            shadow_length_max_chi: 500.0,
            altitude_min_deg: -5.0,
            altitude_max_deg: 90.0,
            azimuth_min_deg: 0.0,
            azimuth_max_deg: 360.0,
            temperature_min_c: -30.0,
            temperature_max_c: 50.0,
            pressure_min_hpa: 800.0,
            pressure_max_hpa: 1080.0,
            humidity_min_pct: 0.0,
            humidity_max_pct: 100.0,
            max_clock_skew_seconds: 3600,
        }
    }
}

pub type DtuToSimulatorTx = mpsc::Sender<(SensorMeasurement, Station)>;
pub type DtuToSimulatorRx = mpsc::Receiver<(SensorMeasurement, Station)>;

pub struct DtuReceiver {
    store: SharedStore,
    validation: DtuValidationConfig,
    simulator_tx: DtuToSimulatorTx,
}

impl DtuReceiver {
    pub fn new(
        store: SharedStore,
        validation: DtuValidationConfig,
        simulator_tx: DtuToSimulatorTx,
    ) -> Self {
        Self { store, validation, simulator_tx }
    }

    pub fn validate(&self, m: &SensorMeasurement) -> Result<()> {
        if m.station_id.trim().is_empty() {
            return Err(anyhow!("station_id 不能为空"));
        }
        if m.gauge_height < self.validation.gauge_height_min_chi
            || m.gauge_height > self.validation.gauge_height_max_chi {
            return Err(anyhow!(
                "表高 {:.2} 尺超出范围 [{:.1}, {:.1}]",
                m.gauge_height,
                self.validation.gauge_height_min_chi,
                self.validation.gauge_height_max_chi
            ));
        }
        if m.shadow_length < self.validation.shadow_length_min_chi
            || m.shadow_length > self.validation.shadow_length_max_chi {
            return Err(anyhow!(
                "影长 {:.2} 尺超出范围 [{:.1}, {:.1}]",
                m.shadow_length,
                self.validation.shadow_length_min_chi,
                self.validation.shadow_length_max_chi
            ));
        }
        if m.sun_altitude < self.validation.altitude_min_deg
            || m.sun_altitude > self.validation.altitude_max_deg {
            return Err(anyhow!(
                "太阳高度角 {:.2}° 超出范围 [{:.1}, {:.1}]",
                m.sun_altitude,
                self.validation.altitude_min_deg,
                self.validation.altitude_max_deg
            ));
        }
        if m.sun_azimuth < self.validation.azimuth_min_deg
            || m.sun_azimuth > self.validation.azimuth_max_deg {
            return Err(anyhow!(
                "太阳方位角 {:.2}° 超出范围 [{:.1}, {:.1}]",
                m.sun_azimuth,
                self.validation.azimuth_min_deg,
                self.validation.azimuth_max_deg
            ));
        }
        if m.temperature < self.validation.temperature_min_c
            || m.temperature > self.validation.temperature_max_c {
            return Err(anyhow!(
                "气温 {:.1}°C 超出范围 [{:.1}, {:.1}]",
                m.temperature,
                self.validation.temperature_min_c,
                self.validation.temperature_max_c
            ));
        }
        if m.pressure < self.validation.pressure_min_hpa
            || m.pressure > self.validation.pressure_max_hpa {
            return Err(anyhow!(
                "气压 {:.0}hPa 超出范围 [{:.0}, {:.0}]",
                m.pressure,
                self.validation.pressure_min_hpa,
                self.validation.pressure_max_hpa
            ));
        }
        if m.humidity < self.validation.humidity_min_pct
            || m.humidity > self.validation.humidity_max_pct {
            return Err(anyhow!(
                "湿度 {:.1}% 超出范围 [{:.0}, {:.0}]",
                m.humidity,
                self.validation.humidity_min_pct,
                self.validation.humidity_max_pct
            ));
        }
        let now = Utc::now();
        let diff = (now - m.measurement_time).num_seconds().abs();
        if diff > self.validation.max_clock_skew_seconds {
            tracing::warn!(
                "DTU时钟偏差过大: station={}, diff={}s (阈值 {}s)",
                m.station_id,
                diff,
                self.validation.max_clock_skew_seconds
            );
        }
        Ok(())
    }

    pub async fn ingest(
        &self,
        mut measurement: SensorMeasurement,
    ) -> Result<(SensorMeasurement, Station)> {
        if measurement.id.is_nil() {
            measurement.id = Uuid::new_v4();
        }

        self.validate(&measurement)
            .map_err(|e| anyhow!("DTU校验失败: {}", e))?;

        let station = self.store.get_station(&measurement.station_id).await
            .map_err(|e| anyhow!("查询台站失败: {}", e))?
            .ok_or_else(|| anyhow!("台站不存在: station_id={}", measurement.station_id))?;

        if let Err(e) = self.store.insert_measurement(&measurement).await {
            tracing::error!("入库测量数据失败: station={} err={}", measurement.station_id, e);
        }

        if let Err(e) = self.simulator_tx.send((measurement.clone(), station.clone())).await {
            tracing::error!(
                "DTU->Simulator mpsc通道失败 (下游关闭): station={} err={}",
                measurement.station_id,
                e
            );
            return Err(anyhow!("光学仿真模块无响应: {}", e));
        }

        Ok((measurement, station))
    }
}

pub fn new_dtu_channel(buffer: usize) -> (DtuToSimulatorTx, DtuToSimulatorRx) {
    mpsc::channel(buffer)
}

impl DtuValidationConfig {
    pub fn from_json_file(path: &std::path::Path) -> Result<Self> {
        if path.exists() {
            let text = std::fs::read_to_string(path)?;
            let val: serde_json::Value = serde_json::from_str(&text)?;
            if let Some(dtuv) = val.get("dtu_validation") {
                Ok(serde_json::from_value(dtuv.clone())?)
            } else {
                Ok(Self::default())
            }
        } else {
            Ok(Self::default())
        }
    }
}
