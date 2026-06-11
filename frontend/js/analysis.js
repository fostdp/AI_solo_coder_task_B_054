const API_BASE = '/api';

document.addEventListener('DOMContentLoaded', () => {
    initTabs();
    initStressModule();
    initConcentrationModule();
    initGPRModule();
    initStabilityModule();
});

function initTabs() {
    const tabBtns = document.querySelectorAll('.tab-btn');
    const tabPanels = document.querySelectorAll('.tab-panel');

    tabBtns.forEach(btn => {
        btn.addEventListener('click', () => {
            const tabId = btn.dataset.tab;

            tabBtns.forEach(b => b.classList.remove('active'));
            tabPanels.forEach(p => p.classList.remove('active'));

            btn.classList.add('active');
            document.getElementById(`${tabId}-panel`).classList.add('active');
        });
    });
}

function stressToColor(stress, maxStress) {
    const t = Math.min(1, Math.max(0, stress / maxStress));
    const colors = [
        [253, 224, 71],
        [251, 191, 36],
        [249, 115, 22],
        [220, 38, 38],
        [127, 29, 29]
    ];

    if (t <= 0.25) {
        const f = t / 0.25;
        return lerpColor(colors[0], colors[1], f);
    } else if (t <= 0.5) {
        const f = (t - 0.25) / 0.25;
        return lerpColor(colors[1], colors[2], f);
    } else if (t <= 0.75) {
        const f = (t - 0.5) / 0.25;
        return lerpColor(colors[2], colors[3], f);
    } else {
        const f = (t - 0.75) / 0.25;
        return lerpColor(colors[3], colors[4], f);
    }
}

function concentrationToColor(conc, maxConc) {
    const t = Math.min(1, Math.max(0, conc / maxConc));
    const colors = [
        [186, 230, 253],
        [56, 189, 248],
        [14, 165, 233],
        [3, 105, 161],
        [8, 47, 73]
    ];

    if (t <= 0.25) {
        const f = t / 0.25;
        return lerpColor(colors[0], colors[1], f);
    } else if (t <= 0.5) {
        const f = (t - 0.25) / 0.25;
        return lerpColor(colors[1], colors[2], f);
    } else if (t <= 0.75) {
        const f = (t - 0.5) / 0.25;
        return lerpColor(colors[2], colors[3], f);
    } else {
        const f = (t - 0.75) / 0.25;
        return lerpColor(colors[3], colors[4], f);
    }
}

function lerpColor(c1, c2, t) {
    return [
        Math.round(c1[0] + (c2[0] - c1[0]) * t),
        Math.round(c1[1] + (c2[1] - c1[1]) * t),
        Math.round(c1[2] + (c2[2] - c1[2]) * t)
    ];
}

function rgbToCss(rgb, alpha = 1) {
    return `rgba(${rgb[0]}, ${rgb[1]}, ${rgb[2]}, ${alpha})`;
}

function initStressModule() {
    const btn = document.getElementById('stress-calc-btn');
    const canvas = document.getElementById('stress-canvas');
    const ctx = canvas.getContext('2d');

    drawStressPlaceholder(ctx, canvas.width, canvas.height);

    btn.addEventListener('click', async () => {
        btn.disabled = true;
        btn.textContent = '计算中...';

        const request = {
            initial_moisture: parseFloat(document.getElementById('stress-initial-moisture').value),
            target_moisture: parseFloat(document.getElementById('stress-target-moisture').value),
            time_hours: parseFloat(document.getElementById('stress-time').value),
            young_modulus: parseFloat(document.getElementById('stress-young').value) * 1e9,
            diffusion_coefficient: 1e-9
        };

        try {
            const response = await fetch(`${API_BASE}/stress/dehydration`, {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify(request)
            });

            const result = await response.json();

            if (result.success) {
                renderStressField(ctx, canvas, result.data);
                updateStressStats(result.data);
            } else {
                alert('计算失败: ' + result.message);
            }
        } catch (e) {
            console.error('Stress calculation error:', e);
            drawStressMock(ctx, canvas);
            updateStressStats(getMockStressData());
        } finally {
            btn.disabled = false;
            btn.textContent = '计算应力场';
        }
    });
}

function drawStressPlaceholder(ctx, w, h) {
    ctx.fillStyle = '#0f172a';
    ctx.fillRect(0, 0, w, h);

    ctx.fillStyle = '#475569';
    ctx.font = '16px sans-serif';
    ctx.textAlign = 'center';
    ctx.fillText('点击"计算应力场"查看应力分布云图', w / 2, h / 2);

    ctx.strokeStyle = '#1e293b';
    ctx.lineWidth = 2;
    ctx.strokeRect(60, 80, w - 120, h - 160);
}

function renderStressField(ctx, canvas, data) {
    const w = canvas.width;
    const h = canvas.height;
    const padding = 50;

    ctx.fillStyle = '#0f172a';
    ctx.fillRect(0, 0, w, h);

    const nx = Math.sqrt(data.sigma_von_mises.length);
    const ny = nx;
    const cellW = (w - padding * 2) / (nx - 1);
    const cellH = (h - padding * 2) / (ny - 1);

    const maxStress = data.max_von_mises;

    for (let j = 0; j < ny - 1; j++) {
        for (let i = 0; i < nx - 1; i++) {
            const idx00 = j * nx + i;
            const idx10 = j * nx + i + 1;
            const idx01 = (j + 1) * nx + i;
            const idx11 = (j + 1) * nx + i + 1;

            const x = padding + i * cellW;
            const y = padding + j * cellH;

            const c00 = stressToColor(data.sigma_von_mises[idx00], maxStress);
            const c10 = stressToColor(data.sigma_von_mises[idx10], maxStress);
            const c01 = stressToColor(data.sigma_von_mises[idx01], maxStress);
            const c11 = stressToColor(data.sigma_von_mises[idx11], maxStress);

            const grad = ctx.createLinearGradient(x, y, x + cellW, y + cellH);
            grad.addColorStop(0, rgbToCss(c00));
            grad.addColorStop(1, rgbToCss(c11));

            ctx.fillStyle = grad;
            ctx.fillRect(x, y, cellW + 1, cellH + 1);
        }
    }

    if (data.danger_zones && data.danger_zones.length > 0) {
        data.danger_zones.forEach(zone => {
            const zx = padding + (zone.center_x / 0.2) * (w - padding * 2);
            const zy = padding + (zone.center_y / 0.15) * (h - padding * 2);
            const radius = Math.sqrt(zone.area_percent) * 15;

            ctx.strokeStyle = '#ef4444';
            ctx.lineWidth = 2;
            ctx.setLineDash([5, 5]);
            ctx.beginPath();
            ctx.arc(zx, zy, radius, 0, Math.PI * 2);
            ctx.stroke();
            ctx.setLineDash([]);

            ctx.fillStyle = '#ef4444';
            ctx.font = 'bold 11px sans-serif';
            ctx.textAlign = 'center';
            ctx.fillText('危险区', zx, zy - radius - 5);
        });
    }

    ctx.strokeStyle = '#334155';
    ctx.lineWidth = 2;
    ctx.strokeRect(padding, padding, w - padding * 2, h - padding * 2);

    ctx.fillStyle = '#94a3b8';
    ctx.font = '11px sans-serif';
    ctx.textAlign = 'center';
    ctx.fillText('宽度方向', w / 2, h - 20);

    ctx.save();
    ctx.translate(20, h / 2);
    ctx.rotate(-Math.PI / 2);
    ctx.fillText('厚度方向', 0, 0);
    ctx.restore();

    document.getElementById('stress-min-val').textContent = '0';
    document.getElementById('stress-max-val').textContent = (maxStress / 1e6).toFixed(1) + ' MPa';
}

function updateStressStats(data) {
    document.getElementById('max-stress-val').textContent = (data.max_von_mises / 1e6).toFixed(2) + ' MPa';
    document.getElementById('safety-factor-val').textContent = data.safety_factor.toFixed(2);
    document.getElementById('avg-sigma-x-val').textContent = (data.avg_sigma_x / 1e6).toFixed(2) + ' MPa';
    document.getElementById('danger-zones-count').textContent = data.danger_zones.length;

    const container = document.getElementById('danger-zones-container');
    if (data.danger_zones.length === 0) {
        container.innerHTML = '<p class="placeholder">无危险区域，安全</p>';
        return;
    }

    container.innerHTML = data.danger_zones.map(zone => `
        <div class="danger-zone-item ${zone.risk_level}">
            <div class="danger-zone-header">
                <span class="danger-zone-level ${zone.risk_level}">${zone.risk_level}</span>
                <span>${zone.area_percent.toFixed(1)}%</span>
            </div>
            <div class="danger-zone-details">
                <span>最大应力: ${(zone.max_stress / 1e6).toFixed(2)} MPa</span>
                <span>安全系数: ${zone.safety_factor.toFixed(2)}</span>
            </div>
        </div>
    `).join('');
}

function getMockStressData() {
    return {
        max_von_mises: 35e6,
        safety_factor: 1.14,
        avg_sigma_x: 18e6,
        danger_zones: [
            { center_x: 0.05, center_y: 0.03, area_percent: 8.5, max_stress: 42e6, safety_factor: 0.95, risk_level: 'critical' },
            { center_x: 0.15, center_y: 0.12, area_percent: 5.2, max_stress: 35e6, safety_factor: 1.14, risk_level: 'high' }
        ],
        sigma_von_mises: new Array(441).fill(0).map((_, i) => {
            const x = (i % 21) / 20;
            const y = Math.floor(i / 21) / 20;
            const edge = Math.min(x, 1 - x, y, 1 - y) * 5;
            return (1 - edge) * 35e6 + Math.random() * 5e6;
        })
    };
}

function drawStressMock(ctx, canvas) {
    const data = getMockStressData();
    renderStressField(ctx, canvas, data);
}

function initConcentrationModule() {
    const btn = document.getElementById('conc-calc-btn');
    const canvas = document.getElementById('concentration-canvas');
    const ctx = canvas.getContext('2d');

    drawConcPlaceholder(ctx, canvas.width, canvas.height);

    btn.addEventListener('click', async () => {
        btn.disabled = true;
        btn.textContent = '计算中...';

        const request = {
            surface_concentration: parseFloat(document.getElementById('conc-surface').value),
            total_time_hours: parseFloat(document.getElementById('conc-time').value),
            num_grid_x: 40,
            num_grid_y: 30
        };

        try {
            const response = await fetch(`${API_BASE}/concentration/peg`, {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify(request)
            });

            const result = await response.json();

            if (result.success) {
                renderConcentrationField(ctx, canvas, result.data);
                updateConcStats(result.data);
                drawDepthProfile(result.data);
            } else {
                alert('计算失败: ' + result.message);
            }
        } catch (e) {
            console.error('Concentration calculation error:', e);
            const mock = getMockConcData();
            renderConcentrationField(ctx, canvas, mock);
            updateConcStats(mock);
            drawDepthProfile(mock);
        } finally {
            btn.disabled = false;
            btn.textContent = '计算浓度场';
        }
    });
}

function drawConcPlaceholder(ctx, w, h) {
    ctx.fillStyle = '#0f172a';
    ctx.fillRect(0, 0, w, h);

    ctx.fillStyle = '#475569';
    ctx.font = '16px sans-serif';
    ctx.textAlign = 'center';
    ctx.fillText('点击"计算浓度场"查看PEG渗透分布', w / 2, h / 2);

    ctx.strokeStyle = '#1e293b';
    ctx.lineWidth = 2;
    ctx.strokeRect(60, 50, w - 120, h - 100);
}

function renderConcentrationField(ctx, canvas, data) {
    const w = canvas.width;
    const h = canvas.height;
    const padding = 50;

    ctx.fillStyle = '#0f172a';
    ctx.fillRect(0, 0, w, h);

    const nx = data.grid_x.length;
    const ny = data.grid_y.length;
    const cellW = (w - padding * 2) / (nx - 1);
    const cellH = (h - padding * 2) / (ny - 1);

    const maxConc = data.max_concentration;

    for (let j = 0; j < ny - 1; j++) {
        for (let i = 0; i < nx - 1; i++) {
            const idx00 = j * nx + i;
            const idx10 = j * nx + i + 1;
            const idx01 = (j + 1) * nx + i;
            const idx11 = (j + 1) * nx + i + 1;

            const c00 = data.concentration[j][i];
            const c10 = data.concentration[j][i + 1];
            const c01 = data.concentration[j + 1][i];
            const c11 = data.concentration[j + 1][i + 1];

            const x = padding + i * cellW;
            const y = padding + j * cellH;

            const color = concentrationToColor((c00 + c11) / 2, maxConc);

            ctx.fillStyle = rgbToCss(color);
            ctx.fillRect(x, y, cellW + 1, cellH + 1);
        }
    }

    if (data.penetration_front_x && data.penetration_front_x.length > 0) {
        ctx.strokeStyle = '#f59e0b';
        ctx.lineWidth = 2.5;
        ctx.setLineDash([8, 4]);
        ctx.beginPath();

        for (let i = 0; i < data.penetration_front_x.length; i++) {
            const px = padding + (data.penetration_front_x[i] / 0.2) * (w - padding * 2);
            const py = padding + (data.penetration_front_y[i] / 0.05) * (h - padding * 2);

            if (i === 0) {
                ctx.moveTo(px, py);
            } else {
                ctx.lineTo(px, py);
            }
        }
        ctx.stroke();
        ctx.setLineDash([]);

        ctx.fillStyle = '#f59e0b';
        ctx.font = 'bold 11px sans-serif';
        ctx.textAlign = 'left';
        ctx.fillText('渗透前沿', padding + 10, padding + 30);
    }

    ctx.strokeStyle = '#334155';
    ctx.lineWidth = 2;
    ctx.strokeRect(padding, padding, w - padding * 2, h - padding * 2);

    ctx.fillStyle = '#0369a1';
    ctx.fillRect(padding, padding - 3, w - padding * 2, 3);
    ctx.fillStyle = '#94a3b8';
    ctx.font = '10px sans-serif';
    ctx.textAlign = 'center';
    ctx.fillText('表面（PEG溶液）', w / 2, padding - 10);

    ctx.fillStyle = '#94a3b8';
    ctx.font = '11px sans-serif';
    ctx.textAlign = 'center';
    ctx.fillText('宽度方向 (mm)', w / 2, h - 20);

    ctx.save();
    ctx.translate(20, h / 2);
    ctx.rotate(-Math.PI / 2);
    ctx.fillText('深度方向 (mm)', 0, 0);
    ctx.restore();

    document.getElementById('conc-min-val').textContent = '0 %';
    document.getElementById('conc-max-val').textContent = maxConc.toFixed(1) + ' %';
}

function updateConcStats(data) {
    document.getElementById('avg-conc-val').textContent = data.avg_concentration.toFixed(2) + ' %';
    document.getElementById('front-depth-val').textContent = (data.penetration_depth_values ? 
        (data.penetration_depth_values[data.penetration_depth_values.length - 1] * 1000).toFixed(2) + ' mm' : '--');
    document.getElementById('darcy-vel-val').textContent = (data.darcy_velocity * 1e6).toFixed(3) + ' μm/s';
    document.getElementById('peclet-val').textContent = data.peclet_number.toFixed(2);
}

function drawDepthProfile(data) {
    const canvas = document.getElementById('depth-profile-canvas');
    const ctx = canvas.getContext('2d');
    const w = canvas.width;
    const h = canvas.height;

    ctx.fillStyle = '#0f172a';
    ctx.fillRect(0, 0, w, h);

    const padding = { left: 40, right: 20, top: 20, bottom: 30 };
    const chartW = w - padding.left - padding.right;
    const chartH = h - padding.top - padding.bottom;

    const profile = data.concentration_profile_centerline || [];
    if (profile.length < 2) return;

    const maxConc = data.max_concentration;
    const maxDepth = data.grid_y ? data.grid_y[data.grid_y.length - 1] : 0.05;

    ctx.strokeStyle = '#334155';
    ctx.lineWidth = 1;
    ctx.strokeRect(padding.left, padding.top, chartW, chartH);

    ctx.strokeStyle = '#0ea5e9';
    ctx.lineWidth = 2;
    ctx.beginPath();

    for (let i = 0; i < profile.length; i++) {
        const x = padding.left + (profile[i] / maxConc) * chartW;
        const y = padding.top + (i / (profile.length - 1)) * chartH;

        if (i === 0) {
            ctx.moveTo(x, y);
        } else {
            ctx.lineTo(x, y);
        }
    }
    ctx.stroke();

    ctx.fillStyle = '#94a3b8';
    ctx.font = '10px sans-serif';
    ctx.textAlign = 'center';
    ctx.fillText('浓度 (%)', padding.left + chartW / 2, h - 10);

    ctx.save();
    ctx.translate(15, padding.top + chartH / 2);
    ctx.rotate(-Math.PI / 2);
    ctx.fillText('深度', 0, 0);
    ctx.restore();
}

function getMockConcData() {
    const nx = 40, ny = 30;
    const conc = [];
    for (let j = 0; j < ny; j++) {
        const row = [];
        for (let i = 0; i < nx; i++) {
            const depthRatio = j / (ny - 1);
            const edgeEffect = Math.min(i / 5, (nx - 1 - i) / 5, 1);
            const val = 30 * (1 - depthRatio * depthRatio) * (0.7 + 0.3 * edgeEffect);
            row.push(Math.max(0, val));
        }
        conc.push(row);
    }

    return {
        concentration: conc,
        grid_x: new Array(nx).fill(0).map((_, i) => i * 0.2 / (nx - 1)),
        grid_y: new Array(ny).fill(0).map((_, i) => i * 0.05 / (ny - 1)),
        avg_concentration: 12.5,
        max_concentration: 30,
        darcy_velocity: 5.2e-7,
        peclet_number: 12.5,
        penetration_front_x: new Array(nx).fill(0).map((_, i) => i * 0.2 / (nx - 1)),
        penetration_front_y: new Array(nx).fill(0).map((_, i) => 0.025 + Math.sin(i * 0.5) * 0.005),
        penetration_depth_values: [0, 0.005, 0.01, 0.015, 0.02, 0.025],
        concentration_profile_centerline: new Array(ny).fill(0).map((_, i) => 30 * (1 - (i / (ny - 1)) ** 2))
    };
}

function initGPRModule() {
    const btn = document.getElementById('gpr-calc-btn');
    const sampleBtn = document.getElementById('gpr-load-sample');
    const canvas = document.getElementById('gpr-chart');
    const ctx = canvas.getContext('2d');

    drawGprPlaceholder(ctx, canvas.width, canvas.height);

    sampleBtn.addEventListener('click', () => {
        document.getElementById('gpr-data-input').value =
`0, 80
24, 72
48, 65
72, 58
96, 52
120, 47
144, 42
168, 38
192, 35
216, 32`;
    });

    btn.addEventListener('click', async () => {
        const input = document.getElementById('gpr-data-input').value.trim();
        const lines = input.split('\n').filter(l => l.trim());
        const times = [];
        const moistures = [];

        for (const line of lines) {
            const parts = line.split(/[,\s]+/);
            if (parts.length >= 2) {
                const t = parseFloat(parts[0]);
                const m = parseFloat(parts[1]);
                if (!isNaN(t) && !isNaN(m)) {
                    times.push(t);
                    moistures.push(m);
                }
            }
        }

        if (times.length < 2) {
            alert('请输入至少2个数据点');
            return;
        }

        btn.disabled = true;
        btn.textContent = '预测中...';

        const request = {
            time_hours: times,
            moisture_values: moistures,
            target_moisture: parseFloat(document.getElementById('gpr-target').value),
            confidence_level: parseFloat(document.getElementById('gpr-confidence').value),
            kernel_type: document.getElementById('gpr-kernel').value,
            optimize_hyperparams: document.getElementById('gpr-optimize').checked
        };

        try {
            const response = await fetch(`${API_BASE}/prediction/gpr-endpoint`, {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify(request)
            });

            const result = await response.json();

            if (result.success) {
                renderGprChart(ctx, canvas, result.data);
                updateGprResults(result.data, times);
            } else {
                alert('预测失败: ' + result.message);
            }
        } catch (e) {
            console.error('GPR prediction error:', e);
            const mock = getMockGprData(times, moistures);
            renderGprChart(ctx, canvas, mock);
            updateGprResults(mock, times);
        } finally {
            btn.disabled = false;
            btn.textContent = '运行预测';
        }
    });
}

function drawGprPlaceholder(ctx, w, h) {
    ctx.fillStyle = '#0f172a';
    ctx.fillRect(0, 0, w, h);

    ctx.fillStyle = '#475569';
    ctx.font = '16px sans-serif';
    ctx.textAlign = 'center';
    ctx.fillText('输入历史数据后点击"运行预测"', w / 2, h / 2);
}

function renderGprChart(ctx, canvas, data) {
    const w = canvas.width;
    const h = canvas.height;
    const padding = { left: 50, right: 30, top: 30, bottom: 40 };

    ctx.fillStyle = '#0f172a';
    ctx.fillRect(0, 0, w, h);

    const chartW = w - padding.left - padding.right;
    const chartH = h - padding.top - padding.bottom;

    const allTimes = data.predicted_curve_time || [];
    const allMeans = data.predicted_curve_mean || [];
    const allLower = data.predicted_curve_lower || [];
    const allUpper = data.predicted_curve_upper || [];
    const trainTimes = data.training_data_time || [];
    const trainMoistures = data.training_data_moisture || [];

    if (allTimes.length === 0) return;

    const tMin = allTimes[0];
    const tMax = allTimes[allTimes.length - 1];
    const mMin = Math.min(...allLower, ...trainMoistures) * 0.9;
    const mMax = Math.max(...allUpper, ...trainMoistures) * 1.1;

    ctx.strokeStyle = '#1e293b';
    ctx.lineWidth = 1;
    for (let i = 0; i <= 5; i++) {
        const y = padding.top + (i / 5) * chartH;
        ctx.beginPath();
        ctx.moveTo(padding.left, y);
        ctx.lineTo(w - padding.right, y);
        ctx.stroke();
    }

    if (data.predicted_end_time_hours && data.target_moisture !== undefined) {
        const targetY = padding.top + chartH - ((data.target_moisture - mMin) / (mMax - mMin)) * chartH;
        ctx.strokeStyle = '#10b981';
        ctx.lineWidth = 1.5;
        ctx.setLineDash([6, 4]);
        ctx.beginPath();
        ctx.moveTo(padding.left, targetY);
        ctx.lineTo(w - padding.right, targetY);
        ctx.stroke();
        ctx.setLineDash([]);

        ctx.fillStyle = '#10b981';
        ctx.font = '11px sans-serif';
        ctx.textAlign = 'right';
        ctx.fillText(`目标: ${data.target_moisture}%`, w - padding.right - 5, targetY - 5);
    }

    ctx.fillStyle = 'rgba(93, 173, 226, 0.15)';
    ctx.beginPath();
    for (let i = 0; i < allTimes.length; i++) {
        const x = padding.left + ((allTimes[i] - tMin) / (tMax - tMin)) * chartW;
        const y = padding.top + chartH - ((allUpper[i] - mMin) / (mMax - mMin)) * chartH;
        if (i === 0) ctx.moveTo(x, y);
        else ctx.lineTo(x, y);
    }
    for (let i = allTimes.length - 1; i >= 0; i--) {
        const x = padding.left + ((allTimes[i] - tMin) / (tMax - tMin)) * chartW;
        const y = padding.top + chartH - ((allLower[i] - mMin) / (mMax - mMin)) * chartH;
        ctx.lineTo(x, y);
    }
    ctx.closePath();
    ctx.fill();

    ctx.strokeStyle = '#5dade2';
    ctx.lineWidth = 2.5;
    ctx.beginPath();
    for (let i = 0; i < allTimes.length; i++) {
        const x = padding.left + ((allTimes[i] - tMin) / (tMax - tMin)) * chartW;
        const y = padding.top + chartH - ((allMeans[i] - mMin) / (mMax - mMin)) * chartH;
        if (i === 0) ctx.moveTo(x, y);
        else ctx.lineTo(x, y);
    }
    ctx.stroke();

    ctx.fillStyle = '#e74c3c';
    for (let i = 0; i < trainTimes.length; i++) {
        const x = padding.left + ((trainTimes[i] - tMin) / (tMax - tMin)) * chartW;
        const y = padding.top + chartH - ((trainMoistures[i] - mMin) / (mMax - mMin)) * chartH;
        ctx.beginPath();
        ctx.arc(x, y, 4, 0, Math.PI * 2);
        ctx.fill();
    }

    if (data.predicted_end_time_hours) {
        const endX = padding.left + ((data.predicted_end_time_hours - tMin) / (tMax - tMin)) * chartW;
        const endY = padding.top + chartH - ((data.target_moisture - mMin) / (mMax - mMin)) * chartH;

        ctx.strokeStyle = '#f59e0b';
        ctx.lineWidth = 2;
        ctx.setLineDash([5, 3]);
        ctx.beginPath();
        ctx.moveTo(endX, padding.top);
        ctx.lineTo(endX, endY);
        ctx.stroke();
        ctx.setLineDash([]);

        ctx.fillStyle = '#f59e0b';
        ctx.beginPath();
        ctx.arc(endX, endY, 6, 0, Math.PI * 2);
        ctx.fill();

        ctx.fillStyle = '#fbbf24';
        ctx.font = 'bold 11px sans-serif';
        ctx.textAlign = 'center';
        ctx.fillText(`终点: ${(data.predicted_end_time_hours).toFixed(0)}h`, endX, padding.top - 8);
    }

    ctx.strokeStyle = '#334155';
    ctx.lineWidth = 1.5;
    ctx.strokeRect(padding.left, padding.top, chartW, chartH);

    ctx.fillStyle = '#94a3b8';
    ctx.font = '11px sans-serif';
    ctx.textAlign = 'center';
    ctx.fillText('时间 (小时)', w / 2, h - 12);

    ctx.save();
    ctx.translate(18, padding.top + chartH / 2);
    ctx.rotate(-Math.PI / 2);
    ctx.fillText('含水率 (%)', 0, 0);
    ctx.restore();

    const legendY = padding.top + 10;
    ctx.fillStyle = '#94a3b8';
    ctx.font = '11px sans-serif';
    ctx.textAlign = 'right';

    ctx.fillStyle = 'rgba(93, 173, 226, 0.3)';
    ctx.fillRect(w - padding.right - 110, legendY, 14, 14);
    ctx.fillStyle = '#94a3b8';
    ctx.fillText('置信区间', w - padding.right - 120, legendY + 11);

    ctx.fillStyle = '#5dade2';
    ctx.fillRect(w - padding.right - 110, legendY + 22, 14, 3);
    ctx.fillStyle = '#94a3b8';
    ctx.fillText('预测均值', w - padding.right - 120, legendY + 28);

    ctx.fillStyle = '#e74c3c';
    ctx.beginPath();
    ctx.arc(w - padding.right - 103, legendY + 47, 4, 0, Math.PI * 2);
    ctx.fill();
    ctx.fillStyle = '#94a3b8';
    ctx.fillText('实测数据', w - padding.right - 120, legendY + 51);
}

function updateGprResults(data, trainTimes) {
    const endTime = data.predicted_end_time_hours;
    const currentTime = trainTimes[trainTimes.length - 1] || 0;
    const remaining = endTime - currentTime;

    document.getElementById('gpr-end-time').textContent = endTime.toFixed(1) + ' h';
    document.getElementById('gpr-confidence-interval').textContent =
        `${data.confidence_lower_hours.toFixed(0)} ~ ${data.confidence_upper_hours.toFixed(0)} h (95%置信)`;

    document.getElementById('gpr-remaining').textContent =
        remaining > 0 ? remaining.toFixed(1) + ' h' : '已达到';

    document.getElementById('gpr-r2').textContent = (data.r_squared || 0).toFixed(3);
    document.getElementById('gpr-uncertainty').textContent = '±' + (data.uncertainty_at_target || 0).toFixed(2);
}

function getMockGprData(times, moistures) {
    const target = 15;
    const predTimes = [];
    const predMean = [];
    const predLower = [];
    const predUpper = [];

    const lastT = times[times.length - 1];
    const lastM = moistures[moistures.length - 1];
    const endT = lastT * 3;

    for (let t = 0; t <= endT; t += endT / 100) {
        predTimes.push(t);
        const m = lastM - (lastM - target) * Math.min(1, (t / endT) * 0.9);
        const uncertainty = 2 + t / 100;
        predMean.push(m);
        predLower.push(m - uncertainty);
        predUpper.push(m + uncertainty);
    }

    const endIdx = predMean.findIndex(m => m <= target);
    const endTime = endIdx > 0 ? predTimes[endIdx] : endT;

    return {
        predicted_end_time_hours: endTime,
        confidence_lower_hours: endTime * 0.8,
        confidence_upper_hours: endTime * 1.3,
        remaining_hours: endTime - lastT,
        r_squared: 0.987,
        uncertainty_at_target: 2.5,
        predicted_curve_time: predTimes,
        predicted_curve_mean: predMean,
        predicted_curve_lower: predLower,
        predicted_curve_upper: predUpper,
        training_data_time: times,
        training_data_moisture: moistures,
        target_moisture: target
    };
}

function initStabilityModule() {
    const btn = document.getElementById('stab-calc-btn');
    const canvas = document.getElementById('stability-chart');
    const ctx = canvas.getContext('2d');

    drawStabPlaceholder(ctx, canvas.width, canvas.height);

    btn.addEventListener('click', async () => {
        btn.disabled = true;
        btn.textContent = '评估中...';

        const request = {
            initial_moisture: parseFloat(document.getElementById('stab-initial').value),
            low_moisture: parseFloat(document.getElementById('stab-low').value),
            high_moisture: parseFloat(document.getElementById('stab-high').value),
            agent_concentration: parseFloat(document.getElementById('stab-conc').value),
            num_cycles: parseInt(document.getElementById('stab-cycles').value),
            cycle_duration_hours: 168,
            compare_without_reinforcement: document.getElementById('stab-compare').checked
        };

        try {
            const response = await fetch(`${API_BASE}/stability/dimensional`, {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify(request)
            });

            const result = await response.json();

            if (result.success) {
                const data = result.data.with_reinforcement || result.data;
                renderStabilityChart(ctx, canvas, data, result.data.without_reinforcement);
                updateStabilityResults(data, result.data.without_reinforcement);
            } else {
                alert('评估失败: ' + result.message);
            }
        } catch (e) {
            console.error('Stability assessment error:', e);
            const mockWith = getMockStabilityData(true);
            const mockWithout = document.getElementById('stab-compare').checked ? getMockStabilityData(false) : null;
            renderStabilityChart(ctx, canvas, mockWith, mockWithout);
            updateStabilityResults(mockWith, mockWithout);
        } finally {
            btn.disabled = false;
            btn.textContent = '评估稳定性';
        }
    });
}

function drawStabPlaceholder(ctx, w, h) {
    ctx.fillStyle = '#0f172a';
    ctx.fillRect(0, 0, w, h);

    ctx.fillStyle = '#475569';
    ctx.font = '16px sans-serif';
    ctx.textAlign = 'center';
    ctx.fillText('点击"评估稳定性"查看尺寸变化曲线', w / 2, h / 2);
}

function renderStabilityChart(ctx, canvas, dataWith, dataWithout) {
    const w = canvas.width;
    const h = canvas.height;
    const padding = { left: 60, right: 30, top: 30, bottom: 40 };

    ctx.fillStyle = '#0f172a';
    ctx.fillRect(0, 0, w, h);

    const chartW = w - padding.left - padding.right;
    const chartH = h - padding.top - padding.bottom;

    const series = dataWith.time_series || [];
    if (series.length === 0) return;

    const allValues = series.map(s => s.dimensional_change_percent);
    if (dataWithout && dataWithout.time_series) {
        allValues.push(...dataWithout.time_series.map(s => s.dimensional_change_percent));
    }

    const vMin = Math.min(...allValues) * 1.2;
    const vMax = Math.max(...allValues) * 1.2;
    const tMax = series[series.length - 1].time_hours;

    ctx.strokeStyle = '#1e293b';
    ctx.lineWidth = 1;
    for (let i = 0; i <= 5; i++) {
        const y = padding.top + (i / 5) * chartH;
        ctx.beginPath();
        ctx.moveTo(padding.left, y);
        ctx.lineTo(w - padding.right, y);
        ctx.stroke();
    }

    ctx.strokeStyle = '#475569';
    ctx.lineWidth = 1;
    ctx.setLineDash([4, 4]);
    const zeroY = padding.top + chartH - ((0 - vMin) / (vMax - vMin)) * chartH;
    ctx.beginPath();
    ctx.moveTo(padding.left, zeroY);
    ctx.lineTo(w - padding.right, zeroY);
    ctx.stroke();
    ctx.setLineDash([]);

    if (dataWithout && dataWithout.time_series) {
        ctx.strokeStyle = '#ef4444';
        ctx.lineWidth = 2;
        ctx.beginPath();
        for (let i = 0; i < dataWithout.time_series.length; i++) {
            const d = dataWithout.time_series[i];
            const x = padding.left + (d.time_hours / tMax) * chartW;
            const y = padding.top + chartH - ((d.dimensional_change_percent - vMin) / (vMax - vMin)) * chartH;
            if (i === 0) ctx.moveTo(x, y);
            else ctx.lineTo(x, y);
        }
        ctx.stroke();
    }

    ctx.strokeStyle = '#10b981';
    ctx.lineWidth = 2.5;
    ctx.beginPath();
    for (let i = 0; i < series.length; i++) {
        const d = series[i];
        const x = padding.left + (d.time_hours / tMax) * chartW;
        const y = padding.top + chartH - ((d.dimensional_change_percent - vMin) / (vMax - vMin)) * chartH;
        if (i === 0) ctx.moveTo(x, y);
        else ctx.lineTo(x, y);
    }
    ctx.stroke();

    if (dataWith.cycle_summaries) {
        ctx.fillStyle = '#10b981';
        dataWith.cycle_summaries.forEach((cs, i) => {
            const t = (i + 1) * (tMax / dataWith.cycle_summaries.length);
            const x = padding.left + (t / tMax) * chartW;
            const y = padding.top + chartH - ((cs.residual_deformation_percent - vMin) / (vMax - vMin)) * chartH;
            ctx.beginPath();
            ctx.arc(x, y, 3, 0, Math.PI * 2);
            ctx.fill();
        });
    }

    ctx.strokeStyle = '#334155';
    ctx.lineWidth = 1.5;
    ctx.strokeRect(padding.left, padding.top, chartW, chartH);

    ctx.fillStyle = '#94a3b8';
    ctx.font = '11px sans-serif';
    ctx.textAlign = 'center';
    ctx.fillText('时间 (小时)', w / 2, h - 12);

    ctx.save();
    ctx.translate(18, padding.top + chartH / 2);
    ctx.rotate(-Math.PI / 2);
    ctx.fillText('尺寸变化率 (%)', 0, 0);
    ctx.restore();

    const legendY = padding.top + 15;
    ctx.font = '11px sans-serif';
    ctx.textAlign = 'right';

    ctx.strokeStyle = '#10b981';
    ctx.lineWidth = 2.5;
    ctx.beginPath();
    ctx.moveTo(w - padding.right - 110, legendY + 7);
    ctx.lineTo(w - padding.right - 80, legendY + 7);
    ctx.stroke();
    ctx.fillStyle = '#94a3b8';
    ctx.fillText('加固后', w - padding.right - 120, legendY + 11);

    if (dataWithout) {
        ctx.strokeStyle = '#ef4444';
        ctx.lineWidth = 2;
        ctx.beginPath();
        ctx.moveTo(w - padding.right - 110, legendY + 32);
        ctx.lineTo(w - padding.right - 80, legendY + 32);
        ctx.stroke();
        ctx.fillStyle = '#94a3b8';
        ctx.fillText('未加固', w - padding.right - 120, legendY + 36);
    }
}

function updateStabilityResults(data, dataWithout) {
    const rating = data.stability_rating || 'good';
    const score = data.stability_score || 0;

    document.getElementById('stab-rating').textContent = score.toFixed(0);
    document.getElementById('stab-rating').className = 'rating-score ' + rating;
    document.getElementById('stab-rating-text').textContent = getRatingText(rating);

    document.getElementById('stab-swing').textContent = data.total_dimensional_swing.toFixed(3) + ' %';
    document.getElementById('stab-residual').textContent = data.final_residual_deformation_percent.toFixed(3) + ' %';
    document.getElementById('stab-10yr').textContent = data.long_term_prediction_10yr.toFixed(3) + ' %';
    document.getElementById('stab-50yr').textContent = data.long_term_prediction_50yr.toFixed(3) + ' %';
    document.getElementById('stab-improvement').textContent = (data.improvement_factor || 1).toFixed(2) + 'x';
    document.getElementById('stab-failure').textContent = data.cycles_to_failure.toFixed(0) + ' 次';

    const listContainer = document.getElementById('cycle-summary-list');
    if (data.cycle_summaries && data.cycle_summaries.length > 0) {
        listContainer.innerHTML = data.cycle_summaries.map(cs => `
            <div class="cycle-summary-item">
                <span class="cycle-number">第${cs.cycle_number}循环</span>
                <span class="cycle-details">摆幅: ${cs.dimensional_swing_percent.toFixed(3)}%</span>
            </div>
        `).join('');
    }
}

function getRatingText(rating) {
    const map = {
        excellent: '优秀 - 长期稳定',
        good: '良好 - 可安全使用',
        fair: '一般 - 需定期检查',
        poor: '较差 - 建议加固',
        critical: '危险 - 急需处理'
    };
    return map[rating] || rating;
}

function getMockStabilityData(withReinforcement) {
    const numCycles = 5;
    const stepsPerCycle = 40;
    const cycleDuration = 168;
    const series = [];

    const baseShrink = withReinforcement ? 0.8 : 1.5;
    const hysteresis = withReinforcement ? 0.15 : 0.4;
    let currentDim = 0;
    let cumulativeResidual = 0;

    for (let cycle = 0; cycle < numCycles; cycle++) {
        const shrinkThisCycle = baseShrink * (1 + cycle * 0.05);

        for (let i = 0; i < stepsPerCycle / 2; i++) {
            const t = cycle * cycleDuration + i * (cycleDuration / (stepsPerCycle / 2));
            const progress = i / (stepsPerCycle / 2 - 1);
            const sCurve = 1 / (1 + Math.exp(-4 * (progress - 0.5)));
            currentDim = -shrinkThisCycle * sCurve + cumulativeResidual;

            series.push({
                time_hours: t,
                dimensional_change_percent: currentDim,
                moisture: 50 - 42 * sCurve
            });
        }

        for (let i = 0; i < stepsPerCycle / 2; i++) {
            const t = cycle * cycleDuration + (stepsPerCycle / 2 + i) * (cycleDuration / stepsPerCycle);
            const progress = i / (stepsPerCycle / 2 - 1);
            const sCurve = 1 / (1 + Math.exp(-4 * (progress - 0.5)));
            const expand = shrinkThisCycle * (1 - hysteresis) * sCurve;
            currentDim = -shrinkThisCycle + expand + cumulativeResidual;

            series.push({
                time_hours: t,
                dimensional_change_percent: currentDim,
                moisture: 8 + 57 * sCurve
            });
        }

        cumulativeResidual -= hysteresis * shrinkThisCycle * 0.5;
    }

    const cycleSummaries = [];
    for (let i = 0; i < numCycles; i++) {
        cycleSummaries.push({
            cycle_number: i + 1,
            dimensional_swing_percent: baseShrink * (1 + i * 0.05) * (1 - hysteresis * 0.5),
            residual_deformation_percent: cumulativeResidual * ((i + 1) / numCycles)
        });
    }

    return {
        time_series: series,
        cycle_summaries: cycleSummaries,
        total_dimensional_swing: baseShrink * (1 + (numCycles - 1) * 0.05),
        final_residual_deformation_percent: cumulativeResidual,
        stability_rating: withReinforcement ? 'good' : 'fair',
        stability_score: withReinforcement ? 78 : 52,
        long_term_prediction_10yr: cumulativeResidual * 3,
        long_term_prediction_50yr: cumulativeResidual * 8,
        improvement_factor: withReinforcement ? 1.8 : 1.0,
        cycles_to_failure: withReinforcement ? 120 : 50,
        equivalent_years: numCycles / 4
    };
}
