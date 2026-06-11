-- ============================================================
-- 古代竹木漆器脱水加固监测系统 - 数据库初始化脚本
-- TimescaleDB + PostgreSQL
-- ============================================================

-- 创建数据库（如果不存在
-- 注意：请先手动创建数据库：CREATE DATABASE lacquer_monitor;

\c lacquer_monitor;

-- 启用TimescaleDB扩展
CREATE EXTENSION IF NOT EXISTS timescaledb;

-- ============================================================
-- 漆器表 - 50件出土饱水漆器
-- ============================================================
CREATE TABLE IF NOT EXISTS lacquer_ware (
    id SERIAL PRIMARY KEY,
    name VARCHAR(100) NOT NULL,
    artifact_code VARCHAR(50) UNIQUE NOT NULL,
    description TEXT,
    material VARCHAR(50),
    excavation_site VARCHAR(100),
    dynasty VARCHAR(50),
    initial_moisture FLOAT NOT NULL,
    current_moisture FLOAT,
    target_moisture FLOAT,
    status VARCHAR(20) DEFAULT 'active',
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW()
);

-- ============================================================
-- 传感器表 - 30台含水率传感器 + 20台收缩应变片
-- ============================================================
CREATE TABLE IF NOT EXISTS sensors (
    id SERIAL PRIMARY KEY,
    device_id VARCHAR(50) UNIQUE NOT NULL,
    sensor_type VARCHAR(20) NOT NULL, -- 'moisture' (含水率) 或 'strain' (收缩应变)
    lacquer_ware_id INTEGER REFERENCES lacquer_ware(id),
    location_on_xyz VARCHAR(100),
    installation_date DATE,
    calibration_data JSONB,
    status VARCHAR(20) DEFAULT 'active',
    nb_iot_imsi VARCHAR(50),
    created_at TIMESTAMPTZ DEFAULT NOW()
);

-- ============================================================
-- 含水率数据表 - 介电法含水率数据
-- ============================================================
CREATE TABLE IF NOT EXISTS moisture_data (
    time TIMESTAMPTZ NOT NULL,
    sensor_id INTEGER NOT NULL,
    lacquer_ware_id INTEGER NOT NULL,
    moisture_content FLOAT NOT NULL, -- 含水率 (%)
    temperature FLOAT,
    raw_value FLOAT,
    battery_level FLOAT,
    signal_strength FLOAT
);

-- 创建TimescaleDB超表
SELECT create_hypertable('moisture_data', 'time', if_not_exists => TRUE);

-- 添加索引
CREATE INDEX IF NOT EXISTS idx_moisture_sensor_time ON moisture_data(sensor_id, time DESC);
CREATE INDEX IF NOT EXISTS idx_moisture_lacquer_time ON moisture_data(lacquer_ware_id, time DESC);

-- ============================================================
-- 收缩应变数据表 - 应变片数据
-- ============================================================
CREATE TABLE IF NOT EXISTS strain_data (
    time TIMESTAMPTZ NOT NULL,
    sensor_id INTEGER NOT NULL,
    lacquer_ware_id INTEGER NOT NULL,
    strain_value FLOAT NOT NULL, -- 收缩应变 (%)
    temperature FLOAT,
    raw_value FLOAT,
    battery_level FLOAT,
    signal_strength FLOAT
);

-- 创建TimescaleDB超表
SELECT create_hypertable('strain_data', 'time', if_not_exists => TRUE);

-- 添加索引
CREATE INDEX IF NOT EXISTS idx_strain_sensor_time ON strain_data(sensor_id, time DESC);
CREATE INDEX IF NOT EXISTS idx_strain_lacquer_time ON strain_data(lacquer_ware_id, time DESC);

-- ============================================================
-- 加固剂表 - PEG、蔗糖等
-- ============================================================
CREATE TABLE IF NOT EXISTS reinforcement_agents (
    id SERIAL PRIMARY KEY,
    name VARCHAR(50) NOT NULL,
    agent_type VARCHAR(20) NOT NULL, -- 'PEG, sucrose
    concentration FLOAT NOT NULL, -- 浓度 (%)
    viscosity FLOAT, -- 粘度 (Pa·s)
    description TEXT,
    created_at TIMESTAMPTZ DEFAULT NOW()
);

-- ============================================================
-- 渗透深度预测表
-- ============================================================
CREATE TABLE IF NOT EXISTS penetration_predictions (
    id SERIAL PRIMARY KEY,
    lacquer_ware_id INTEGER REFERENCES lacquer_ware(id),
    agent_id INTEGER REFERENCES reinforcement_agents(id),
    prediction_time TIMESTAMPTZ NOT NULL,
    depth FLOAT NOT NULL, -- 预测深度 (mm)
    time_hours FLOAT NOT NULL, -- 预测时间 (小时)
    model_params JSONB,
    created_at TIMESTAMPTZ DEFAULT NOW()
);

-- ============================================================
-- 告警表
-- ============================================================
CREATE TABLE IF NOT EXISTS alerts (
    id SERIAL PRIMARY KEY,
    alert_type VARCHAR(50) NOT NULL, -- 'moisture_drop', 'strain_exceed'
    severity VARCHAR(20) NOT NULL, -- 'warning', 'critical'
    lacquer_ware_id INTEGER REFERENCES lacquer_ware(id),
    sensor_id INTEGER,
    message TEXT,
    value FLOAT, -- 触发值
    threshold FLOAT, -- 阈值
    is_acknowledged BOOLEAN DEFAULT FALSE,
    created_at TIMESTAMPTZ DEFAULT NOW()
);

-- ============================================================
-- 告警推送记录表
-- ============================================================
CREATE TABLE IF NOT EXISTS alert_notifications (
    id SERIAL PRIMARY KEY,
    alert_id INTEGER REFERENCES alerts(id),
    channel VARCHAR(20) NOT NULL, -- 'sms', 'satellite'
    recipient VARCHAR(100) NOT NULL,
    status VARCHAR(20) DEFAULT 'pending', -- 'pending', 'sent', 'failed'
    response TEXT,
    sent_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ DEFAULT NOW()
);

-- ============================================================
-- 插入示例数据 - 漆器 (50件)
-- ============================================================
DO $$
DECLARE
    i INTEGER;
    ware_names TEXT[] := ARRAY[
        '战国黑漆耳杯', '西汉漆木俑', '秦代漆奁', '汉代漆耳杯', '唐代漆盒',
        '宋代漆盘', '明代漆盒', '清代漆瓶', '战国漆木梳', '汉代漆木盘',
        '唐代漆木碗', '宋代漆木盒', '明代漆木盘', '清代漆木箱', '战国漆木鼎',
        '汉代漆木壶', '唐代漆木盒', '宋代漆木碗', '明代漆木瓶', '清代漆木盘',
        '战国漆木奁', '汉代漆木杯', '唐代漆木盘', '宋代漆木盒', '明代漆木碗',
        '清代漆木瓶', '战国漆木箱', '汉代漆木奁', '唐代漆木杯', '宋代漆木盘',
        '明代漆木盒', '清代漆木碗', '战国漆木瓶', '汉代漆木箱', '唐代漆木奁',
        '宋代漆木杯', '明代漆木盘', '清代漆木盒', '战国漆木碗', '汉代漆木瓶',
        '唐代漆木箱', '宋代漆木奁', '明代漆木杯', '清代漆木盘', '战国漆木盒',
        '汉代漆木碗', '唐代漆木瓶', '宋代漆木箱', '明代漆木奁', '清代漆木杯'
    ];
    materials TEXT[] := ARRAY['木胎漆', '竹胎漆', '夹纻胎', '布胎漆'];
    sites TEXT[] := ARRAY['马王堆汉墓', '曾侯乙墓', '江陵楚墓', '包山楚墓', '睡虎地秦墓'];
    dynasties TEXT[] := ARRAY['战国', '秦代', '汉代', '唐代', '宋代', '明代', '清代'];
BEGIN
    FOR i IN 1..50 LOOP
        INSERT INTO lacquer_ware (name, artifact_code, description, material, excavation_site, dynasty, initial_moisture, current_moisture, target_moisture)
        VALUES (
            ware_names[i],
            'LQ' || LPAD(i::TEXT, 4, '0'),
            '出土饱水漆器，编号' || i,
            materials[1 + (i % 4)],
            sites[1 + (i % 5)],
            dynasties[1 + (i % 7)],
            80 + (i % 15)::FLOAT,
            80 + (i % 15)::FLOAT,
            12.0
        );
    END LOOP;
END $$;

-- ============================================================
-- 插入示例数据 - 传感器 (30台含水率 + 20台应变片)
-- ============================================================
-- 30台含水率传感器 (介电法)
DO $$
DECLARE
    i INTEGER;
BEGIN
    FOR i IN 1..30 LOOP
        INSERT INTO sensors (device_id, sensor_type, lacquer_ware_id, location_on_xyz, nb_iot_imsi)
        VALUES (
            'MS' || LPAD(i::TEXT, 4, '0'),
            'moisture',
            1 + ((i - 1) * 50 / 30),
            '表面点' || (i % 5),
            '46000' || LPAD(i::TEXT, 10, '0')
        );
    END LOOP;
END $$;

-- 20台收缩应变片
DO $$
DECLARE
    i INTEGER;
BEGIN
    FOR i IN 1..20 LOOP
        INSERT INTO sensors (device_id, sensor_type, lacquer_ware_id, location_on_xyz, nb_iot_imsi)
        VALUES (
            'SS' || LPAD(i::TEXT, 4, '0'),
            'strain',
            1 + ((i - 1) * 50 / 20),
            '边缘点' || (i % 4),
            '46000' || LPAD(30 + i, 10, '0')
        );
    END LOOP;
END $$;

-- ============================================================
-- 插入加固剂数据
-- ============================================================
INSERT INTO reinforcement_agents (name, agent_type, concentration, viscosity, description) VALUES
('PEG-2000', 'PEG', 30.0, 0.056, '聚乙二醇2000，30%浓度'),
('PEG-4000', 'PEG', 40.0, 0.089, '聚乙二醇4000，40%浓度'),
('蔗糖溶液', 'sucrose', 50.0, 0.12, '蔗糖溶液，50%浓度');

-- ============================================================
-- 创建视图 - 最新含水率统计
-- ============================================================
CREATE OR REPLACE VIEW latest_moisture_view AS
SELECT DISTINCT ON (sensor_id)
    sensor_id,
    lacquer_ware_id,
    time,
    moisture_content,
    temperature
FROM moisture_data
ORDER BY sensor_id, time DESC;

-- ============================================================
-- 创建视图 - 最新应变统计
-- ============================================================
CREATE OR REPLACE VIEW latest_strain_view AS
SELECT DISTINCT ON (sensor_id)
    sensor_id,
    lacquer_ware_id,
    time,
    strain_value,
    temperature
FROM strain_data
ORDER BY sensor_id, time DESC;

-- ============================================================
-- 插入一些历史数据生成函数 - 生成7天历史数据
-- ============================================================
CREATE OR REPLACE FUNCTION generate_sample_data()
RETURNS VOID AS $$
DECLARE
    sensor_row RECORD;
    t TIMESTAMPTZ;
    start_time TIMESTAMPTZ;
    moisture_val FLOAT;
    strain_val FLOAT;
    lacquer_id INTEGER;
BEGIN
    start_time := NOW() - INTERVAL '7 days';
    
    -- 为每个含水率传感器生成历史数据
    FOR sensor_row IN SELECT id, lacquer_ware_id, sensor_type FROM sensors LOOP
        IF sensor_row.sensor_type = 'moisture' THEN
            -- 生成7天的每小时数据
            FOR i IN 0..168 LOOP
                t := start_time + (i * INTERVAL '1 hour');
                moisture_val := 75 + (random() * 10 - 5) + (i * -0.1 * random());
                INSERT INTO moisture_data (time, sensor_id, lacquer_ware_id, moisture_content, temperature, battery_level, signal_strength)
                VALUES (
                    t,
                    sensor_row.id,
                    sensor_row.lacquer_ware_id,
                    GREATEST(moisture_val, 5.0),
                    20 + random() * 5,
                    80 + random() * 20,
                    -70 + random() * 30
                );
            END LOOP;
        ELSIF sensor_row.sensor_type = 'strain' THEN
            FOR i IN 0..168 LOOP
                t := start_time + (i * INTERVAL '1 hour');
                strain_val := 0.5 + (random() * 2) + (i * 0.01 * random());
                INSERT INTO strain_data (time, sensor_id, lacquer_ware_id, strain_value, temperature, battery_level, signal_strength)
                VALUES (
                    t,
                    sensor_row.id,
                    sensor_row.lacquer_ware_id,
                    LEAST(strain_val, 8.0),
                    20 + random() * 5,
                    80 + random() * 20,
                    -70 + random() * 30
                );
            END LOOP;
        END IF;
    END LOOP;
END;
$$ LANGUAGE plpgsql;

-- 执行生成示例数据
SELECT generate_sample_data();

-- ============================================================
-- TimescaleDB 压缩策略 - 原始数据保留2年
-- ============================================================

-- 启用含水率数据表压缩
ALTER TABLE moisture_data SET (
  timescaledb.compress,
  timescaledb.compress_segmentby = 'sensor_id, lacquer_ware_id',
  timescaledb.compress_orderby = 'time DESC'
);

-- 启用应变数据表压缩
ALTER TABLE strain_data SET (
  timescaledb.compress,
  timescaledb.compress_segmentby = 'sensor_id, lacquer_ware_id',
  timescaledb.compress_orderby = 'time DESC'
);

-- 添加压缩策略：7天以上的数据自动压缩
SELECT add_compression_policy('moisture_data', INTERVAL '7 days', if_not_exists => TRUE);
SELECT add_compression_policy('strain_data', INTERVAL '7 days', if_not_exists => TRUE);

-- 添加数据保留策略：原始数据保留2年，2年以上自动删除
SELECT add_retention_policy('moisture_data', INTERVAL '2 years', if_not_exists => TRUE);
SELECT add_retention_policy('strain_data', INTERVAL '2 years', if_not_exists => TRUE);

-- 创建连续聚合视图：每日含水率统计
CREATE MATERIALIZED VIEW IF NOT EXISTS moisture_daily_summary
WITH (timescaledb.continuous) AS
SELECT
  time_bucket('1 day', time) AS bucket,
  sensor_id,
  lacquer_ware_id,
  AVG(moisture_content) AS avg_moisture,
  MIN(moisture_content) AS min_moisture,
  MAX(moisture_content) AS max_moisture,
  COUNT(*) AS reading_count
FROM moisture_data
GROUP BY bucket, sensor_id, lacquer_ware_id
WITH NO DATA;

-- 为连续聚合视图添加刷新策略
SELECT add_continuous_aggregate_policy('moisture_daily_summary',
  start_offset => INTERVAL '3 days',
  end_offset => INTERVAL '1 hour',
  schedule_interval => INTERVAL '1 hour',
  if_not_exists => TRUE
);

-- 创建连续聚合视图：每日应变统计
CREATE MATERIALIZED VIEW IF NOT EXISTS strain_daily_summary
WITH (timescaledb.continuous) AS
SELECT
  time_bucket('1 day', time) AS bucket,
  sensor_id,
  lacquer_ware_id,
  AVG(strain_value) AS avg_strain,
  MIN(strain_value) AS min_strain,
  MAX(strain_value) AS max_strain,
  COUNT(*) AS reading_count
FROM strain_data
GROUP BY bucket, sensor_id, lacquer_ware_id
WITH NO DATA;

-- 为连续聚合视图添加刷新策略
SELECT add_continuous_aggregate_policy('strain_daily_summary',
  start_offset => INTERVAL '3 days',
  end_offset => INTERVAL '1 hour',
  schedule_interval => INTERVAL '1 hour',
  if_not_exists => TRUE
);

-- 启用告警表索引
CREATE INDEX IF NOT EXISTS idx_alerts_created ON alerts(created_at DESC);
CREATE INDEX IF NOT EXISTS idx_alerts_type ON alerts(alert_type);
CREATE INDEX IF NOT EXISTS idx_alerts_severity ON alerts(severity);
CREATE INDEX IF NOT EXISTS idx_alerts_acknowledged ON alerts(is_acknowledged);
