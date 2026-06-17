use std::collections::HashMap;
use std::sync::Arc;
use anyhow::Result;
use axum::{
    extract::{ws::{Message, WebSocket, WebSocketUpgrade}},
    response::IntoResponse,
};
use chrono::{DateTime, Utc};
use futures::{sink::SinkExt, stream::StreamExt};
use serde::{Deserialize, Serialize};
use tokio::sync::{broadcast, mpsc, RwLock};
use uuid::Uuid;

use crate::models::{AlertEvent, OpticalSimulationResult, SensorMeasurement, WsMessage};
use crate::optical_simulator::SimToAlarmRx;
use crate::storage::SharedStore;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlarmConfig {
    pub deviation_threshold_cun: f64,
    pub warning_high_multiple: f64,
    pub critical_multiple: f64,
    pub debounce_seconds: i64,
    pub ws_channel_capacity: usize,
    pub channel_buffer: usize,
}

impl Default for AlarmConfig {
    fn default() -> Self {
        Self {
            deviation_threshold_cun: 1.0,
            warning_high_multiple: 2.0,
            critical_multiple: 3.0,
            debounce_seconds: 60,
            ws_channel_capacity: 1024,
            channel_buffer: 256,
        }
    }
}

impl AlarmConfig {
    pub fn from_json_file(path: &std::path::Path) -> Result<Self> {
        if path.exists() {
            let text = std::fs::read_to_string(path)?;
            let val: serde_json::Value = serde_json::from_str(&text)?;
            if let Some(a) = val.get("alarm") {
                Ok(Self {
                    deviation_threshold_cun: a
                        .get("shadow_deviation_threshold_cun")
                        .and_then(|v| v.as_f64())
                        .unwrap_or(1.0),
                    warning_high_multiple: a
                        .get("level_warning_high_multiple")
                        .and_then(|v| v.as_f64())
                        .unwrap_or(2.0),
                    critical_multiple: a
                        .get("level_critical_multiple")
                        .and_then(|v| v.as_f64())
                        .unwrap_or(3.0),
                    debounce_seconds: a
                        .get("debounce_seconds")
                        .and_then(|v| v.as_u64())
                        .unwrap_or(60) as i64,
                    ws_channel_capacity: a
                        .get("ws_channel_capacity")
                        .and_then(|v| v.as_u64())
                        .unwrap_or(1024) as usize,
                    channel_buffer: 256,
                })
            } else {
                Ok(Self::default())
            }
        } else {
            Ok(Self::default())
        }
    }
}

#[derive(Clone)]
pub struct AlarmWsState {
    pub store: SharedStore,
    pub broadcast: broadcast::Sender<WsMessage>,
    pub alarm_config: AlarmConfig,
    pub last_alert: Arc<RwLock<HashMap<String, DateTime<Utc>>>>,
}

impl AlarmWsState {
    pub fn new(store: SharedStore, alarm_config: AlarmConfig) -> Self {
        let (tx, _) = broadcast::channel(alarm_config.ws_channel_capacity);
        Self {
            store,
            broadcast: tx,
            alarm_config,
            last_alert: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn broadcast_message(&self, msg: WsMessage) {
        let _ = self.broadcast.send(msg);
    }

    pub fn classify_level(&self, deviation_cun: f64) -> &'static str {
        let t = self.alarm_config.deviation_threshold_cun;
        if deviation_cun >= self.alarm_config.critical_multiple * t {
            "CRITICAL"
        } else if deviation_cun >= self.alarm_config.warning_high_multiple * t {
            "WARNING"
        } else {
            "WARNING"
        }
    }

    pub async fn evaluate(
        &self,
        measurement: &SensorMeasurement,
        expected_shadow: f64,
    ) -> Option<AlertEvent> {
        let t = self.alarm_config.deviation_threshold_cun;
        let deviation_cun = (measurement.shadow_length - expected_shadow).abs() * 10.0;
        if deviation_cun < t {
            return None;
        }

        {
            let last_alerts = self.last_alert.read().await;
            if let Some(last) = last_alerts.get(&measurement.station_id) {
                if (Utc::now() - *last).num_seconds() < self.alarm_config.debounce_seconds {
                    return None;
                }
            }
        }

        let mut last_alerts = self.last_alert.write().await;
        let now = Utc::now();
        if let Some(last) = last_alerts.get(&measurement.station_id) {
            if (now - *last).num_seconds() < self.alarm_config.debounce_seconds {
                return None;
            }
        }
        last_alerts.insert(measurement.station_id.clone(), now);

        let level = self.classify_level(deviation_cun).to_string();
        let alert = AlertEvent {
            id: Uuid::new_v4(),
            station_id: measurement.station_id.clone(),
            alert_time: now,
            alert_type: "SHADOW_DEVIATION".to_string(),
            alert_level: level,
            measured_shadow_length: measurement.shadow_length,
            expected_shadow_length: expected_shadow,
            deviation_cun,
            threshold_cun: t,
            message: format!(
                "影长偏差超过阈值: 测量 {:.2}尺, 预期 {:.2}尺, 偏差 {:.2}寸 (阈值 {:.2}寸)",
                measurement.shadow_length, expected_shadow, deviation_cun, t
            ),
            is_acknowledged: 0,
        };
        Some(alert)
    }
}

pub type AlarmBroadcastState = AlarmWsState;

pub async fn run_alarm_loop(
    state: AlarmWsState,
    mut sim_rx: SimToAlarmRx,
) {
    tracing::info!("alarm_ws 事件循环已启动 (阈值 {} 寸)", state.alarm_config.deviation_threshold_cun);

    while let Some((measurement, simulation)) = sim_rx.recv().await {
        if let Some(alert) = state.evaluate(&measurement, simulation.refracted_shadow_length).await {
            if let Err(e) = state.store.insert_alert(&alert).await {
                tracing::error!(
                    "告警入库失败: station={} err={}",
                    measurement.station_id,
                    e
                );
            }

            let ws_alert = WsMessage::alert(&alert);
            state.broadcast_message(ws_alert).await;
        }

        let ws_meas = WsMessage::measurement(&measurement);
        state.broadcast_message(ws_meas).await;

        let ws_sim = WsMessage::simulation(&simulation);
        state.broadcast_message(ws_sim).await;
    }

    tracing::warn!("alarm_ws 事件循环已退出 (上游Simulator通道关闭)");
}

pub async fn ws_handler(
    ws: WebSocketUpgrade,
    state: AlarmWsState,
) -> impl IntoResponse {
    ws.on_upgrade(|socket| handle_socket(socket, state))
}

async fn handle_socket(socket: WebSocket, state: AlarmWsState) {
    let (mut sender, mut receiver) = socket.split();
    let mut rx = state.broadcast.subscribe();

    let send_task = tokio::spawn(async move {
        while let Ok(msg) = rx.recv().await {
            if let Ok(text) = serde_json::to_string(&msg) {
                if sender.send(Message::Text(text)).await.is_err() {
                    break;
                }
            }
        }
        let _ = sender.close().await;
    });

    let recv_task = tokio::spawn(async move {
        while let Some(Ok(msg)) = receiver.next().await {
            match msg {
                Message::Text(text) => {
                    tracing::debug!("收到WS消息: {}", text);
                }
                Message::Ping(_) | Message::Pong(_) | Message::Binary(_) => {}
                Message::Close(_) => {
                    break;
                }
            }
        }
    });

    tokio::select! {
        _ = send_task => {}
        _ = recv_task => {}
    }
}

pub fn new_sim_alarm_channel(buffer: usize) -> (mpsc::Sender<(SensorMeasurement, OpticalSimulationResult)>, SimToAlarmRx) {
    mpsc::channel(buffer)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::ClickHouseStore;

    #[tokio::test]
    async fn test_alarm_classify_level() {
        let store = std::sync::Arc::new(ClickHouseStore::new("http://localhost:8123", "guibiao"));
        let cfg = AlarmConfig::default();
        let state = AlarmWsState::new(store, cfg);
        assert_eq!(state.classify_level(0.5), "WARNING");
        assert_eq!(state.classify_level(1.5), "WARNING");
        assert_eq!(state.classify_level(3.5), "CRITICAL");
    }

    #[tokio::test]
    async fn test_alarm_debounce() {
        let store = std::sync::Arc::new(ClickHouseStore::new("http://localhost:8123", "guibiao"));
        let cfg = AlarmConfig {
            debounce_seconds: 3600,
            ..Default::default()
        };
        let state = AlarmWsState::new(store, cfg);
        let m = SensorMeasurement {
            id: Uuid::new_v4(),
            station_id: "debounce_test".to_string(),
            station_name: "T".to_string(),
            measurement_time: Utc::now(),
            gauge_height: 40.0,
            shadow_length: 90.0,
            sun_altitude: 20.0,
            sun_azimuth: 180.0,
            atmospheric_refraction: 1.00029,
            temperature: 10.0,
            pressure: 1013.25,
            humidity: 50.0,
            is_solstice: 0,
        };
        let a1 = state.evaluate(&m, 89.0).await;
        assert!(a1.is_some());
        let a2 = state.evaluate(&m, 89.0).await;
        assert!(a2.is_none(), "去抖应阻止相同站秒级重复告警");
    }
}
