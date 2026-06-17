const VirtualExperience = (function () {
    let API_URL = '';
    let currentResult = null;
    let canvas, ctx;
    let animTimer = null;
    let lightParticles = [];
    let timeAcceleration = 1;
    let timeLoopTimer = null;
    let timeSeriesData = null;

    function init(apiUrl) {
        API_URL = apiUrl;
        canvas = document.getElementById('virtual-canvas');
        if (canvas) {
            ctx = canvas.getContext('2d');
        }

        document.getElementById('btn-virtual-sim').addEventListener('click', runSimulation);
        document.getElementById('virtual-gauge').addEventListener('input', onSliderChange);
        document.getElementById('virtual-hour').addEventListener('input', onSliderChange);
        document.getElementById('virtual-month').addEventListener('input', onParamChange);
        document.getElementById('virtual-day').addEventListener('input', onParamChange);
        document.getElementById('virtual-latitude').addEventListener('input', onParamChange);

        document.querySelectorAll('.btn-time').forEach(btn => {
            btn.addEventListener('click', function() {
                const speed = parseInt(this.dataset.speed);
                setTimeAcceleration(speed);
                document.querySelectorAll('.btn-time').forEach(b => b.classList.remove('active'));
                this.classList.add('active');
            });
        });

        for (let i = 0; i < 60; i++) {
            lightParticles.push({
                x: Math.random(),
                y: Math.random() * 0.3,
                speed: 0.002 + Math.random() * 0.003,
                size: 1 + Math.random() * 2,
                alpha: 0.2 + Math.random() * 0.3,
            });
        }

        resizeCanvas();
        window.addEventListener('resize', () => setTimeout(resizeCanvas, 100));
        startAnimation();
        fetchTimeSeries();
    }

    function setTimeAcceleration(speed) {
        timeAcceleration = speed;
        const info = document.getElementById('virtual-time-info');
        if (!info) return;

        if (speed === 0) {
            info.textContent = '⏸️ 已暂停';
            if (timeLoopTimer) {
                clearInterval(timeLoopTimer);
                timeLoopTimer = null;
            }
        } else if (speed === 1) {
            info.textContent = '⏱️ 实时：1× 实时模式（每帧推进3秒）';
        } else if (speed < 60) {
            info.textContent = `⏱️ ${speed}× 加速模式（每帧推进${speed * 3}秒）`;
        } else if (speed === 60) {
            info.textContent = '⏱️ 60× 加速模式（每帧推进3分钟）';
        } else if (speed < 1440) {
            info.textContent = `⏱️ ${speed}× 加速模式（每帧推进${(speed * 3 / 60).toFixed(1)}分钟）`;
        } else {
            info.textContent = '⏱️ 1天/秒 超快模式（可观察全年日出日落变化）';
        }

        if (speed > 0) {
            startLoopTimer();
        }
    }

    function startLoopTimer() {
        if (timeLoopTimer) clearInterval(timeLoopTimer);
        timeLoopTimer = setInterval(() => {
            const hourSlider = document.getElementById('virtual-hour');
            if (!hourSlider) return;

            let currentHour = parseFloat(hourSlider.value) || 12.0;
            let advanceMinutes;

            if (timeAcceleration <= 5) {
                advanceMinutes = timeAcceleration * 0.05;
            } else if (timeAcceleration <= 60) {
                advanceMinutes = timeAcceleration * 0.1;
            } else if (timeAcceleration <= 600) {
                advanceMinutes = timeAcceleration * 0.2;
            } else {
                advanceMinutes = 60 * 24;
            }

            currentHour += advanceMinutes / 60.0;
            while (currentHour >= 24.0) {
                currentHour -= 24.0;
                let dayInput = document.getElementById('virtual-day');
                let monthInput = document.getElementById('virtual-month');
                if (dayInput && monthInput) {
                    let day = parseInt(dayInput.value);
                    let month = parseInt(monthInput.value);
                    day++;
                    const daysInMonth = [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
                    const maxDay = daysInMonth[month - 1] || 31;
                    if (day > maxDay) {
                        day = 1;
                        month++;
                        if (month > 12) month = 1;
                        monthInput.value = month;
                    }
                    dayInput.value = day;
                }
            }

            hourSlider.value = currentHour.toFixed(2);
            document.getElementById('virtual-hour-val').textContent = currentHour.toFixed(1);
            runSimulation();
        }, 200);
    }

    function resizeCanvas() {
        if (!canvas) return;
        const container = document.getElementById('virtual-canvas-container');
        if (!container) return;
        const dpr = window.devicePixelRatio || 1;
        canvas.width = container.clientWidth * dpr;
        canvas.height = container.clientHeight * dpr;
        canvas.style.width = container.clientWidth + 'px';
        canvas.style.height = container.clientHeight + 'px';
        ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
    }

    function onSliderChange() {
        updateSliderLabels();
        runSimulation();
    }

    function onParamChange() {
        runSimulation();
        fetchTimeSeries();
    }

    function updateSliderLabels() {
        const gauge = document.getElementById('virtual-gauge').value;
        const hour = document.getElementById('virtual-hour').value;
        document.getElementById('virtual-gauge-val').textContent = gauge;
        document.getElementById('virtual-hour-val').textContent = hour;
    }

    async function fetchTimeSeries() {
        try {
            const gauge = parseFloat(document.getElementById('virtual-gauge').value) || 8.0;
            const month = parseInt(document.getElementById('virtual-month').value) || 12;
            const day = parseInt(document.getElementById('virtual-day').value) || 22;
            const latitude = parseFloat(document.getElementById('virtual-latitude').value) || 34.49;
            const temp = parseFloat(document.getElementById('virtual-temp').value) || 5.0;
            const pressure = parseFloat(document.getElementById('virtual-pressure').value) || 1013.0;
            const humidity = parseFloat(document.getElementById('virtual-humidity').value) || 50.0;

            const resp = await fetch(`${API_URL}/api/virtual/time_series`, {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({
                    gauge_height_chi: gauge,
                    latitude,
                    month,
                    day,
                    hour: 12.0,
                    temperature: temp,
                    pressure,
                    humidity,
                    time_acceleration: 60,
                }),
            });
            const data = await resp.json();
            if (data.success && data.data) {
                timeSeriesData = data.data;
            }
        } catch (e) {
        }
    }

    async function runSimulation() {
        try {
            const gauge = parseFloat(document.getElementById('virtual-gauge').value) || 8.0;
            const month = parseInt(document.getElementById('virtual-month').value) || 12;
            const day = parseInt(document.getElementById('virtual-day').value) || 22;
            const hour = parseFloat(document.getElementById('virtual-hour').value) || 12.0;
            const latitude = parseFloat(document.getElementById('virtual-latitude').value) || 34.49;
            const temp = parseFloat(document.getElementById('virtual-temp').value) || 5.0;
            const pressure = parseFloat(document.getElementById('virtual-pressure').value) || 1013.0;
            const humidity = parseFloat(document.getElementById('virtual-humidity').value) || 50.0;

            const resp = await fetch(`${API_URL}/api/virtual/experience`, {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({
                    gauge_height_chi: gauge,
                    latitude,
                    month,
                    day,
                    hour,
                    temperature: temp,
                    pressure,
                    humidity,
                    time_acceleration: timeAcceleration || 1,
                }),
            });
            const result = await resp.json();
            if (result.success && result.data) {
                currentResult = result.data;
                renderResult(result.data);
            }
        } catch (e) {
            console.error('[VirtualExperience] 仿真失败:', e);
        }
    }

    function renderResult(r) {
        const container = document.getElementById('virtual-results');
        if (!container) return;

        const shadowText = r.is_daytime ?
            `${r.refracted_shadow_chi.toFixed(2)} 尺 (${r.shadow_length_cun.toFixed(1)} 寸)` : '日落/夜间';
        const altColor = r.sun_altitude > 30 ? '#44ff88' : r.sun_altitude > 10 ? '#ffaa44' : '#ff6666';

        let timeSeriesInfo = '';
        if (timeSeriesData && timeSeriesData.sunrise_hour && timeSeriesData.sunset_hour) {
            timeSeriesInfo = `
                <div style="margin-top:8px;padding:8px;background:rgba(68,255,136,0.05);border-radius:4px;border:1px solid rgba(68,255,136,0.15)">
                    <div style="color:#8899aa;font-size:10px;margin-bottom:4px">📊 今日天文数据</div>
                    <div style="display:grid;grid-template-columns:1fr 1fr 1fr;gap:6px;font-size:10px">
                        <div><span style="color:#8899aa">日出</span><br><b style="color:#c9a959">${timeSeriesData.sunrise_hour.toFixed(2)}时</b></div>
                        <div><span style="color:#8899aa">日落</span><br><b style="color:#c9a959">${timeSeriesData.sunset_hour.toFixed(2)}时</b></div>
                        <div><span style="color:#8899aa">昼长</span><br><b style="color:#c9a959">${timeSeriesData.total_daylight_hours.toFixed(1)}小时</b></div>
                    </div>
                </div>`;
        }

        container.innerHTML = `
            <div class="virtual-grid">
                <div class="data-card"><div class="data-label">太阳高度角</div><div class="data-value" style="color:${altColor}">${r.sun_altitude.toFixed(2)}<span class="data-unit">°</span></div></div>
                <div class="data-card"><div class="data-label">太阳方位角</div><div class="data-value">${r.sun_azimuth.toFixed(1)}<span class="data-unit">°</span></div></div>
                <div class="data-card"><div class="data-label">影长</div><div class="data-value" style="color:#c9a959;font-size:16px">${shadowText}</div></div>
                <div class="data-card"><div class="data-label">蒙气差修正</div><div class="data-value">${r.refraction_correction_arcsec.toFixed(2)}<span class="data-unit">"</span></div></div>
                <div class="data-card"><div class="data-label">赤纬</div><div class="data-value">${r.sun_declination.toFixed(2)}<span class="data-unit">°</span></div></div>
                <div class="data-card"><div class="data-label">时差</div><div class="data-value">${r.equation_of_time_min.toFixed(1)}<span class="data-unit">分</span></div></div>
                <div class="data-card"><div class="data-label">本地真太阳时</div><div class="data-value">${r.local_solar_time_hour.toFixed(2)}<span class="data-unit">时</span></div></div>
                <div class="data-card"><div class="data-label">时间加速</div><div class="data-value">${r.time_acceleration_applied}<span class="data-unit">×</span></div></div>
            </div>
            ${timeSeriesInfo}
            <div class="dynasty-hint" style="margin-top:10px;padding:10px;background:rgba(201,169,89,0.1);border-radius:6px;border:1px solid rgba(201,169,89,0.3)">
                <div style="color:#c9a959;font-weight:bold;margin-bottom:4px">🏛️ ${r.dynasty_hint}圭表</div>
                <div style="color:#8899aa;font-size:11px;line-height:1.5">${r.historical_note}</div>
            </div>`;
    }

    function startAnimation() {
        function loop() {
            animTimer = requestAnimationFrame(loop);
            draw();
        }
        loop();
    }

    function draw() {
        if (!ctx || !canvas) return;
        const container = document.getElementById('virtual-canvas-container');
        if (!container) return;
        const w = container.clientWidth;
        const h = container.clientHeight;

        ctx.clearRect(0, 0, w, h);

        const grad = ctx.createLinearGradient(0, 0, 0, h);
        grad.addColorStop(0, 'rgba(30, 40, 70, 0.9)');
        grad.addColorStop(1, 'rgba(20, 30, 50, 0.95)');
        ctx.fillStyle = grad;
        ctx.fillRect(0, 0, w, h);

        const groundY = h * 0.75;
        const gaugeX = w * 0.2;
        const gaugeH = h * 0.55;
        const gaugeW = 10;

        ctx.fillStyle = 'rgba(100, 80, 60, 0.3)';
        ctx.fillRect(0, groundY, w, h - groundY);

        ctx.strokeStyle = 'rgba(201, 169, 89, 0.3)';
        ctx.lineWidth = 1;
        for (let i = 0; i < 20; i++) {
            const x = w * 0.25 + i * (w * 0.7 / 20);
            ctx.beginPath();
            ctx.moveTo(x, groundY);
            ctx.lineTo(x, groundY + 6);
            ctx.stroke();
            if (i % 5 === 0) {
                ctx.fillStyle = 'rgba(201, 169, 89, 0.5)';
                ctx.font = '9px Consolas';
                ctx.fillText(i * 5 + '尺', x - 6, groundY + 18);
            }
        }

        if (currentResult && currentResult.is_daytime) {
            const alt = currentResult.sun_altitude;
            const dirY = -Math.sin(alt * Math.PI / 180);
            const dirX = Math.cos(alt * Math.PI / 180);
            lightParticles.forEach(p => {
                p.x += dirX * p.speed;
                p.y += -dirY * p.speed * 0.4;
                if (p.x > 1 || p.y > 0.9) {
                    p.x = 0.05 + Math.random() * 0.15;
                    p.y = 0;
                }
                ctx.beginPath();
                ctx.arc(p.x * w, p.y * h, p.size, 0, Math.PI * 2);
                ctx.fillStyle = `rgba(255, 240, 150, ${p.alpha})`;
                ctx.fill();
            });
        }

        const gaugeGrad = ctx.createLinearGradient(gaugeX - gaugeW / 2, groundY - gaugeH, gaugeX + gaugeW / 2, groundY);
        gaugeGrad.addColorStop(0, '#c9a959');
        gaugeGrad.addColorStop(0.5, '#d4b866');
        gaugeGrad.addColorStop(1, '#8b7333');
        ctx.fillStyle = gaugeGrad;
        ctx.fillRect(gaugeX - gaugeW / 2, groundY - gaugeH, gaugeW, gaugeH);

        if (currentResult && currentResult.is_daytime) {
            const shadowLen = currentResult.refracted_shadow_chi;
            const maxDisplay = 100;
            const shadowPx = Math.min((shadowLen / maxDisplay) * (w * 0.7), w * 0.65);
            const shadowStartX = gaugeX + gaugeW / 2;
            const shadowEndX = shadowStartX + shadowPx;

            ctx.fillStyle = 'rgba(0, 0, 0, 0.5)';
            ctx.fillRect(shadowStartX, groundY - 1, shadowPx, 20);

            ctx.save();
            ctx.filter = 'blur(1px)';
            ctx.fillStyle = 'rgba(0, 0, 0, 0.25)';
            ctx.fillRect(shadowStartX, groundY, shadowPx, 15);
            ctx.restore();

            ctx.strokeStyle = 'rgba(255, 220, 100, 0.4)';
            ctx.lineWidth = 2;
            ctx.setLineDash([4, 4]);
            ctx.beginPath();
            const topY = groundY - gaugeH;
            ctx.moveTo(gaugeX, topY);
            ctx.lineTo(shadowEndX, groundY);
            ctx.stroke();
            ctx.setLineDash([]);

            ctx.fillStyle = '#d4b866';
            ctx.font = 'bold 12px Consolas';
            ctx.fillText(`影长: ${shadowLen.toFixed(1)}尺`, shadowStartX + 8, groundY - 8);

            ctx.beginPath();
            ctx.arc(shadowEndX, groundY, 5, 0, Math.PI * 2);
            ctx.fillStyle = '#ff6b6b';
            ctx.fill();
        }

        if (timeSeriesData && timeSeriesData.points && timeSeriesData.points.length > 1) {
            drawTimeSeriesMiniChart(w, h);
        }

        ctx.fillStyle = 'rgba(201, 169, 89, 0.9)';
        ctx.font = 'bold 10px Consolas, "Microsoft YaHei"';
        const heightChi = currentResult ? currentResult.gauge_height_chi : 8;
        ctx.fillText(`表高${heightChi.toFixed(0)}尺`, gaugeX - 20, groundY - gaugeH - 8);

        if (timeAcceleration > 0) {
            ctx.fillStyle = 'rgba(68, 255, 136, 0.8)';
            ctx.font = 'bold 10px Consolas';
            const speedLabel = timeAcceleration >= 1440 ? '1天/秒' : `${timeAcceleration}×`;
            ctx.fillText(`⏩ ${speedLabel}`, w - 60, 20);
        }
    }

    function drawTimeSeriesMiniChart(w, h) {
        const chartW = w * 0.3;
        const chartH = h * 0.2;
        const chartX = w - chartW - 10;
        const chartY = h - chartH - 10;

        ctx.save();
        ctx.fillStyle = 'rgba(20, 30, 50, 0.7)';
        ctx.fillRect(chartX, chartY, chartW, chartH);
        ctx.strokeStyle = 'rgba(201, 169, 89, 0.3)';
        ctx.strokeRect(chartX, chartY, chartW, chartH);

        const points = timeSeriesData.points;
        let minAlt = 90, maxAlt = -90;
        points.forEach(p => {
            if (p.sun_altitude < minAlt) minAlt = p.sun_altitude;
            if (p.sun_altitude > maxAlt) maxAlt = p.sun_altitude;
        });
        const altRange = Math.max(maxAlt - minAlt, 1);

        ctx.strokeStyle = 'rgba(68, 255, 136, 0.7)';
        ctx.lineWidth = 1.5;
        ctx.beginPath();
        points.forEach((p, i) => {
            const x = chartX + (p.hour / 24.0) * chartW;
            const y = chartY + chartH - ((p.sun_altitude - minAlt) / altRange) * chartH;
            if (i === 0) ctx.moveTo(x, y);
            else ctx.lineTo(x, y);
        });
        ctx.stroke();

        if (currentResult) {
            const curX = chartX + ((parseFloat(document.getElementById('virtual-hour').value) || 12) / 24.0) * chartW;
            ctx.strokeStyle = 'rgba(255, 107, 107, 0.8)';
            ctx.lineWidth = 1;
            ctx.beginPath();
            ctx.moveTo(curX, chartY);
            ctx.lineTo(curX, chartY + chartH);
            ctx.stroke();
        }

        ctx.fillStyle = 'rgba(136, 153, 170, 0.8)';
        ctx.font = '8px Consolas';
        ctx.fillText('太阳高度日变化', chartX + 4, chartY + 10);
        ctx.restore();
    }

    return { init, runSimulation };
})();

if (typeof window !== 'undefined') {
    window.VirtualExperience = VirtualExperience;
}
