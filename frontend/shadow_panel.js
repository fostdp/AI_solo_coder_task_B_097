const ShadowPanel = (function () {
    let ctx;
    let lightParticles = [];
    let showParticles = true;
    let showLabels = true;
    let currentMeasurement = null;
    let config = null;
    let animTimer = null;

    function init(userConfig) {
        const canvas = document.getElementById('shadow-canvas');
        if (!canvas) {
            console.error('[ShadowPanel] 未找到 shadow-canvas 元素');
            return;
        }
        ctx = canvas.getContext('2d');
        config = userConfig || {};

        resize();
        window.addEventListener('resize', () => {
            if (window.__shadowPanelResizeTimer) clearTimeout(window.__shadowPanelResizeTimer);
            window.__shadowPanelResizeTimer = setTimeout(resize, 100);
        });

        const particleCount = (config.render && config.render.particle_count_2d) || 100;
        lightParticles = [];
        for (let i = 0; i < particleCount; i++) {
            lightParticles.push({
                x: Math.random(),
                y: Math.random() * 0.3,
                speed: 0.002 + Math.random() * 0.004,
                size: 1 + Math.random() * 2,
                alpha: 0.3 + Math.random() * 0.4,
            });
        }

        start();
    }

    function resize() {
        const canvas = document.getElementById('shadow-canvas');
        const container = document.getElementById('shadow-canvas-container');
        if (!canvas || !container || !ctx) return;
        const dpr = window.devicePixelRatio || 1;
        canvas.width = container.clientWidth * dpr;
        canvas.height = container.clientHeight * dpr;
        canvas.style.width = container.clientWidth + 'px';
        canvas.style.height = container.clientHeight + 'px';
        ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
    }

    function start() {
        function loop() {
            animTimer = requestAnimationFrame(loop);
            draw();
        }
        loop();
    }

    function setMeasurement(m) {
        currentMeasurement = m;
    }

    function setShowParticles(v) {
        showParticles = !!v;
    }

    function setShowLabels(v) {
        showLabels = !!v;
    }

    function draw() {
        if (!ctx) return;
        const container = document.getElementById('shadow-canvas-container');
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
        const gaugeX = w * 0.15;
        const gaugeH = h * 0.6;
        const gaugeW = 12;

        ctx.fillStyle = 'rgba(100, 80, 60, 0.3)';
        ctx.fillRect(0, groundY, w, h - groundY);

        ctx.strokeStyle = 'rgba(201, 169, 89, 0.3)';
        ctx.lineWidth = 1;
        for (let i = 0; i < 20; i++) {
            const x = w * 0.2 + i * (w * 0.75 / 20);
            ctx.beginPath();
            ctx.moveTo(x, groundY);
            ctx.lineTo(x, groundY + 8);
            ctx.stroke();
            if (i % 5 === 0) {
                ctx.save();
                ctx.fillStyle = 'rgba(201, 169, 89, 0.6)';
                ctx.font = '10px Consolas';
                ctx.fillText(i * 5 + '尺', x - 8, groundY + 22);
                ctx.restore();
            }
        }

        if (showParticles) {
            const alt = currentMeasurement ? currentMeasurement.sun_altitude : ((config && config.render && config.render.default_sun_altitude) || 30);
            const azi = currentMeasurement ? currentMeasurement.sun_azimuth : ((config && config.render && config.render.default_sun_azimuth) || 180);
            const dirX = Math.cos(alt * Math.PI / 180) * (azi > 180 ? 1 : -1);
            const dirY = -Math.sin(alt * Math.PI / 180);
            lightParticles.forEach(p => {
                p.x += dirX * p.speed;
                p.y += -dirY * p.speed * 0.5;
                if (p.x > 1 || p.y > 0.9) {
                    p.x = 0.1 + Math.random() * 0.2;
                    p.y = 0;
                }
                ctx.save();
                ctx.beginPath();
                ctx.arc(p.x * w, p.y * h, p.size, 0, Math.PI * 2);
                ctx.fillStyle = `rgba(255, 240, 150, ${p.alpha})`;
                ctx.fill();
                ctx.restore();
            });
        }

        const gaugeGrad = ctx.createLinearGradient(gaugeX - gaugeW / 2, groundY - gaugeH, gaugeX + gaugeW / 2, groundY);
        gaugeGrad.addColorStop(0, '#c9a959);
        gaugeGrad.addColorStop(0.5, '#d4b866);
        gaugeGrad.addColorStop(1, '#8b7333);
        ctx.fillStyle = gaugeGrad;
        ctx.fillRect(gaugeX - gaugeW / 2, groundY - gaugeH, gaugeW, gaugeH);

        ctx.strokeStyle = '#000';
        ctx.lineWidth = 1;
        for (let i = 0; i <= 40; i++) {
            const y = groundY - (gaugeH * i / 40);
            const ww = i % 5 === 0 ? 10 : 5;
            ctx.beginPath();
            ctx.moveTo(gaugeX - gaugeW / 2, y);
            ctx.lineTo(gaugeX - gaugeW / 2 - ww, y);
            ctx.stroke();
        }

        if (currentMeasurement) {
            const shadowLen = currentMeasurement.shadow_length;
            const shadowPx = (shadowLen / 100) * (w * 0.75;
            const shadowStartX = gaugeX + gaugeW / 2;
            const shadowEndX = Math.min(shadowStartX + shadowPx, w - 10);

            const pcfLayers = (config.pcf_soft_shadow && config.pcf_soft_shadow.layers) || [
                { dx: 0, dy: 0, a: 0.35, yOff: 0 },
                { dx: 0.8, dy: 0.4, a: 0.12, yOff: 1 },
                { dx: -0.8, dy: -0.4, a: 0.12, yOff: -1 },
                { dx: 0.6, dy: -0.5, a: 0.10, yOff: 0 },
                { dx: -0.6, dy: 0.5, a: 0.10, yOff: 1 },
                { dx: 1.2, dy: 0, a: 0.09, yOff: 1 },
                { dx: -1.2, dy: 0, a: 0.09, yOff: -1 },
                { dx: 0, dy: 1.0, a: 0.08, yOff: 2 },
            ];
            pcfLayers.forEach(layer => {
                const sx = shadowStartX + layer.dx * 2;
                const ex = shadowEndX + layer.dx * 3;
                const gy = groundY + layer.yOff;
                const sh = 25 + Math.abs(layer.dy) * 4;
                const sg = ctx.createLinearGradient(sx, gy, ex, gy);
                sg.addColorStop(0, `rgba(0, 0, 0, ${0.85 * layer.a * 3})`);
                sg.addColorStop(0.5, `rgba(0, 0, 0, ${0.5 * layer.a * 3})`);
                sg.addColorStop(1, `rgba(0, 0, 0, ${0.15 * layer.a * 3})`);
                ctx.fillStyle = sg;
                ctx.fillRect(sx, gy - 1, ex - sx, sh);
            });

            const blurPx = (config.pcf_soft_shadow && config.pcf_soft_shadow.blur_px) || 1.5;
            ctx.save();
            ctx.filter = `blur(${blurPx}px)`;
            const sg = ctx.createLinearGradient(shadowStartX, groundY, shadowEndX, groundY);
            sg.addColorStop(0, 'rgba(0, 0, 0, 0.55)');
            sg.addColorStop(0.5, 'rgba(0, 0, 0, 0.3)');
            sg.addColorStop(1, 'rgba(0, 0, 0, 0.08)');
            ctx.fillStyle = sg;
            ctx.fillRect(shadowStartX, groundY, shadowEndX - shadowStartX, 25);
            ctx.restore();

            const alt = currentMeasurement.sun_altitude;
            const topY = groundY - gaugeH;
            const rayLen = shadowEndX - gaugeX;
            const endY = groundY - gaugeH - rayLen * Math.tan(alt * Math.PI / 180);

            ctx.save();
            ctx.strokeStyle = 'rgba(255, 230, 130, 0.15)';
            ctx.lineWidth = 6;
            ctx.lineCap = 'round';
            ctx.beginPath();
            ctx.moveTo(gaugeX, topY);
            ctx.lineTo(shadowEndX, groundY);
            ctx.stroke();
            ctx.restore();

            ctx.strokeStyle = 'rgba(255, 220, 100, 0.45)';
            ctx.lineWidth = 2;
            ctx.setLineDash([5, 5]);
            ctx.beginPath();
            ctx.moveTo(gaugeX, topY);
            ctx.lineTo(shadowEndX, groundY);
            ctx.stroke();
            ctx.setLineDash([]);

            if (showLabels) {
                drawLabels(gaugeX, topY, groundY, alt, shadowLen, shadowStartX, shadowEndX);
            }
        }

        ctx.save();
        ctx.shadowColor = 'rgba(0, 0, 0, 0.7)';
        ctx.shadowBlur = 2;
        ctx.fillStyle = 'rgba(201, 169, 89, 0.95)';
        ctx.font = 'bold 11px Consolas, "Microsoft YaHei"';
        ctx.fillText('圭(表高40尺)', gaugeX - 35, groundY - gaugeH - 10);
        ctx.fillText('圭尺', w * 0.6, groundY + 38);
        ctx.restore();
    }

    function drawLabels(gaugeX, topY, groundY, alt, shadowLen, shadowStartX, shadowEndX) {
        ctx.save();
        ctx.shadowColor = 'rgba(0, 0, 0, 0.9)';
        ctx.shadowBlur = 3;
        ctx.shadowOffsetX = 1;
        ctx.shadowOffsetY = 1;
        ctx.fillStyle = 'rgba(255, 225, 120, 0.95)';
        ctx.font = 'bold 13px Consolas, "Microsoft YaHei", monospace';
        ctx.textBaseline = 'alphabetic';
        ctx.fillText(`影长: ${shadowLen.toFixed(2)} 尺 (${(shadowLen * 10).toFixed(1)} 寸)`, shadowStartX + 10, groundY - 8);
        ctx.fillText(`太阳高度: ${alt.toFixed(2)}°`, shadowStartX + 10, groundY + 42);
        ctx.restore();

        ctx.save();
        ctx.lineCap = 'round';
        ctx.lineJoin = 'round';
        ctx.beginPath();
        ctx.moveTo(gaugeX, topY);
        ctx.arc(gaugeX, topY, 30, Math.PI / 2, Math.PI / 2 + (90 - alt) * Math.PI / 180, false);
        ctx.strokeStyle = 'rgba(201, 169, 89, 0.35)';
        ctx.lineWidth = 6;
        ctx.stroke();
        ctx.strokeStyle = 'rgba(201, 169, 89, 0.9)';
        ctx.lineWidth = 2;
        ctx.stroke();
        ctx.restore();

        ctx.save();
        ctx.shadowColor = 'rgba(0, 0, 0, 0.7)';
        ctx.shadowBlur = 2;
        ctx.fillStyle = '#d4b866';
        ctx.font = 'bold 12px Consolas';
        ctx.fillText(`${alt.toFixed(1)}°`, gaugeX + 35, topY + 15);
        ctx.restore();

        ctx.save();
        ctx.beginPath();
        ctx.arc(shadowEndX, groundY, 8, 0, Math.PI * 2);
        ctx.fillStyle = 'rgba(255, 107, 107, 0.25)';
        ctx.fill();
        ctx.beginPath();
        ctx.arc(shadowEndX, groundY, 5, 0, Math.PI * 2);
        ctx.fillStyle = '#ff6b6b';
        ctx.shadowColor = 'rgba(255, 107, 107, 0.8)';
        ctx.shadowBlur = 5;
        ctx.fill();
        ctx.restore();

        ctx.save();
        ctx.strokeStyle = 'rgba(255, 107, 107, 0.3)';
        ctx.lineWidth = 3;
        ctx.lineCap = 'round';
        ctx.beginPath();
        ctx.moveTo(shadowEndX, groundY);
        ctx.lineTo(shadowEndX, groundY - 35);
        ctx.stroke();
        ctx.strokeStyle = '#ff6b6b';
        ctx.lineWidth = 1;
        ctx.stroke();
        ctx.restore();

        ctx.save();
        ctx.shadowColor = 'rgba(0, 0, 0, 0.7)';
        ctx.shadowBlur = 2;
        ctx.fillStyle = '#ff7b7b';
        ctx.font = 'bold 11px Consolas';
        ctx.textAlign = 'center';
        ctx.fillText(`影端`, shadowEndX, groundY - 40);
        ctx.textAlign = 'start';
        ctx.restore();
    }

    function dispose() {
        if (animTimer) cancelAnimationFrame(animTimer);
    }

    return {
        init,
        setMeasurement,
        setShowParticles,
        setShowLabels,
        dispose,
        resize,
    };
})();

if (typeof window !== 'undefined') {
    window.ShadowPanel = ShadowPanel;
}
