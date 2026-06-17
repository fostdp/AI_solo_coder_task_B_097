use std::sync::Arc;
use tokio::sync::Semaphore;
use tokio::task::JoinHandle;
use uuid::Uuid;

use crate::constants::*;
use crate::models::{MonteCarloConfig, MonteCarloResult, SensorMeasurement};
use crate::monte_carlo::MonteCarloAnalyzer;
use crate::optics::OpticalSimulator;

#[derive(Clone)]
pub struct MonteCarloThreadPool {
    semaphore: Arc<Semaphore>,
    analyzer: Arc<MonteCarloAnalyzer>,
    pool_size: usize,
}

impl MonteCarloThreadPool {
    pub fn new(pool_size: usize) -> Self {
        let size = pool_size.max(1);
        let sim = OpticalSimulator::new(
            DEFAULT_STATION_LAT,
            DEFAULT_STATION_LON,
            DEFAULT_STATION_ALT,
        );
        let analyzer = MonteCarloAnalyzer::new(sim);

        Self {
            semaphore: Arc::new(Semaphore::new(size)),
            analyzer: Arc::new(analyzer),
            pool_size: size,
        }
    }

    pub fn with_default_config() -> Self {
        Self::new(MONTE_CARLO_THREAD_POOL_SIZE)
    }

    pub fn pool_size(&self) -> usize {
        self.pool_size
    }

    pub async fn analyze_single(
        &self,
        measurement: SensorMeasurement,
        config: MonteCarloConfig,
    ) -> Result<MonteCarloResult, String> {
        let permit = self.semaphore
            .clone()
            .acquire_owned()
            .await
            .map_err(|e| format!("信号量获取失败: {}", e))?;

        let analyzer = self.analyzer.clone();
        let result = tokio::task::spawn_blocking(move || {
            let _permit = permit;
            analyzer.analyze(&measurement, &config)
        }).await.map_err(|e| format!("任务执行失败: {}", e))?;

        Ok(result)
    }

    pub async fn analyze_batch(
        &self,
        measurements: Vec<SensorMeasurement>,
        config: MonteCarloConfig,
    ) -> Vec<Result<MonteCarloResult, String>> {
        let mut handles: Vec<JoinHandle<(Uuid, Result<MonteCarloResult, String>)>> = Vec::new();

        for measurement in measurements {
            let permit = self.semaphore
                .clone()
                .acquire_owned()
                .await
                .expect("信号量不应被关闭");

            let analyzer = self.analyzer.clone();
            let config_clone = config.clone();
            let measurement_id = measurement.id;

            let handle = tokio::task::spawn_blocking(move || {
                let _permit = permit;
                (measurement_id, analyzer.analyze(&measurement, &config_clone))
            });

            handles.push(handle);
        }

        let mut results = Vec::with_capacity(handles.len());
        for handle in handles {
            match handle.await {
                Ok((_id, result)) => results.push(Ok(result)),
                Err(e) => results.push(Err(format!("任务失败: {}", e))),
            }
        }

        results
    }

    pub async fn analyze_batch_with_progress<F>(
        &self,
        measurements: Vec<SensorMeasurement>,
        config: MonteCarloConfig,
        mut progress_callback: F,
    ) -> Vec<Result<MonteCarloResult, String>>
    where
        F: FnMut(usize, usize) + Send + 'static,
    {
        use std::sync::Mutex;

        let total = measurements.len();
        let completed = Arc::new(Mutex::new(0usize));
        let mut handles: Vec<JoinHandle<(Uuid, Result<MonteCarloResult, String>)>> = Vec::new();

        for measurement in measurements {
            let permit = self.semaphore
                .clone()
                .acquire_owned()
                .await
                .expect("信号量不应被关闭");

            let analyzer = self.analyzer.clone();
            let config_clone = config.clone();
            let measurement_id = measurement.id;
            let completed_clone = completed.clone();

            let handle = tokio::task::spawn_blocking(move || {
                let _permit = permit;
                let result = analyzer.analyze(&measurement, &config_clone);

                let mut count = completed_clone.lock().unwrap();
                *count += 1;
                (measurement_id, result)
            });

            handles.push(handle);
        }

        let progress_handle = tokio::spawn(async move {
            loop {
                let current = *completed.lock().unwrap();
                progress_callback(current, total);
                if current >= total {
                    break;
                }
                tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
            }
        });

        let mut results = Vec::with_capacity(handles.len());
        for handle in handles {
            match handle.await {
                Ok((_id, result)) => results.push(Ok(result)),
                Err(e) => results.push(Err(format!("任务失败: {}", e))),
            }
        }

        let _ = progress_handle.await;

        results
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::MonteCarloConfig;
    use chrono::{TimeZone, Utc};

    fn create_test_measurement() -> SensorMeasurement {
        SensorMeasurement {
            id: Uuid::new_v4(),
            station_id: "test_001".to_string(),
            station_name: "Test Station".to_string(),
            measurement_time: Utc.with_ymd_and_hms(2023, 12, 22, 4, 30, 0).unwrap(),
            gauge_height: 40.0,
            shadow_length: 88.0,
            sun_altitude: 24.5,
            sun_azimuth: 180.0,
            atmospheric_refraction: 1.00029,
            temperature: 5.0,
            pressure: 1013.25,
            humidity: 50.0,
            is_solstice: 1,
        }
    }

    #[tokio::test]
    async fn test_thread_pool_creation() {
        let pool = MonteCarloThreadPool::new(2);
        assert_eq!(pool.pool_size(), 2);
    }

    #[tokio::test]
    async fn test_thread_pool_default_size() {
        let pool = MonteCarloThreadPool::with_default_config();
        assert_eq!(pool.pool_size(), MONTE_CARLO_THREAD_POOL_SIZE);
    }

    #[tokio::test]
    async fn test_analyze_single() {
        let pool = MonteCarloThreadPool::new(2);
        let measurement = create_test_measurement();
        let config = MonteCarloConfig {
            simulation_count: 100,
            ..Default::default()
        };

        let result = pool.analyze_single(measurement, config).await;
        assert!(result.is_ok());
        let mc_result = result.unwrap();
        assert_eq!(mc_result.simulation_count, 100);
        assert!(mc_result.shadow_length_std > 0.0);
    }

    #[tokio::test]
    async fn test_analyze_batch() {
        let pool = MonteCarloThreadPool::new(2);
        let measurements: Vec<SensorMeasurement> = (0..5)
            .map(|_| create_test_measurement())
            .collect();
        let config = MonteCarloConfig {
            simulation_count: 50,
            ..Default::default()
        };

        let results = pool.analyze_batch(measurements, config).await;
        assert_eq!(results.len(), 5);
        for result in results {
            assert!(result.is_ok());
            let mc_result = result.unwrap();
            assert_eq!(mc_result.simulation_count, 50);
        }
    }

    #[tokio::test]
    async fn test_thread_pool_concurrency_limit() {
        let pool = MonteCarloThreadPool::new(2);
        let measurements: Vec<SensorMeasurement> = (0..10)
            .map(|_| create_test_measurement())
            .collect();
        let config = MonteCarloConfig {
            simulation_count: 200,
            ..Default::default()
        };

        let results = pool.analyze_batch(measurements, config).await;
        assert_eq!(results.len(), 10);
        for result in results {
            assert!(result.is_ok());
        }
    }

    #[tokio::test]
    async fn test_analyze_batch_with_progress() {
        let pool = MonteCarloThreadPool::new(2);
        let measurements: Vec<SensorMeasurement> = (0..4)
            .map(|_| create_test_measurement())
            .collect();
        let config = MonteCarloConfig {
            simulation_count: 50,
            ..Default::default()
        };

        let progress_received = Arc::new(std::sync::Mutex::new(Vec::new()));
        let progress_clone = progress_received.clone();

        let results = pool.analyze_batch_with_progress(
            measurements,
            config,
            move |current, total| {
                progress_clone.lock().unwrap().push((current, total));
            },
        ).await;

        assert_eq!(results.len(), 4);
        let progress = progress_received.lock().unwrap();
        assert!(progress.len() > 0);
        assert_eq!(progress.last().unwrap(), &(4, 4));
    }
}
