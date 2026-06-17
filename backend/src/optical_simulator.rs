use std::sync::Arc;
use anyhow::Result;
use tokio::sync::mpsc;

use crate::dtu_receiver::{DtuToSimulatorRx, DtuToSimulatorTx};
use crate::models::{OpticalSimulationResult, SensorMeasurement, Station};
use crate::optics::OpticalSimulator;
use crate::storage::SharedStore;

pub type SimToAlarmTx = mpsc::Sender<(SensorMeasurement, OpticalSimulationResult)>;
pub type SimToAlarmRx = mpsc::Receiver<(SensorMeasurement, OpticalSimulationResult)>;

pub fn new_simulator_channel(buffer: usize) -> (SimToAlarmTx, SimToAlarmRx) {
    mpsc::channel(buffer)
}

pub fn new_dtu_to_sim_channel(buffer: usize) -> (DtuToSimulatorTx, DtuToSimulatorRx) {
    mpsc::channel(buffer)
}

pub struct OpticalSimulatorService {
    store: SharedStore,
    inner: OpticalSimulator,
}

impl OpticalSimulatorService {
    pub fn new(store: SharedStore, station: &Station) -> Self {
        Self {
            store,
            inner: OpticalSimulator::new(
                station.latitude,
                station.longitude,
                station.altitude,
            ),
        }
    }

    pub fn from_coords(store: SharedStore, lat: f64, lon: f64, alt: f64) -> Self {
        Self {
            store,
            inner: OpticalSimulator::new(lat, lon, alt),
        }
    }

    pub fn simulator(&self) -> &OpticalSimulator {
        &self.inner
    }

    pub fn run_simulation(
        &self,
        measurement: &SensorMeasurement,
    ) -> OpticalSimulationResult {
        self.inner.simulate_optics(
            measurement.id,
            &measurement.station_id,
            measurement.gauge_height,
            measurement.shadow_length,
            measurement.sun_altitude,
            measurement.temperature,
            measurement.pressure,
            measurement.measurement_time,
        )
    }
}

pub async fn run_simulator_loop(
    store: SharedStore,
    mut dtu_rx: DtuToSimulatorRx,
    alarm_tx: SimToAlarmTx,
) {
    tracing::info!("optical_simulator 事件循环已启动");

    let mut station_cache: std::collections::HashMap<String, Arc<OpticalSimulatorService>> =
        std::collections::HashMap::new();

    while let Some((measurement, station)) = dtu_rx.recv().await {
        let service = station_cache
            .entry(station.station_id.clone())
            .or_insert_with(|| {
                Arc::new(OpticalSimulatorService::new(store.clone(), &station))
            })
            .clone();

        let sim_result = service.run_simulation(&measurement);

        if let Err(e) = service.store.insert_simulation(&sim_result).await {
            tracing::error!(
                "光学仿真入库失败: station={} err={}",
                measurement.station_id,
                e
            );
        }

        if let Err(e) = alarm_tx.send((measurement.clone(), sim_result.clone())).await {
            tracing::error!(
                "Simulator->Alarm mpsc通道失败: station={} err={}",
                measurement.station_id,
                e
            );
        }
    }

    tracing::warn!("optical_simulator 事件循环已退出 (上游DTU通道关闭)");
}

#[derive(Debug, Clone, Default)]
pub struct SimulatorConfig {
    pub channel_buffer: usize,
}

impl SimulatorConfig {
    pub fn from_json_file(path: &std::path::Path) -> Result<Self> {
        if path.exists() {
            let text = std::fs::read_to_string(path)?;
            let val: serde_json::Value = serde_json::from_str(&text)?;
            let buffer = val
                .get("service")
                .and_then(|v| v.get("simulator_channel_buffer"))
                .and_then(|v| v.as_u64())
                .unwrap_or(256) as usize;
            Ok(Self { channel_buffer: buffer })
        } else {
            Ok(Self { channel_buffer: 256 })
        }
    }
}
