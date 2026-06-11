# 古代竹木漆器脱水加固监测系统

## 项目概述

本系统是一套完整的文物保护监测解决方案，针对出土饱水漆器的脱水加固过程进行实时监测、预测和告警。系统采用Rust后端 + TimescaleDB时序数据库 + Three.js前端3D可视化的全栈架构。

## 系统架构

```
┌─────────────────────────────────────────────────────────┐
│                        前端 (Three.js)                   │
│  - 3D漆器模型展示  - 含水率热力图  - 应变网格变形       │
└───────────────────────┬─────────────────────────────────┘
                        │ HTTP API
┌───────────────────────▼─────────────────────────────────┐
│                   Rust 后端 (Actix-web)                 │
│  - REST API  - 核心算法  - 告警系统  - 数据处理         │
└───────────────────────┬─────────────────────────────────┘
                        │
┌───────────────────────▼─────────────────────────────────┐
│                 TimescaleDB (PostgreSQL)                │
│  - 超表存储时序数据  - 高效聚合查询  - 数据压缩         │
└───────────────────────▲─────────────────────────────────┘
                        │ NB-IoT
┌───────────────────────┴─────────────────────────────────┐
│                  NB-IoT 传感器网络                      │
│  - 30台含水率传感器(介电法)  - 20台收缩应变片           │
└─────────────────────────────────────────────────────────┘
```

## 核心功能

### 1. 实时监测
- **含水率监测**：30台介电法含水率传感器，每小时上报
- **收缩应变监测**：20台应变片，实时监测漆器变形
- **3D可视化**：Three.js展示漆器三维模型，蓝色热力图显示含水率分布，红色网格显示收缩应变

### 2. 核心算法

#### Fickian扩散模型（脱水过程预测）
基于菲克第二定律，模拟水分在漆器中的扩散过程：
```
∂C/∂t = D · ∂²C/∂x²
```
- 预测不同时间点的含水率分布
- 估算完全脱水所需时间
- 支持自定义扩散系数和样品厚度

#### 达西定律（加固剂渗透预测）
基于达西定律预测加固剂（PEG、蔗糖）的渗透深度：
```
v = (k · ΔP) / (μ · φ · L)
```
- 预测PEG、蔗糖等加固剂的渗透深度
- 支持不同浓度、粘度参数
- 计算渗透速度和流量

### 3. 告警系统
- **含水率突降告警**：每小时下降超过10%触发
- **收缩应变超标告警**：应变值超过5%触发
- **多渠道推送**：短信 + 卫星通信
- **告警分级**：警告(warning) / 严重(critical)

### 4. NB-IoT模拟器
模拟50台传感器通过NB-IoT网络上报数据
- 支持连续模式和单次模式
- 可自定义上报间隔
- 随机生成异常数据用于告警测试
- 快速模式（10秒上报）便于演示

## 目录结构

```
lacquer-monitor-system/
├── backend/                    # Rust后端
│   ├── Cargo.toml             # 项目配置
│   ├── .env                    # 环境变量
│   └── src/
│       ├── main.rs            # 主入口
│       ├── models.rs          # 数据模型
│       ├── db.rs              # 数据库连接
│       ├── handlers.rs        # API处理器
│       ├── algorithms.rs      # 核心算法
│       └── alerts.rs          # 告警系统
├── database/                   # 数据库
│   └── init.sql               # 初始化脚本
├── frontend/                   # 前端
│   ├── index.html             # 入口页面
│   ├── css/
│   │   └── style.css          # 样式
│   └── js/
│       ├── app.js             # 主应用逻辑
│       └── visualization.js   # Three.js可视化
└── simulator/                  # NB-IoT模拟器
    └── nb_iot_simulator.py    # 模拟器脚本
```

## 快速开始

### 前置要求
- Rust 1.70+
- PostgreSQL 14+ / TimescaleDB 2.10+
- Python 3.8+ (模拟器)
- Node.js (可选，用于前端静态服务)

### 1. 数据库初始化

```bash
# 创建数据库
psql -U postgres -c "CREATE DATABASE lacquer_monitor;"

# 执行初始化脚本
psql -U postgres -d lacquer_monitor -f database/init.sql
```

初始化脚本将自动创建：
- 50件漆器记录
- 30台含水率传感器 + 20台应变片
- 7天的历史监测数据
- TimescaleDB超表和索引

### 2. 后端启动

```bash
cd backend

# 配置环境变量
cp .env.example .env
# 编辑 .env 文件配置数据库连接

# 编译运行
cargo run --release
```

后端服务默认运行在 `http://localhost:8080`

### 3. 前端启动

```bash
cd frontend

# 方式一：使用Python简单HTTP服务器
python -m http.server 8000

# 方式二：使用Node.js serve
npx serve .
```

访问 `http://localhost:8000` 查看前端界面

### 4. 启动NB-IoT模拟器

```bash
cd simulator

# 安装依赖
pip install requests

# 运行模拟器（默认每小时上报）
python nb_iot_simulator.py

# 快速模式（每10秒上报，便于演示）
python nb_iot_simulator.py --fast

# 单次上报
python nb_iot_simulator.py --once
```

## API接口

### 统计信息
- `GET /api/statistics` - 获取系统统计数据

### 漆器管理
- `GET /api/lacquer-wares` - 获取漆器列表
- `GET /api/lacquer-wares/{id}` - 获取漆器详情

### 传感器数据
- `GET /api/sensors` - 获取传感器列表
- `GET /api/moisture/latest` - 获取最新含水率数据
- `GET /api/strain/latest` - 获取最新应变数据
- `GET /api/lacquer-wares/{id}/moisture` - 获取指定漆器含水率历史
- `GET /api/lacquer-wares/{id}/strain` - 获取指定漆器应变历史

### 预测算法
- `POST /api/predict/moisture` - 含水率损失预测（Fickian扩散）
- `POST /api/predict/penetration` - 加固剂渗透预测（达西定律）

### 加固剂
- `GET /api/reinforcement-agents` - 获取加固剂列表

### 告警
- `GET /api/alerts` - 获取告警列表

### NB-IoT数据上报
- `POST /api/nb-iot/data` - 接收NB-IoT传感器数据

## 配置说明

### 环境变量 (backend/.env)

| 变量名 | 说明 | 默认值 |
|--------|------|--------|
| DATABASE_URL | 数据库连接串 | postgres://postgres:postgres@localhost:5432/lacquer_monitor |
| SERVER_HOST | 服务监听地址 | 0.0.0.0 |
| SERVER_PORT | 服务端口 | 8080 |
| SMS_API_KEY | 短信API密钥 | - |
| SMS_API_URL | 短信API地址 | - |
| SATELLITE_API_KEY | 卫星通信API密钥 | - |
| SATELLITE_API_URL | 卫星通信API地址 | - |
| ALERT_PHONE_NUMBER | 告警接收手机号 | 13800138000 |

### 告警阈值

| 告警类型 | 阈值 | 级别 |
|----------|------|------|
| 含水率突降 | >10%/小时 | warning |
| 含水率突降 | >20%/小时 | critical |
| 收缩应变 | >5% | warning |
| 收缩应变 | >8% | critical |

## 技术栈

### 后端
- **语言**：Rust
- **框架**：Actix-web 4.4
- **数据库**：PostgreSQL + TimescaleDB
- **连接池**：deadpool-postgres
- **序列化**：Serde
- **日志**：tracing

### 前端
- **3D引擎**：Three.js r128
- **控制器**：OrbitControls
- **图表**：原生Canvas 2D

### 数据库
- **时序数据库**：TimescaleDB 2.x
- **基础数据库**：PostgreSQL 14+

## 核心算法详解

### Fickian扩散模型

菲克第二定律描述了扩散过程中浓度随时间的变化：

```
∂C/∂t = D · ∇²C
```

对于一维平板样品，解析解为：

```
C(x,t) = Cs + (C0 - Cs) · (4/π) · Σ[(-1)^n/(2n+1) · cos((2n+1)πx/(2L)) · exp(-D(2n+1)²π²t/(4L²))]
```

其中：
- C(x,t)：位置x处t时刻的浓度
- D：扩散系数
- L：样品半厚度
- C0：初始浓度
- Cs：表面浓度

### 达西定律

达西定律描述了流体在多孔介质中的流动：

```
Q = (k · A · ΔP) / (μ · L)
```

渗透深度随时间的变化：

```
x(t) = √(2kΔPt / (μφ))
```

其中：
- k：渗透率
- μ：流体粘度
- ΔP：压力差
- φ：孔隙率

## 扩展说明

- 系统支持水平扩展，可接入更多传感器
- 告警通道可扩展为邮件、微信、钉钉等
- 算法模型可根据实际文物材料参数进行校准
- 支持导出监测报告和预测分析报告

## 许可证

本项目仅供学术研究和文物保护使用。
