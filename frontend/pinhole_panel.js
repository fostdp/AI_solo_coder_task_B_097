const PinholePanel = (function () {
    let API_URL = '';
    let canvas, ctx;
    let animTimer = null;
    let currentResult = null;
    let sweepData = [];

    function init(apiUrl) {
        API_URL = apiUrl;
        canvas = document.getElementById('pinhole-canvas');
        if (canvas) {
            ctx = canvas.getContext('2d');
        }
        document.getElementById('btn-pinhole-sim').addEventListener('click', runSimulation);
        document.getElementById('btn-pinhole-sweep').addEventListener('click', runSweep);
        resizeCanvas();
        window.addEventListener('resize', () => {
            setTimeout(resizeCanvas, 100);
        });
    }

    function resizeCanvas() {
        if (!canvas) return;
        const container = document.getElementById('pinhole-canvas-container');
        if (!container) return;
        const dpr = window.devicePixelRatio || 1;
        canvas.width = container.clientWidth * dpr;
        canvas.height = container.clientHeight * dpr;
        canvas.style.width = container.clientWidth + 'px';
        canvas.style.height = container.clientHeight + 'px';
        ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
    }

    async function runSimulation() {
        const btn = document.getElementById('btn-pinhole-sim');
        btn.textContent = '仿真中...';
        btn.disabled = true;
        try {
            const gauge = parseFloat(document.getElementById('pinhole-gauge').value) || 40.0;
            const diameter = parseFloat(document.getElementById('pinhole-diameter').value) || 1.0;
            const alt = parseFloat(document.getElementById('pinhole-alt').value) || 26.0;
            const distance = parseFloat(document.getElementById('pinhole-distance').value) || 40.0;
            const temp = parseFloat(document.getElementById('pinhole-temp').value) || 5.0;
            const pressure = parseFloat(document.getElementById('pinhole-pressure').value) || 1013.25;

            const resp = await fetch(`${API_URL}/api/v2/pinhole/optimize`, {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({
                    gauge_height_chi: gauge,
                    pinhole_diameter_cun: diameter,
                    sun_altitude: alt,
                    screen_distance_chi: distance,
                    temperature: temp,
                    pressure: pressure,
                }),
            });
            const result = await resp.json();
            if (result.success && result.data) {
                currentResult = result.data;
                renderResult(result.data);
                drawPinholeDiagram(result.data);
            }
        } catch (e) {
            console.error('[PinholePanel] 仿真失败:', e);
        }
        btn.textContent = '针孔成像仿真';
        btn.disabled = false;
    }

    async function runSweep() {
        const btn = document.getElementById('btn-pinhole-sweep');
        btn.textContent = '扫描中...';
        btn.disabled = true;
        sweepData = [];
        try {
            const gauge = parseFloat(document.getElementById('pinhole-gauge').value) || 40.0;
            const alt = parseFloat(document.getElementById('pinhole-alt').value) || 26.0;
            const distance = parseFloat(document.getElementById('pinhole-distance').value) || 40.0;
            const temp = parseFloat(document.getElementById('pinhole-temp').value) || 5.0;
            const pressure = parseFloat(document.getElementById('pinhole-pressure').value) || 1013.25;

            const resp = await fetch(`${API_URL}/api/v2/pinhole/scan`, {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({
                    gauge_height_chi: gauge,
                    min_diameter_cun: 0.1,
                    max_diameter_cun: 5.0,
                    steps: 20,
                }),
            });
            const result = await resp.json();
            if (result.success && result.data) {
                for (const [d, blur] of result.data) {
                    const detailResp = await fetch(`${API_URL}/api/v2/pinhole/optimize`, {
                        method: 'POST',
                        headers: { 'Content-Type': 'application/json' },
                        body: JSON.stringify({
                            gauge_height_chi: gauge,
                            pinhole_diameter_cun: d,
                            sun_altitude: alt,
                            screen_distance_chi: distance,
                            temperature: temp,
                            pressure: pressure,
                        }),
                    });
                    const detail = await detailResp.json();
                    if (detail.success && detail.data) {
                        sweepData.push(detail.data);
                    }
                }
            }
            drawSweepChart();
        } catch (e) {
            console.error('[PinholePanel] 扫描失败:', e);
        }
        btn.textContent = '针孔直径扫描';
        btn.disabled = false;
    }

    function renderResult(r) {
        const container = document.getElementById('pinhole-results');
        if (!container) return;
        container.innerHTML = `
            <div class="pinhole-grid">
                <div class="data-card"><div class="data-label">太阳像直径</div><div class="data-value">${r.sun_image_diameter_cun.toFixed(3)} <span class="data-unit">寸</span></div></div>
                <div class="data-card"><div class="data-label">几何模糊</div><div class="data-value">${r.geometric_blur_cun.toFixed(3)} <span class="data-unit">寸</span></div></div>
                <div class="data-card"><div class="data-label">衍射模糊</div><div class="data-value">${r.diffraction_blur_cun.toFixed(4)} <span class="data-unit">寸</span></div></div>
                <div class="data-card"><div class="data-label">总模糊</div><div class="data-value">${r.total_blur_cun.toFixed(4)} <span class="data-unit">寸</span></div></div>
                <div class="data-card"><div class="data-label">最优孔径</div><div class="data-value" style="color:#44ff88">${r.optimal_diameter_cun.toFixed(3)} <span class="data-unit">寸</span></div></div>
                <div class="data-card"><div class="data-label">角分辨率</div><div class="data-value">${r.altitude_resolution_arcmin.toFixed(2)} <span class="data-unit">角分</span></div></div>
                <div class="data-card"><div class="data-label">信噪比</div><div class="data-value">${r.signal_to_noise_ratio.toFixed(2)}</div></div>
                <div class="data-card"><div class="data-label">边缘锐度</div><div class="data-value">${r.shadow_edge_sharpness.toFixed(3)}</div></div>
            </div>`;
    }

    function drawPinholeDiagram(r) {
        if (!ctx || !canvas) return;
        const container = document.getElementById('pinhole-canvas-container');
        if (!container) return;
        const w = container.clientWidth;
        const h = container.clientHeight;

        ctx.clearRect(0, 0, w, h);
        ctx.fillStyle = 'rgba(15, 20, 40, 0.95)';
        ctx.fillRect(0, 0, w, h);

        const barX = w * 0.25;
        const barTopY = h * 0.1;
        const barBottomY = h * 0.85;
        const barH = barBottomY - barTopY;
        const pinholeY = barTopY + 10;
        const screenX = w * 0.7;

        ctx.fillStyle = '#c9a959';
        ctx.fillRect(barX - 4, barTopY, 8, barH);

        ctx.fillStyle = '#333';
        ctx.fillRect(barX - 6, pinholeY, 12, 3);

        ctx.strokeStyle = 'rgba(255, 220, 100, 0.3)';
        ctx.lineWidth = 1;
        ctx.beginPath();
        ctx.moveTo(0, barTopY - 30);
        ctx.lineTo(barX - 3, pinholeY + 1);
        ctx.stroke();
        ctx.beginPath();
        ctx.moveTo(0, barTopY + 10);
        ctx.lineTo(barX - 3, pinholeY + 1);
        ctx.stroke();

        ctx.strokeStyle = 'rgba(255, 220, 100, 0.5)';
        ctx.lineWidth = 2;
        ctx.beginPath();
        ctx.moveTo(barX + 6, pinholeY + 1);
        ctx.lineTo(screenX, h * 0.35);
        ctx.stroke();
        ctx.beginPath();
        ctx.moveTo(barX + 6, pinholeY + 1);
        ctx.lineTo(screenX, h * 0.65);
        ctx.stroke();

        ctx.fillStyle = 'rgba(255, 240, 150, 0.15)';
        ctx.beginPath();
        ctx.moveTo(barX + 6, pinholeY + 1);
        ctx.lineTo(screenX, h * 0.35);
        ctx.lineTo(screenX, h * 0.65);
        ctx.closePath();
        ctx.fill();

        const sunImgH = Math.min((r.sun_image_diameter_cun / 5) * (h * 0.3), h * 0.25);
        ctx.fillStyle = 'rgba(255, 230, 100, 0.6)';
        ctx.fillRect(screenX - 4, h * 0.5 - sunImgH / 2, 8, sunImgH);

        ctx.fillStyle = '#d4b866';
        ctx.font = 'bold 11px Consolas, "Microsoft YaHei"';
        ctx.fillText('表杆', barX - 20, barBottomY + 18);
        ctx.fillText('针孔', barX + 14, pinholeY + 5);
        ctx.fillText('像屏', screenX - 12, h * 0.5 + sunImgH / 2 + 18);

        ctx.fillStyle = '#8899aa';
        ctx.font = '10px Consolas';
        ctx.fillText(`像径=${r.sun_image_diameter_cun.toFixed(2)}寸`, screenX + 14, h * 0.5);
        ctx.fillText(`最优孔径=${r.optimal_diameter_cun.toFixed(2)}寸`, w * 0.35, h * 0.92);
    }

    function drawSweepChart() {
        if (!ctx || !canvas || sweepData.length === 0) return;
        const container = document.getElementById('pinhole-canvas-container');
        if (!container) return;
        const w = container.clientWidth;
        const h = container.clientHeight;

        ctx.clearRect(0, 0, w, h);
        ctx.fillStyle = 'rgba(15, 20, 40, 0.95)';
        ctx.fillRect(0, 0, w, h);

        const padL = 50, padR = 20, padT = 30, padB = 50;
        const chartW = w - padL - padR;
        const chartH = h - padT - padB;

        ctx.strokeStyle = 'rgba(201, 169, 89, 0.3)';
        ctx.lineWidth = 1;
        ctx.beginPath();
        ctx.moveTo(padL, padT);
        ctx.lineTo(padL, padT + chartH);
        ctx.lineTo(padL + chartW, padT + chartH);
        ctx.stroke();

        const maxBlur = Math.max(...sweepData.map(d => d.total_blur_cun));
        const maxD = Math.max(...sweepData.map(d => d.pinhole_diameter_cun));

        ctx.strokeStyle = '#ff6b6b';
        ctx.lineWidth = 2;
        ctx.beginPath();
        sweepData.forEach((d, i) => {
            const x = padL + (d.pinhole_diameter_cun / maxD) * chartW;
            const y = padT + chartH - (d.total_blur_cun / maxBlur) * chartH;
            if (i === 0) ctx.moveTo(x, y); else ctx.lineTo(x, y);
        });
        ctx.stroke();

        ctx.strokeStyle = '#88ddff';
        ctx.lineWidth = 1;
        ctx.setLineDash([4, 4]);
        ctx.beginPath();
        sweepData.forEach((d, i) => {
            const x = padL + (d.pinhole_diameter_cun / maxD) * chartW;
            const y = padT + chartH - (d.geometric_blur_cun / maxBlur) * chartH;
            if (i === 0) ctx.moveTo(x, y); else ctx.lineTo(x, y);
        });
        ctx.stroke();

        ctx.strokeStyle = '#44ff88';
        ctx.beginPath();
        sweepData.forEach((d, i) => {
            const x = padL + (d.pinhole_diameter_cun / maxD) * chartW;
            const y = padT + chartH - (d.diffraction_blur_cun / maxBlur) * chartH;
            if (i === 0) ctx.moveTo(x, y); else ctx.lineTo(x, y);
        });
        ctx.stroke();
        ctx.setLineDash([]);

        const opt = sweepData.reduce((a, b) => a.total_blur_cun < b.total_blur_cun ? a : b);
        const optX = padL + (opt.pinhole_diameter_cun / maxD) * chartW;
        const optY = padT + chartH - (opt.total_blur_cun / maxBlur) * chartH;
        ctx.beginPath();
        ctx.arc(optX, optY, 6, 0, Math.PI * 2);
        ctx.fillStyle = '#ffdd44';
        ctx.fill();
        ctx.strokeStyle = '#ffdd44';
        ctx.lineWidth = 1;
        ctx.setLineDash([3, 3]);
        ctx.beginPath();
        ctx.moveTo(optX, optY);
        ctx.lineTo(optX, padT + chartH);
        ctx.stroke();
        ctx.setLineDash([]);

        ctx.fillStyle = '#8899aa';
        ctx.font = '10px Consolas';
        ctx.fillText('孔径(寸)', padL + chartW / 2, h - 8);
        ctx.save();
        ctx.translate(12, padT + chartH / 2);
        ctx.rotate(-Math.PI / 2);
        ctx.fillText('模糊(寸)', 0, 0);
        ctx.restore();

        ctx.fillStyle = '#ff6b6b'; ctx.fillText('━ 总模糊', padL + 10, padT + 14);
        ctx.fillStyle = '#88ddff'; ctx.fillText('┅ 几何模糊', padL + 80, padT + 14);
        ctx.fillStyle = '#44ff88'; ctx.fillText('━ 衍射模糊', padL + 160, padT + 14);
        ctx.fillStyle = '#ffdd44'; ctx.fillText(`⬤ 最优=${opt.pinhole_diameter_cun.toFixed(2)}寸`, padL + 240, padT + 14);
    }

    return { init, runSimulation, runSweep };
})();

if (typeof window !== 'undefined') {
    window.PinholePanel = PinholePanel;
}
