const App = (function () {
    let APP_CONFIG = null;
    let ws = null;
    let currentMeasurement = null;
    let currentSimulation = null;
    let showParticles = true;
    let showLabels = true;

    function resolveEndpoints() {
        const apiCfg = (APP_CONFIG && APP_CONFIG.api) || {};
        let base = apiCfg.fallback_base_url || 'http://localhost:3000';
        if (apiCfg.base_url_auto && typeof window !== 'undefined' && window.location && window.location.origin) {
            const lo = window.location;
            if (lo.protocol === 'http:' || lo.protocol === 'https:') {
                if (lo.hostname !== 'localhost' && lo.hostname !== '127.0.0.1') {
                    base = lo.origin;
                }
            }
        }
        const wsProto = base.startsWith('https:') ? 'wss:' : 'ws:';
        const hostPart = base.replace(/^https?:\/\//, '');
        const wsPath = (apiCfg.ws_path) || '/ws';
        return {
            API_URL: base,
            WS_URL: `${wsProto}//${hostPart}${wsPath}`,
        };
    }

    async function loadConfig() {
        try {
            const resp = await fetch('./config.json', { cache: 'no-cache' });
            if (resp.ok) {
                APP_CONFIG = await resp.json();
                console.log('[App] 加载前端配置成功');
                return APP_CONFIG;
            }
            console.warn('[App] config.json 加载失败，使用内置默认配置');
        } catch (e) {
            console.warn('[App] config.json 读取异常:', e);
        }
        APP_CONFIG = {
            api: { base_url_auto: true, fallback_base_url: 'http://localhost:3000', ws_path: '/ws' },
            render: { chi_scale: 0.5, gauge_height_chi: 40, ruler_length_chi: 120, default_sun_altitude: 30, default_sun_azimuth: 180, particle_count_3d: 200, particle_count_2d: 100 },
            tier_config: {
                low:    { shadow_map_size: 1024, dpr_cap: 1.5, antialias: false, shadow_bias: -0.0008, shadow_normal_bias: 0.04, shadow_radius: 2 },
                medium: { shadow_map_size: 2048, dpr_cap: 2.0, antialias: true,  shadow_bias: -0.0005, shadow_normal_bias: 0.06, shadow_radius: 4 },
                high:   { shadow_map_size: 4096, dpr_cap: 2.5, antialias: true,  shadow_bias: -0.0003, shadow_normal_bias: 0.06, shadow_radius: 6 },
            },
            colors: {
                scene_background: '0x0a0a1a',
                fog_color: '0x0a0a1a',
                ground: '0x5a4a3a',
                gauge: '0xc9a959',
                gauge_top: '0xd4b866',
                ruler: '0xd4c4a4',
                sun_light: '0xfff0c8',
                hemisphere_sky: '0xfff1d6',
                hemisphere_ground: '0x443322',
                beam: '0xfff5d4',
            },
            pcf_soft_shadow: { blur_px: 1.5, layers: [] },
        };
        return APP_CONFIG;
    }

    function updateMeasurementUI(m) {
        document.getElementById('m-time').textContent = formatTime(m.measurement_time);
        document.getElementById('m-gauge').innerHTML = `${m.gauge_height.toFixed(2)} <span class="data-unit">尺</span>`;
        document.getElementById('m-shadow').innerHTML = `${m.shadow_length.toFixed(2)} <span class="data-unit">尺</span>`;
        document.getElementById('m-shadow-cun').innerHTML = `${(m.shadow_length * 10).toFixed(1)} <span class="data-unit">寸</span>`;
        document.getElementById('m-alt').innerHTML = `${m.sun_altitude.toFixed(2)} <span class="data-unit">°</span>`;
        document.getElementById('m-azi').innerHTML = `${m.sun_azimuth.toFixed(1)} <span class="data-unit">°</span>`;
        document.getElementById('m-refr').textContent = m.atmospheric_refraction.toFixed(6);
        document.getElementById('m-tp').innerHTML = `${m.temperature.toFixed(1)}°C / ${m.pressure.toFixed(0)}hPa`;
        currentMeasurement = m;
        if (window.Gnomon3D) {
            window.Gnomon3D.updateSunPosition(m.sun_altitude, m.sun_azimuth);
            window.Gnomon3D.updateShadow(m.shadow_length);
        }
        if (window.ShadowPanel) {
            window.ShadowPanel.setMeasurement(m);
        }
    }

    function updateSimulationUI(s) {
        document.getElementById('s-true-alt').textContent = s.true_sun_altitude.toFixed(4);
        document.getElementById('s-app-alt').textContent = s.apparent_sun_altitude.toFixed(4);
        document.getElementById('s-refr-corr').textContent = s.atmospheric_refraction_correction.toFixed(2);
        document.getElementById('s-curv-corr').textContent = s.earth_curvature_correction.toFixed(6);
        document.getElementById('s-theo-shadow').textContent = s.theoretical_shadow_length.toFixed(4);
        document.getElementById('s-refr-shadow').textContent = s.refracted_shadow_length.toFixed(4);
        const dev = s.shadow_deviation;
        const devEl = document.getElementById('s-deviation');
        devEl.textContent = dev.toFixed(3);
        devEl.style.color = Math.abs(dev) >= 1 ? '#ff6b6b' : '#c9a959';
        document.getElementById('s-solstice').textContent = s.winter_solstice_moment ? formatTime(s.winter_solstice_moment) : '非冬至期';
        currentSimulation = s;
    }

    function addAlert(alert) {
        const list = document.getElementById('alert-list');
        if (list.querySelector('div[style*="color: #666"]')) {
            list.innerHTML = '';
        }
        const level = (alert.alert_level || 'warning').toLowerCase();
        const item = document.createElement('div');
        item.className = `alert-item ${level}`;
        item.innerHTML = `
            <div class="alert-time">${formatTime(alert.alert_time)} | ${alert.alert_level}</div>
            <div class="alert-msg">${alert.message}</div>
        `;
        list.insertBefore(item, list.firstChild);
        while (list.children.length > 20) {
            list.removeChild(list.lastChild);
        }
    }

    function showMonteCarloResult(r) {
        document.getElementById('mc-result').style.display = 'block';
        document.getElementById('mc-count').textContent = r.simulation_count;
        document.getElementById('mc-shadow-std').textContent = r.shadow_length_std.toFixed(4);
        document.getElementById('mc-shadow-ci').textContent = `${r.shadow_length_95ci_low.toFixed(3)}, ${r.shadow_length_95ci_high.toFixed(3)}`;
        document.getElementById('mc-sol-std').textContent = r.solstice_time_std.toFixed(2);
        document.getElementById('mc-combined').textContent = r.combined_uncertainty.toFixed(6);
        document.getElementById('mc-expanded').textContent = r.expanded_uncertainty.toFixed(6);
    }

    function formatTime(isoStr) {
        if (!isoStr) return '--';
        try {
            const d = new Date(isoStr);
            return d.toLocaleString('zh-CN', { timeZone: 'Asia/Shanghai' });
        } catch {
            return isoStr;
        }
    }

    function connectWebSocket() {
        const { WS_URL } = resolveEndpoints();
        try {
            ws = new WebSocket(WS_URL);
        } catch (e) {
            console.error('WS连接失败:', e);
            setTimeout(connectWebSocket, 3000);
            return;
        }
        ws.onopen = () => {
            document.getElementById('ws-status').classList.add('connected');
            document.getElementById('ws-text').textContent = 'WebSocket: 已连接';
        };
        ws.onmessage = (event) => {
            try {
                const msg = JSON.parse(event.data);
                if (msg.message_type === 'measurement') {
                    updateMeasurementUI(msg.data);
                } else if (msg.message_type === 'simulation') {
                    updateSimulationUI(msg.data);
                } else if (msg.message_type === 'alert') {
                    addAlert(msg.data);
                }
            } catch (e) {
                console.error('解析WS消息失败:', e);
            }
        };
        ws.onclose = () => {
            document.getElementById('ws-status').classList.remove('connected');
            document.getElementById('ws-text').textContent = 'WebSocket: 断开重连...';
            setTimeout(connectWebSocket, 3000);
        };
        ws.onerror = () => {
            if (ws) ws.close();
        };
    }

    async function loadInitialData() {
        const { API_URL } = resolveEndpoints();
        try {
            const resp = await fetch(`${API_URL}/api/measurements/latest`);
            const result = await resp.json();
            if (result.success && result.data && result.data.length > 0) {
                updateMeasurementUI(result.data[0]);
            }
        } catch (e) {
            console.error('加载初始数据失败:', e);
        }
    }

    async function runMonteCarlo() {
        const btn = document.getElementById('btn-monte-carlo');
        btn.textContent = '分析中...';
        btn.disabled = true;
        const { API_URL } = resolveEndpoints();
        try {
            const resp = await fetch(`${API_URL}/api/analyze/monte-carlo`, {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({
                    simulation_count: 10000,
                    gauge_height_error_std: 0.01,
                    refraction_error_std: 5.0,
                    confidence_level: 0.95,
                }),
            });
            const result = await resp.json();
            if (result.success) {
                showMonteCarloResult(result.data);
            }
        } catch (e) {
            console.error('蒙特卡洛分析失败:', e);
        }
        btn.textContent = '运行蒙特卡洛误差分析';
        btn.disabled = false;
    }

    function updateClock() {
        const now = new Date();
        document.getElementById('current-time').textContent = now.toLocaleString('zh-CN', { timeZone: 'Asia/Shanghai' });
    }

    async function bootstrap() {
        const cfg = await loadConfig();
        if (window.Gnomon3D) {
            window.Gnomon3D.init(cfg);
        } else {
            console.error('[App] Gnomon3D 未加载');
        }
        if (window.ShadowPanel) {
            window.ShadowPanel.init(cfg);
        } else {
            console.error('[App] ShadowPanel 未加载');
        }
        connectWebSocket();
        loadInitialData();
        setInterval(updateClock, 1000);
        updateClock();

        const { API_URL } = resolveEndpoints();
        if (window.DynastyPanel) {
            window.DynastyPanel.init(API_URL);
        }
        if (window.MeridianPanel) {
            window.MeridianPanel.init(API_URL);
        }
        if (window.PinholePanel) {
            window.PinholePanel.init(API_URL);
        }
        if (window.VirtualExperience) {
            window.VirtualExperience.init(API_URL);
        }

        document.getElementById('btn-monte-carlo').addEventListener('click', runMonteCarlo);
        document.getElementById('toggle-particles').addEventListener('click', (e) => {
            showParticles = !showParticles;
            e.target.textContent = `粒子: ${showParticles ? '开' : '关'}`;
            if (window.Gnomon3D) window.Gnomon3D.setShowParticles(showParticles);
            if (window.ShadowPanel) window.ShadowPanel.setShowParticles(showParticles);
        });
        document.getElementById('toggle-labels').addEventListener('click', (e) => {
            showLabels = !showLabels;
            e.target.textContent = `标注: ${showLabels ? '开' : '关'}`;
            if (window.ShadowPanel) window.ShadowPanel.setShowLabels(showLabels);
        });
    }

    document.addEventListener('DOMContentLoaded', bootstrap);

    return {
        getConfig: () => APP_CONFIG,
        getEndpoints: resolveEndpoints,
        updateMeasurementUI,
        updateSimulationUI,
        addAlert,
        showMonteCarloResult,
    };
})();
