-- ============================================================
-- 古代圭表测影光学仿真与冬至时刻精度分析系统
-- ClickHouse 数据库初始化脚本
-- ============================================================

CREATE DATABASE IF NOT EXISTS guibiao
    COMMENT '圭表测影仿真数据库'
    ENGINE = Atomic;

USE guibiao;

-- ============================================================
-- 1. 圭表传感器实时测量数据表
-- ============================================================
CREATE TABLE IF NOT EXISTS sensor_measurements (
    id UUID DEFAULT generateUUIDv4(),
    station_id String COMMENT '圭表站点ID',
    station_name String COMMENT '圭表站点名称',
    measurement_time DateTime64(3, 'Asia/Shanghai') COMMENT '测量时间（毫秒精度）',
    gauge_height Float64 COMMENT '表高（尺）',
    shadow_length Float64 COMMENT '影长（尺）',
    shadow_length_cun Float64 COMMENT '影长（寸）',
    sun_altitude Float64 COMMENT '太阳高度角（度）',
    sun_azimuth Float64 COMMENT '太阳方位角（度）',
    atmospheric_refraction Float64 COMMENT '大气折射率',
    temperature Float64 COMMENT '气温（摄氏度）',
    pressure Float64 COMMENT '气压（百帕）',
    humidity Float64 COMMENT '相对湿度（%）',
    is_solstice UInt8 DEFAULT 0 COMMENT '是否冬至时刻 0-否 1-是',
    created_at DateTime64(3, 'Asia/Shanghai') DEFAULT now64(3)
)
ENGINE = MergeTree()
PARTITION BY toYYYYMM(measurement_time)
ORDER BY (station_id, measurement_time)
TTL measurement_time + INTERVAL 90 DAY
COMMENT '圭表传感器每分钟测量数据表';

-- ============================================================
-- 2. 光学仿真计算结果表
-- ============================================================
CREATE TABLE IF NOT EXISTS optical_simulations (
    id UUID DEFAULT generateUUIDv4(),
    measurement_id UUID COMMENT '关联测量记录ID',
    station_id String COMMENT '圭表站点ID',
    simulation_time DateTime64(3, 'Asia/Shanghai') COMMENT '仿真计算时间',
    true_sun_altitude Float64 COMMENT '真实太阳高度角（度，考虑大气折射前）',
    apparent_sun_altitude Float64 COMMENT '视太阳高度角（度，考虑大气折射后）',
    atmospheric_refraction_correction Float64 COMMENT '大气折射修正量（角秒）',
    earth_curvature_correction Float64 COMMENT '地球曲率修正量（尺）',
    theoretical_shadow_length Float64 COMMENT '理论影长（尺，无折射）',
    refracted_shadow_length Float64 COMMENT '折射影长（尺，含蒙气差）',
    shadow_deviation Float64 COMMENT '影长偏差（寸）',
    winter_solstice_moment DateTime64(6, 'Asia/Shanghai') COMMENT '计算的冬至精确时刻',
    solstice_uncertainty Float64 COMMENT '冬至时刻不确定度（秒）',
    created_at DateTime64(3, 'Asia/Shanghai') DEFAULT now64(3)
)
ENGINE = MergeTree()
PARTITION BY toYYYYMM(simulation_time)
ORDER BY (station_id, simulation_time)
COMMENT '光学仿真计算结果表';

-- ============================================================
-- 3. 蒙特卡洛误差分析结果表
-- ============================================================
CREATE TABLE IF NOT EXISTS monte_carlo_analysis (
    id UUID DEFAULT generateUUIDv4(),
    station_id String COMMENT '圭表站点ID',
    analysis_time DateTime64(3, 'Asia/Shanghai') COMMENT '分析时间',
    reference_time DateTime64(3, 'Asia/Shanghai') COMMENT '分析参考时间点',
    simulation_count UInt32 COMMENT '蒙特卡洛模拟次数',
    gauge_height_error_mean Float64 COMMENT '表高误差均值（尺）',
    gauge_height_error_std Float64 COMMENT '表高误差标准差（尺）',
    refraction_error_mean Float64 COMMENT '蒙气差误差均值（角秒）',
    refraction_error_std Float64 COMMENT '蒙气差误差标准差（角秒）',
    shadow_length_mean Float64 COMMENT '影长分布均值（尺）',
    shadow_length_std Float64 COMMENT '影长分布标准差（尺）',
    shadow_length_95ci_low Float64 COMMENT '影长95%置信区间下限（尺）',
    shadow_length_95ci_high Float64 COMMENT '影长95%置信区间上限（尺）',
    solstice_time_mean Float64 COMMENT '冬至时刻均值偏移（秒）',
    solstice_time_std Float64 COMMENT '冬至时刻标准差（秒）',
    solstice_time_95ci_low Float64 COMMENT '冬至时刻95%置信区间下限（秒）',
    solstice_time_95ci_high Float64 COMMENT '冬至时刻95%置信区间上限（秒）',
    combined_uncertainty Float64 COMMENT '合成标准不确定度',
    expanded_uncertainty Float64 COMMENT '扩展不确定度（k=2）',
    created_at DateTime64(3, 'Asia/Shanghai') DEFAULT now64(3)
)
ENGINE = MergeTree()
PARTITION BY toYYYYMM(analysis_time)
ORDER BY (station_id, analysis_time)
COMMENT '蒙特卡洛误差分析结果表';

-- ============================================================
-- 4. 告警事件表
-- ============================================================
CREATE TABLE IF NOT EXISTS alert_events (
    id UUID DEFAULT generateUUIDv4(),
    station_id String COMMENT '圭表站点ID',
    alert_time DateTime64(3, 'Asia/Shanghai') COMMENT '告警时间',
    alert_type String COMMENT '告警类型: SHADOW_DEVIATION/DEVICE_FAULT/WEATHER',
    alert_level String COMMENT '告警级别: INFO/WARNING/CRITICAL',
    measured_shadow_length Float64 COMMENT '测量影长（尺）',
    expected_shadow_length Float64 COMMENT '预期影长（尺）',
    deviation_cun Float64 COMMENT '偏差（寸）',
    threshold_cun Float64 COMMENT '告警阈值（寸）',
    message String COMMENT '告警消息',
    is_acknowledged UInt8 DEFAULT 0 COMMENT '是否已确认',
    acknowledged_at Nullable(DateTime64(3, 'Asia/Shanghai')),
    created_at DateTime64(3, 'Asia/Shanghai') DEFAULT now64(3)
)
ENGINE = MergeTree()
PARTITION BY toYYYYMM(alert_time)
ORDER BY (station_id, alert_time, alert_level)
COMMENT '告警事件表';

-- ============================================================
-- 5. 站点元数据表
-- ============================================================
CREATE TABLE IF NOT EXISTS stations (
    station_id String COMMENT '站点ID',
    station_name String COMMENT '站点名称',
    latitude Float64 COMMENT '纬度（度）',
    longitude Float64 COMMENT '经度（度）',
    altitude Float64 COMMENT '海拔高度（米）',
    standard_gauge_height Float64 COMMENT '标准表高（尺）',
    location String COMMENT '位置描述',
    created_at DateTime64(3, 'Asia/Shanghai') DEFAULT now64(3),
    updated_at DateTime64(3, 'Asia/Shanghai') DEFAULT now64(3)
)
ENGINE = ReplacingMergeTree(updated_at)
ORDER BY station_id
PRIMARY KEY station_id
COMMENT '圭表站点元数据表';

-- ============================================================
-- 插入登封观星台默认站点数据
-- ============================================================
INSERT INTO stations (station_id, station_name, latitude, longitude, altitude, standard_gauge_height, location)
VALUES (
    'dengfeng_001',
    '登封观星台元代圭表',
    34.4897,
    113.0875,
    420.0,
    40.0,
    '河南省登封市告成镇，元代郭守敬所建'
);

-- ============================================================
-- 创建视图：最新测量数据
-- ============================================================
CREATE VIEW IF NOT EXISTS latest_measurements AS
SELECT
    sm.station_id,
    sm.station_name,
    sm.measurement_time,
    sm.gauge_height,
    sm.shadow_length,
    sm.shadow_length_cun,
    sm.sun_altitude,
    sm.sun_azimuth,
    sm.atmospheric_refraction,
    sm.temperature,
    sm.pressure,
    sm.humidity,
    sm.is_solstice
FROM sensor_measurements sm
INNER JOIN (
    SELECT station_id, max(measurement_time) as max_time
    FROM sensor_measurements
    GROUP BY station_id
) lm ON sm.station_id = lm.station_id AND sm.measurement_time = lm.max_time;

-- ============================================================
-- 创建视图：未处理告警
-- ============================================================
CREATE VIEW IF NOT EXISTS active_alerts AS
SELECT *
FROM alert_events
WHERE is_acknowledged = 0
ORDER BY alert_time DESC;

-- ============================================================
-- 创建聚合视图：小时统计
-- ============================================================
CREATE MATERIALIZED VIEW IF NOT EXISTS hourly_stats_mv
TO hourly_stats
AS
SELECT
    station_id,
    toStartOfHour(measurement_time) AS hour_start,
    count() AS measurement_count,
    avg(shadow_length) AS avg_shadow_length,
    min(shadow_length) AS min_shadow_length,
    max(shadow_length) AS max_shadow_length,
    avg(sun_altitude) AS avg_sun_altitude,
    max(sun_altitude) AS max_sun_altitude,
    avg(temperature) AS avg_temperature
FROM sensor_measurements
GROUP BY station_id, hour_start;

CREATE TABLE IF NOT EXISTS hourly_stats (
    station_id String,
    hour_start DateTime64(3, 'Asia/Shanghai'),
    measurement_count UInt64,
    avg_shadow_length Float64,
    min_shadow_length Float64,
    max_shadow_length Float64,
    avg_sun_altitude Float64,
    max_sun_altitude Float64,
    avg_temperature Float64
)
ENGINE = SummingMergeTree()
PARTITION BY toYYYYMM(hour_start)
ORDER BY (station_id, hour_start);

-- ============================================================
-- 6. 日降采样统计表（从小时聚合再聚合为日级）
-- ============================================================
CREATE TABLE IF NOT EXISTS daily_stats (
    station_id String,
    day_start Date COMMENT '日期',
    measurement_count UInt64,
    avg_shadow_length Float64,
    min_shadow_length Float64,
    max_shadow_length Float64,
    avg_sun_altitude Float64,
    max_sun_altitude Float64,
    avg_temperature Float64,
    alert_count UInt64 DEFAULT 0
)
ENGINE = SummingMergeTree()
PARTITION BY toYYYYMM(day_start)
ORDER BY (station_id, day_start)
TTL day_start + INTERVAL 3 YEAR
COMMENT '日级降采样统计表，保留3年';

-- ============================================================
-- 日降采样物化视图：小时→日聚合
-- ============================================================
CREATE MATERIALIZED VIEW IF NOT EXISTS daily_stats_mv
TO daily_stats
AS
SELECT
    station_id,
    toDate(hour_start) AS day_start,
    sum(measurement_count) AS measurement_count,
    avg(avg_shadow_length) AS avg_shadow_length,
    min(min_shadow_length) AS min_shadow_length,
    max(max_shadow_length) AS max_shadow_length,
    avg(avg_sun_altitude) AS avg_sun_altitude,
    max(max_sun_altitude) AS max_sun_altitude,
    avg(avg_temperature) AS avg_temperature,
    0 AS alert_count
FROM hourly_stats
GROUP BY station_id, day_start;

-- ============================================================
-- 仿真结果保留策略：6个月后移至冷存储分区
-- ============================================================
ALTER TABLE optical_simulations MODIFY TTL simulation_time + INTERVAL 180 DAY;

-- ============================================================
-- 告警事件保留策略：1年
-- ============================================================
ALTER TABLE alert_events MODIFY TTL alert_time + INTERVAL 1 YEAR;

-- ============================================================
-- 蒙特卡洛分析结果保留：2年
-- ============================================================
ALTER TABLE monte_carlo_analysis MODIFY TTL analysis_time + INTERVAL 2 YEAR;

-- ============================================================
-- 小时统计保留：6个月
-- ============================================================
ALTER TABLE hourly_stats MODIFY TTL hour_start + INTERVAL 180 DAY;

-- ============================================================
-- 7. 朝代圭表预设数据表
-- ============================================================
CREATE TABLE IF NOT EXISTS dynasty_gnomons (
    dynasty_id String COMMENT '朝代圭表ID',
    dynasty_name String COMMENT '朝代圭表名称',
    period String COMMENT '时期',
    gauge_height_chi Float64 COMMENT '表高（尺）',
    gauge_material String COMMENT '表材',
    gauge_height_error_std_chi Float64 COMMENT '表高误差标准差（尺）',
    shadow_reading_error_std_cun Float64 COMMENT '影长读数误差标准差（寸）',
    latitude Float64 COMMENT '纬度（度）',
    longitude Float64 COMMENT '经度（度）',
    altitude Float64 COMMENT '海拔（米）',
    description String COMMENT '描述',
    created_at DateTime64(3, 'Asia/Shanghai') DEFAULT now64(3)
)
ENGINE = ReplacingMergeTree(created_at)
ORDER BY dynasty_id
PRIMARY KEY dynasty_id
COMMENT '朝代圭表预设数据表';

INSERT INTO dynasty_gnomons (dynasty_id, dynasty_name, period, gauge_height_chi, gauge_material, gauge_height_error_std_chi, shadow_reading_error_std_cun, latitude, longitude, altitude, description) VALUES
('zhou_tugu', '周代土圭', '公元前11世纪—前256年', 8.0, '土筑', 0.1, 2.0, 34.25, 108.93, 400.0, '《周礼》载土圭之法，表高八尺，以土筑成，精度受限'),
('han_tongbiao', '汉代铜表', '公元前206年—公元220年', 8.0, '青铜铸造', 0.02, 0.5, 34.26, 108.94, 405.0, '汉代以铜铸表，表高八尺，材质稳定，刻度精确'),
('yuan_sizhang', '元代四丈高表', '1276年—1368年', 40.0, '砖石砌筑+铜横梁', 0.01, 0.2, 34.4897, 113.0875, 420.0, '郭守敬建登封观星台，表高四丈(40尺)，横梁针孔成像，精度达古代巅峰');

-- ============================================================
-- 8. 现代子午环仪器数据表
-- ============================================================
CREATE TABLE IF NOT EXISTS meridian_instruments (
    instrument_id String COMMENT '仪器ID',
    instrument_name String COMMENT '仪器名称',
    era String COMMENT '年代',
    angle_resolution_arcsec Float64 COMMENT '角度分辨率（角秒）',
    time_resolution_ms Float64 COMMENT '时间分辨率（毫秒）',
    systematic_error_arcsec Float64 COMMENT '系统误差（角秒）',
    description String COMMENT '描述',
    created_at DateTime64(3, 'Asia/Shanghai') DEFAULT now64(3)
)
ENGINE = ReplacingMergeTree(created_at)
ORDER BY instrument_id
PRIMARY KEY instrument_id
COMMENT '现代子午环仪器数据表';

INSERT INTO meridian_instruments (instrument_id, instrument_name, era, angle_resolution_arcsec, time_resolution_ms, systematic_error_arcsec, description) VALUES
('yuan_guibiao', '元代四丈高表', '1276', 60.0, 60000, 30.0, '郭守敬高表，影长分辨率约1分，角度分辨率约1角分'),
('modern_meridian_1900', '20世纪初子午环', '1900', 0.5, 100, 1.0, '经典光学子午环，测微显微镜读数，精度约0.5角秒'),
('modern_meridian_2000', '现代光电子午环', '2000', 0.01, 1, 0.05, 'CCD光电读数子午环，精度达0.01角秒级别');

-- ============================================================
-- 9. 针孔成像仿真结果表
-- ============================================================
CREATE TABLE IF NOT EXISTS pinhole_simulations (
    id UUID DEFAULT generateUUIDv4(),
    gauge_height_chi Float64 COMMENT '表高（尺）',
    pinhole_diameter_cun Float64 COMMENT '针孔直径（寸）',
    sun_altitude Float64 COMMENT '太阳高度角（度）',
    screen_distance_chi Float64 COMMENT '屏距（尺）',
    sun_image_diameter_cun Float64 COMMENT '太阳像直径（寸）',
    geometric_blur_cun Float64 COMMENT '几何模糊（寸）',
    diffraction_blur_cun Float64 COMMENT '衍射模糊（寸）',
    optimal_diameter_cun Float64 COMMENT '最优针孔直径（寸）',
    altitude_resolution_arcmin Float64 COMMENT '高度角分辨率（角分）',
    created_at DateTime64(3, 'Asia/Shanghai') DEFAULT now64(3)
)
ENGINE = MergeTree()
PARTITION BY toYYYYMM(created_at)
ORDER BY created_at
TTL created_at + INTERVAL 180 DAY
COMMENT '针孔成像仿真结果表';

-- ============================================================
-- 10. 虚拟体验记录表
-- ============================================================
CREATE TABLE IF NOT EXISTS virtual_experience_log (
    id UUID DEFAULT generateUUIDv4(),
    gauge_height_chi Float64 COMMENT '用户设置表高（尺）',
    latitude Float64 COMMENT '纬度',
    month UInt32 COMMENT '月份',
    day UInt32 COMMENT '日期',
    hour Float64 COMMENT '时辰（小时）',
    sun_altitude Float64 COMMENT '太阳高度角',
    shadow_length_chi Float64 COMMENT '影长（尺）',
    is_daytime UInt8 COMMENT '是否白天',
    dynasty_hint String COMMENT '朝代提示',
    created_at DateTime64(3, 'Asia/Shanghai') DEFAULT now64(3)
)
ENGINE = MergeTree()
PARTITION BY toYYYYMM(created_at)
ORDER BY created_at
TTL created_at + INTERVAL 90 DAY
COMMENT '公众虚拟体验记录表';
