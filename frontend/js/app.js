const API_BASE = 'http://localhost:8080/api';

let lacquerModel = null;
let moistureHeatmap = null;
let selectedWareId = 1;
let currentMode = 'moisture';
let moistureChartCtx = null;
let strainChartCtx = null;
let predictionChartCtx = null;
let penetrationChartCtx = null;

document.addEventListener('DOMContentLoaded', () => {
    initVisualizer();
    initCharts();
    initEventListeners();
    loadInitialData();
    startDataRefresh();
});

function initVisualizer() {
    lacquerModel = new LacquerModel('threeCanvas');
    moistureHeatmap = new MoistureHeatmap(lacquerModel);
    lacquerModel.setMode('moisture');

    const mockMoisture = {};
    const mockStrain = {};
    for (let i = 0; i < 5; i++) {
        mockMoisture[`sensor_${i}`] = 65 + Math.random() * 15;
    }
    for (let i = 0; i < 4; i++) {
        mockStrain[`strain_${i}`] = 1 + Math.random() * 2;
    }
    moistureHeatmap.updateData(mockMoisture);
    lacquerModel.updateStrainData(mockStrain);
}

function initCharts() {
    moistureChartCtx = document.getElementById('moistureChart').getContext('2d');
    strainChartCtx = document.getElementById('strainChart').getContext('2d');
    predictionChartCtx = document.getElementById('predictionChart').getContext('2d');
    penetrationChartCtx = document.getElementById('penetrationChart').getContext('2d');

    drawMockChart(moistureChartCtx, '#5dade2', 50, 80, '%');
    drawMockChart(strainChartCtx, '#e74c3c', 0, 5, '%');
    drawPredictionChart();
    drawPenetrationChart();
}

function drawMockChart(ctx, color, min, max, unit) {
    const width = ctx.canvas.width;
    const height = ctx.canvas.height;
    const padding = { top: 10, right: 10, bottom: 20, left: 35 };

    ctx.clearRect(0, 0, width, height);

    const dataPoints = 24;
    const data = [];
    let value = (min + max) / 2;

    for (let i = 0; i < dataPoints; i++) {
        value += (Math.random() - 0.55) * (max - min) * 0.1;
        value = Math.max(min, Math.min(max, value));
        data.push(value);
    }

    const chartWidth = width - padding.left - padding.right;
    const chartHeight = height - padding.top - padding.bottom;

    ctx.strokeStyle = 'rgba(42, 58, 74, 0.5)';
    ctx.lineWidth = 1;
    for (let i = 0; i <= 4; i++) {
        const y = padding.top + (chartHeight * i / 4);
        ctx.beginPath();
        ctx.moveTo(padding.left, y);
        ctx.lineTo(width - padding.right, y);
        ctx.stroke();

        const labelValue = max - ((max - min) * i / 4);
        ctx.fillStyle = '#62758a';
        ctx.font = '10px sans-serif';
        ctx.textAlign = 'right';
        ctx.fillText(labelValue.toFixed(0) + unit, padding.left - 5, y + 3);
    }

    ctx.strokeStyle = color;
    ctx.lineWidth = 2;
    ctx.beginPath();

    for (let i = 0; i < dataPoints; i++) {
        const x = padding.left + (chartWidth * i / (dataPoints - 1));
        const y = padding.top + chartHeight - ((data[i] - min) / (max - min)) * chartHeight;

        if (i === 0) {
            ctx.moveTo(x, y);
        } else {
            ctx.lineTo(x, y);
        }
    }
    ctx.stroke();

    const gradient = ctx.createLinearGradient(0, padding.top, 0, height - padding.bottom);
    gradient.addColorStop(0, color + '40');
    gradient.addColorStop(1, color + '05');

    ctx.fillStyle = gradient;
    ctx.beginPath();
    ctx.moveTo(padding.left, height - padding.bottom);
    for (let i = 0; i < dataPoints; i++) {
        const x = padding.left + (chartWidth * i / (dataPoints - 1));
        const y = padding.top + chartHeight - ((data[i] - min) / (max - min)) * chartHeight;
        ctx.lineTo(x, y);
    }
    ctx.lineTo(width - padding.right, height - padding.bottom);
    ctx.closePath();
    ctx.fill();
}

function drawPredictionChart() {
    const ctx = predictionChartCtx;
    const width = ctx.canvas.width;
    const height = ctx.canvas.height;
    const padding = { top: 10, right: 10, bottom: 25, left: 35 };

    ctx.clearRect(0, 0, width, height);

    const dataPoints = 50;
    const initialMoisture = 75;
    const targetMoisture = 12;
    const timeHours = 720;

    const data = [];
    for (let i = 0; i < dataPoints; i++) {
        const t = i / (dataPoints - 1);
        const moisture = targetMoisture + (initialMoisture - targetMoisture) * Math.exp(-t * 3);
        data.push({ time: t * timeHours, moisture });
    }

    const chartWidth = width - padding.left - padding.right;
    const chartHeight = height - padding.top - padding.bottom;

    ctx.strokeStyle = 'rgba(42, 58, 74, 0.5)';
    ctx.lineWidth = 1;
    for (let i = 0; i <= 4; i++) {
        const y = padding.top + (chartHeight * i / 4);
        ctx.beginPath();
        ctx.moveTo(padding.left, y);
        ctx.lineTo(width - padding.right, y);
        ctx.stroke();
    }

    ctx.strokeStyle = '#27ae60';
    ctx.lineWidth = 2;
    ctx.beginPath();

    for (let i = 0; i < data.length; i++) {
        const x = padding.left + (chartWidth * i / (data.length - 1));
        const y = padding.top + chartHeight - ((data[i].moisture - 10) / 70) * chartHeight;

        if (i === 0) {
            ctx.moveTo(x, y);
        } else {
            ctx.lineTo(x, y);
        }
    }
    ctx.stroke();

    const gradient = ctx.createLinearGradient(0, padding.top, 0, height - padding.bottom);
    gradient.addColorStop(0, 'rgba(39, 174, 96, 0.3)');
    gradient.addColorStop(1, 'rgba(39, 174, 96, 0.02)');

    ctx.fillStyle = gradient;
    ctx.beginPath();
    ctx.moveTo(padding.left, height - padding.bottom);
    for (let i = 0; i < data.length; i++) {
        const x = padding.left + (chartWidth * i / (data.length - 1));
        const y = padding.top + chartHeight - ((data[i].moisture - 10) / 70) * chartHeight;
        ctx.lineTo(x, y);
    }
    ctx.lineTo(width - padding.right, height - padding.bottom);
    ctx.closePath();
    ctx.fill();

    ctx.fillStyle = '#62758a';
    ctx.font = '10px sans-serif';
    ctx.textAlign = 'center';
    for (let i = 0; i <= 4; i++) {
        const x = padding.left + (chartWidth * i / 4);
        const days = (timeHours * i / 4 / 24).toFixed(0);
        ctx.fillText(days + 'd', x, height - 8);
    }
}

function drawPenetrationChart() {
    const ctx = penetrationChartCtx;
    const width = ctx.canvas.width;
    const height = ctx.canvas.height;
    const padding = { top: 10, right: 10, bottom: 25, left: 35 };

    ctx.clearRect(0, 0, width, height);

    const dataPoints = 50;
    const timeHours = 48;
    const maxDepth = 5;

    const data = [];
    for (let i = 0; i < dataPoints; i++) {
        const t = i / (dataPoints - 1);
        const depth = maxDepth * Math.sqrt(t);
        data.push({ time: t * timeHours, depth });
    }

    const chartWidth = width - padding.left - padding.right;
    const chartHeight = height - padding.top - padding.bottom;

    ctx.strokeStyle = 'rgba(42, 58, 74, 0.5)';
    ctx.lineWidth = 1;
    for (let i = 0; i <= 4; i++) {
        const y = padding.top + (chartHeight * i / 4);
        ctx.beginPath();
        ctx.moveTo(padding.left, y);
        ctx.lineTo(width - padding.right, y);
        ctx.stroke();
    }

    ctx.strokeStyle = '#9b59b6';
    ctx.lineWidth = 2;
    ctx.beginPath();

    for (let i = 0; i < data.length; i++) {
        const x = padding.left + (chartWidth * i / (data.length - 1));
        const y = padding.top + chartHeight - (data[i].depth / maxDepth) * chartHeight;

        if (i === 0) {
            ctx.moveTo(x, y);
        } else {
            ctx.lineTo(x, y);
        }
    }
    ctx.stroke();

    const gradient = ctx.createLinearGradient(0, padding.top, 0, height - padding.bottom);
    gradient.addColorStop(0, 'rgba(155, 89, 182, 0.3)');
    gradient.addColorStop(1, 'rgba(155, 89, 182, 0.02)');

    ctx.fillStyle = gradient;
    ctx.beginPath();
    ctx.moveTo(padding.left, height - padding.bottom);
    for (let i = 0; i < data.length; i++) {
        const x = padding.left + (chartWidth * i / (data.length - 1));
        const y = padding.top + chartHeight - (data[i].depth / maxDepth) * chartHeight;
        ctx.lineTo(x, y);
    }
    ctx.lineTo(width - padding.right, height - padding.bottom);
    ctx.closePath();
    ctx.fill();

    ctx.fillStyle = '#62758a';
    ctx.font = '10px sans-serif';
    ctx.textAlign = 'center';
    for (let i = 0; i <= 4; i++) {
        const x = padding.left + (chartWidth * i / 4);
        const hours = (timeHours * i / 4).toFixed(0);
        ctx.fillText(hours + 'h', x, height - 8);
    }
}

function initEventListeners() {
    document.querySelectorAll('.mode-btn').forEach(btn => {
        btn.addEventListener('click', () => {
            document.querySelectorAll('.mode-btn').forEach(b => b.classList.remove('active'));
            btn.classList.add('active');
            const mode = btn.dataset.mode;
            currentMode = mode;
            lacquerModel.setMode(mode);
        });
    });

    document.getElementById('resetView').addEventListener('click', () => {
        lacquerModel.resetView();
    });

    document.getElementById('toggleWireframe').addEventListener('click', () => {
        lacquerModel.toggleWireframe();
    });

    document.getElementById('toggleAutoRotate').addEventListener('click', () => {
        lacquerModel.toggleAutoRotate();
    });

    document.getElementById('zoomIn').addEventListener('click', () => {
        lacquerModel.zoomIn();
    });

    document.getElementById('zoomOut').addEventListener('click', () => {
        lacquerModel.zoomOut();
    });
}

async function loadInitialData() {
    try {
        await Promise.all([
            loadStatistics(),
            loadLacquerWares(),
            loadAlerts(),
            loadReinforcementAgents()
        ]);
    } catch (e) {
        console.warn('API not available, using mock data');
        loadMockData();
    }
}

async function loadStatistics() {
    try {
        const response = await fetch(`${API_BASE}/statistics`);
        const data = await response.json();
        if (data.success && data.data) {
            document.getElementById('totalWares').textContent = data.data.total_lacquer_wares || 50;
            document.getElementById('totalSensors').textContent = data.data.total_sensors || 50;
            document.getElementById('avgMoisture').textContent = (data.data.avg_moisture || 65).toFixed(1) + '%';
            document.getElementById('activeAlerts').textContent = data.data.active_alerts || 0;
        }
    } catch (e) {
        throw e;
    }
}

async function loadLacquerWares() {
    try {
        const response = await fetch(`${API_BASE}/lacquer-wares?limit=50`);
        const data = await response.json();
        if (data.success && data.data) {
            renderWareList(data.data);
        }
    } catch (e) {
        throw e;
    }
}

function renderWareList(wares) {
    const container = document.getElementById('wareList');
    container.innerHTML = '';

    wares.forEach(ware => {
        const item = document.createElement('div');
        item.className = 'ware-item' + (ware.id === selectedWareId ? ' active' : '');
        item.innerHTML = `
            <div class="ware-name">${ware.name}</div>
            <div class="ware-code">${ware.artifact_code}</div>
            <div class="ware-moisture">${ware.current_moisture ? ware.current_moisture.toFixed(1) : '--'}%</div>
        `;
        item.addEventListener('click', () => selectWare(ware.id));
        container.appendChild(item);
    });
}

function selectWare(id) {
    selectedWareId = id;
    document.querySelectorAll('.ware-item').forEach(item => {
        item.classList.remove('active');
    });
    event.currentTarget.classList.add('active');

    loadWareData(id);
}

async function loadWareData(wareId) {
    try {
        const [moistureResp, strainResp] = await Promise.all([
            fetch(`${API_BASE}/lacquer-wares/${wareId}/moisture`),
            fetch(`${API_BASE}/lacquer-wares/${wareId}/strain`)
        ]);

        const moistureData = await moistureResp.json();
        const strainData = await strainResp.json();

        if (moistureData.success && moistureData.data) {
            const moistureMap = {};
            moistureData.data.forEach(d => {
                moistureMap[`sensor_${d.sensor_id}`] = d.moisture_content;
            });
            moistureHeatmap.updateData(moistureMap);
        }

        if (strainData.success && strainData.data) {
            const strainMap = {};
            strainData.data.forEach(d => {
                strainMap[`strain_${d.sensor_id}`] = d.strain_value;
            });
            lacquerModel.updateStrainData(strainMap);
        }

        updateSensorInfo(moistureData.data || [], strainData.data || []);
    } catch (e) {
        console.warn('Ware data load failed, using mock');
    }
}

function updateSensorInfo(moistureData, strainData) {
    const container = document.getElementById('sensorInfo');
    
    const latestMoisture = {};
    const latestStrain = {};

    moistureData.forEach(d => {
        if (!latestMoisture[d.sensor_id] || new Date(d.time) > new Date(latestMoisture[d.sensor_id].time)) {
            latestMoisture[d.sensor_id] = d;
        }
    });

    strainData.forEach(d => {
        if (!latestStrain[d.sensor_id] || new Date(d.time) > new Date(latestStrain[d.sensor_id].time)) {
            latestStrain[d.sensor_id] = d;
        }
    });

    let html = '';
    Object.values(latestMoisture).slice(0, 3).forEach(d => {
        html += `
            <div class="sensor-detail">
                <div class="sensor-id">💧 传感器 #${d.sensor_id}</div>
                <div class="sensor-value">${d.moisture_content.toFixed(1)}%</div>
            </div>
        `;
    });

    Object.values(latestStrain).slice(0, 3).forEach(d => {
        html += `
            <div class="sensor-detail">
                <div class="sensor-id">📏 应变片 #${d.sensor_id}</div>
                <div class="sensor-value" style="color:#e74c3c;">${d.strain_value.toFixed(3)}%</div>
            </div>
        `;
    });

    container.innerHTML = html || '<p class="empty-text">暂无传感器数据</p>';
}

async function loadAlerts() {
    try {
        const response = await fetch(`${API_BASE}/alerts?limit=10`);
        const data = await response.json();
        if (data.success && data.data) {
            renderAlerts(data.data);
        }
    } catch (e) {
    }
}

function renderAlerts(alerts) {
    const container = document.getElementById('alertList');
    
    if (!alerts || alerts.length === 0) {
        container.innerHTML = '<p class="empty-text">暂无告警</p>';
        return;
    }

    container.innerHTML = '';
    alerts.slice(0, 5).forEach(alert => {
        const item = document.createElement('div');
        item.className = `alert-item ${alert.severity}`;
        
        const typeText = alert.alert_type === 'moisture_drop' ? '含水率突降' : '收缩应变超标';
        const time = new Date(alert.created_at).toLocaleString('zh-CN', {
            month: 'short',
            day: 'numeric',
            hour: '2-digit',
            minute: '2-digit'
        });

        item.innerHTML = `
            <div class="alert-title">${typeText}</div>
            <div class="alert-desc">${alert.message || ''}</div>
            <div class="alert-time">${time}</div>
        `;
        container.appendChild(item);
    });
}

async function loadReinforcementAgents() {
    try {
        const response = await fetch(`${API_BASE}/reinforcement-agents`);
        const data = await response.json();
        if (data.success && data.data) {
            renderAgents(data.data);
        }
    } catch (e) {
    }
}

function renderAgents(agents) {
    const container = document.getElementById('agentList');
    container.innerHTML = '';

    agents.forEach((agent, index) => {
        const item = document.createElement('div');
        item.className = 'agent-item' + (index === 0 ? ' active' : '');
        item.innerHTML = `
            <div class="agent-name">${agent.name}</div>
            <div class="agent-type">${agent.agent_type.toUpperCase()}</div>
            <div class="agent-conc">浓度: ${agent.concentration}%</div>
        `;
        item.addEventListener('click', () => {
            document.querySelectorAll('.agent-item').forEach(a => a.classList.remove('active'));
            item.classList.add('active');
        });
        container.appendChild(item);
    });
}

function loadMockData() {
    const wares = [];
    for (let i = 1; i <= 20; i++) {
        wares.push({
            id: i,
            name: `漆器 #${i}`,
            artifact_code: `LQ${String(i).padStart(4, '0')}`,
            current_moisture: 60 + Math.random() * 20
        });
    }
    renderWareList(wares);

    const mockAlerts = [
        { id: 1, alert_type: 'moisture_drop', severity: 'warning', message: '含水率下降过快', created_at: new Date().toISOString() },
        { id: 2, alert_type: 'strain_exceed', severity: 'critical', message: '收缩应变超过阈值', created_at: new Date(Date.now() - 3600000).toISOString() },
    ];
    renderAlerts(mockAlerts);

    document.getElementById('activeAlerts').textContent = mockAlerts.length;
    document.getElementById('initMoisture').textContent = '75%';
    document.getElementById('estTime').textContent = '30 天';
}

function startDataRefresh() {
    setInterval(() => {
        loadStatistics().catch(() => {});
        loadAlerts().catch(() => {});
    }, 30000);

    setInterval(() => {
        drawMockChart(moistureChartCtx, '#5dade2', 50, 80, '%');
        drawMockChart(strainChartCtx, '#e74c3c', 0, 5, '%');
    }, 5000);
}
