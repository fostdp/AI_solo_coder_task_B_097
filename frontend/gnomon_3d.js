const Gnomon3D = (function () {
    let scene, camera, renderer, guibiaoGroup, sunLight, sunLightHelper;
    let shadowMesh, gaugeMesh, rulerMesh;
    let particles = [];
    let showParticles = true;
    let config = null;
    let tier = 'medium';
    let animationTimer = null;

    function isMobileDevice() {
        if (typeof navigator !== 'undefined') {
            return /Android|webOS|iPhone|iPad|iPod|BlackBerry|IEMobile|Opera Mini/i.test(navigator.userAgent) ||
                   (window.matchMedia && window.matchMedia('(pointer: coarse)').matches);
        }
        return false;
    }

    function getDeviceTier() {
        const mobile = isMobileDevice();
        const cores = (typeof navigator !== 'undefined' && navigator.hardwareConcurrency) || 4;
        const dpr = window.devicePixelRatio || 1;
        if (mobile || cores <= 4 || dpr >= 2.5) return 'low';
        if (cores <= 6 || dpr >= 2) return 'medium';
        return 'high';
    }

    function init(userConfig) {
        config = userConfig || {};
        tier = getDeviceTier();
        const container = document.getElementById('three-container');
        if (!container) {
            console.error('[Gnomon3D] 未找到 three-container 元素');
            return;
        }
        const width = container.clientWidth;
        const height = container.clientHeight;
        const render = config.tier_config ? config.tier_config[tier] : null;
        const tc = render || (config.tier_config && config.tier_config.medium) || {
            shadow_map_size: 2048,
            dpr_cap: Math.min(window.devicePixelRatio, 2.0),
            antialias: true,
            shadow_bias: -0.0005,
            shadow_normal_bias: 0.06,
            shadow_radius: 4
        };

        const chiScale = (config.render && config.render.chi_scale) || 0.5;
        const gaugeHeight = (config.render && config.render.gauge_height_chi) || 40;
        const rulerLength = (config.render && config.render.ruler_length_chi) || 120;
        const colors = config.colors || {
            scene_background: '0x0a0a1a',
            fog_color: '0x0a0a1a',
            ground: '0x5a4a3a',
            gauge: '0xc9a959',
            gauge_top: '0xd4b866',
            ruler: '0xd4c4a4',
            sun_light: '0xfff0c8',
            hemisphere_sky: '0xfff1d6',
            hemisphere_ground: '0x443322',
            beam: '0xfff5d4'
        };
        window.__CHI_SCALE__ = chiScale;
        window.__GAUGE_HEIGHT__ = gaugeHeight;
        window.__RULER_LENGTH__ = rulerLength;

        scene = new THREE.Scene();
        scene.background = new THREE.Color(parseInt(colors.scene_background, 16) || 0x0a0a1a);
        scene.fog = new THREE.Fog(parseInt(colors.fog_color, 16) || 0x0a0a1a, 80, 200);

        camera = new THREE.PerspectiveCamera(50, width / height, 0.1, 1000);
        camera.position.set(60, 50, 80);
        camera.lookAt(0, 10, 0);

        renderer = new THREE.WebGLRenderer({
            antialias: !!tc.antialias,
            alpha: false,
            powerPreference: "high-performance",
        });
        renderer.setSize(width, height);
        renderer.setPixelRatio(tc.dpr_cap || Math.min(window.devicePixelRatio, 2));
        renderer.outputEncoding = THREE.sRGBEncoding;
        renderer.toneMapping = THREE.ACESFilmicToneMapping;
        renderer.toneMappingExposure = 1.1;
        renderer.shadowMap.enabled = true;
        renderer.shadowMap.type = THREE.PCFSoftShadowMap;
        container.insertBefore(renderer.domElement, container.firstChild);

        const ambientLight = new THREE.AmbientLight(0x404050, 0.5);
        scene.add(ambientLight);

        const hemiLight = new THREE.HemisphereLight(
            parseInt(colors.hemisphere_sky, 16) || 0xfff1d6,
            parseInt(colors.hemisphere_ground, 16) || 0x443322,
            0.45
        );
        scene.add(hemiLight);

        sunLight = new THREE.DirectionalLight(
            parseInt(colors.sun_light, 16) || 0xfff0c8,
            1.8
        );
        sunLight.position.set(50, 60, -30);
        sunLight.castShadow = true;
        const smSize = tc.shadow_map_size || 2048;
        sunLight.shadow.mapSize.width = smSize;
        sunLight.shadow.mapSize.height = smSize;
        sunLight.shadow.camera.near = 0.5;
        sunLight.shadow.camera.far = 350;
        sunLight.shadow.camera.left = -110;
        sunLight.shadow.camera.right = 110;
        sunLight.shadow.camera.top = 110;
        sunLight.shadow.camera.bottom = -110;
        sunLight.shadow.bias = tc.shadow_bias || -0.0005;
        sunLight.shadow.normalBias = tc.shadow_normal_bias || 0.06;
        sunLight.shadow.radius = tc.shadow_radius || 4;
        scene.add(sunLight);

        createGround(colors);
        createGuibiao(chiScale, gaugeHeight, rulerLength, colors);
        createSunParticles(config);

        window.addEventListener('resize', onWindowResize);
        startAnimation();
    }

    function createGround(colors) {
        const groundGeo = new THREE.PlaneGeometry(300, 300, 50, 50);
        const positions = groundGeo.attributes.position;
        for (let i = 0; i < positions.count; i++) {
            const x = positions.getX(i);
            const y = positions.getY(i);
            const noise = Math.sin(x * 0.05) * Math.cos(y * 0.05) * 0.3;
            positions.setZ(i, noise);
        }
        groundGeo.computeVertexNormals();
        const groundMat = new THREE.MeshStandardMaterial({
            color: parseInt(colors.ground, 16) || 0x5a4a3a,
            roughness: 0.9,
            metalness: 0.1,
        });
        const ground = new THREE.Mesh(groundGeo, groundMat);
        ground.rotation.x = -Math.PI / 2;
        ground.receiveShadow = true;
        scene.add(ground);

        const gridHelper = new THREE.GridHelper(200, 40, 0x444466, 0x222233);
        gridHelper.position.y = 0.01;
        scene.add(gridHelper);
    }

    function createGuibiao(chiScale, gaugeHeightChi, rulerLengthChi, colors) {
        guibiaoGroup = new THREE.Group();

        const baseGeo = new THREE.BoxGeometry(20, 2, 12);
        const baseMat = new THREE.MeshStandardMaterial({
            color: 0x6b5b4b,
            roughness: 0.8,
            metalness: 0.2,
        });
        const base = new THREE.Mesh(baseGeo, baseMat);
        base.position.y = 1;
        base.castShadow = true;
        base.receiveShadow = true;
        guibiaoGroup.add(base);

        const gaugeColor = parseInt(colors.gauge, 16) || 0xc9a959;
        const gaugeGeo = new THREE.BoxGeometry(1.5, gaugeHeightChi * chiScale, 1.5);
        const gaugeMat = new THREE.MeshStandardMaterial({
            color: gaugeColor,
            roughness: 0.4,
            metalness: 0.7,
        });
        gaugeMesh = new THREE.Mesh(gaugeGeo, gaugeMat);
        gaugeMesh.position.set(0, gaugeHeightChi * chiScale / 2 + 2, 0);
        gaugeMesh.castShadow = true;
        guibiaoGroup.add(gaugeMesh);

        const topGeo = new THREE.BoxGeometry(3, 1, 3);
        const top = new THREE.Mesh(topGeo, gaugeMat);
        top.position.set(0, gaugeHeightChi * chiScale + 2.5, 0);
        top.castShadow = true;
        guibiaoGroup.add(top);

        const rulerBaseGeo = new THREE.BoxGeometry(rulerLengthChi * chiScale + 10, 0.8, 8);
        const rulerBaseMat = new THREE.MeshStandardMaterial({
            color: parseInt(colors.ground, 16) || 0x5a4a3a,
            roughness: 0.9,
        });
        const rulerBase = new THREE.Mesh(rulerBaseGeo, rulerBaseMat);
        rulerBase.position.set(rulerLengthChi * chiScale / 2 - 5, 0.4, 0);
        rulerBase.receiveShadow = true;
        guibiaoGroup.add(rulerBase);

        const rulerGeo = new THREE.BoxGeometry(rulerLengthChi * chiScale, 0.3, 6);
        const rulerMat = new THREE.MeshStandardMaterial({
            color: parseInt(colors.ruler, 16) || 0xd4c4a4,
            roughness: 0.7,
        });
        rulerMesh = new THREE.Mesh(rulerGeo, rulerMat);
        rulerMesh.position.set(rulerLengthChi * chiScale / 2 - 5, 1, 0);
        rulerMesh.receiveShadow = true;
        guibiaoGroup.add(rulerMesh);

        for (let i = 0; i <= rulerLengthChi; i++) {
            const isMajor = i % 10 === 0;
            const tickHeight = isMajor ? 0.8 : 0.4;
            const tickGeo = new THREE.BoxGeometry(0.1, tickHeight, 0.1);
            const tickMat = new THREE.MeshBasicMaterial({
                color: isMajor ? 0x000000 : 0x333333
            });
            const tick = new THREE.Mesh(tickGeo, tickMat);
            tick.position.set(i * chiScale - 5, 1.3, 2.8);
            guibiaoGroup.add(tick);
        }

        const shadowGeo = new THREE.PlaneGeometry(0.01, 6);
        const shadowMat = new THREE.MeshBasicMaterial({
            color: 0x111111,
            transparent: true,
            opacity: 0.6,
            side: THREE.DoubleSide,
        });
        shadowMesh = new THREE.Mesh(shadowGeo, shadowMat);
        shadowMesh.rotation.x = -Math.PI / 2;
        shadowMesh.rotation.y = -Math.PI / 2;
        shadowMesh.position.set(gaugeHeightChi * chiScale / 2, 1.16, 0);
        guibiaoGroup.add(shadowMesh);

        scene.add(guibiaoGroup);
    }

    function createSunParticles(cfg) {
        const particleCount = (cfg.render && cfg.render.particle_count_3d) || 200;
        const particleGeo = new THREE.BufferGeometry();
        const positions = new Float32Array(particleCount * 3);
        const colorsArr = new Float32Array(particleCount * 3);
        const velocities = [];
        const colors = cfg.colors || {};
        const beamCol = parseInt(colors.beam, 16) || 0xfff5d4;
        const r = ((beamCol >> 16) & 0xff) / 255;
        const g = ((beamCol >> 8) & 0xff) / 255;
        const b = (beamCol & 0xff) / 255;

        for (let i = 0; i < particleCount; i++) {
            positions[i * 3] = (Math.random() - 0.5) * 100;
            positions[i * 3 + 1] = 50 + Math.random() * 50;
            positions[i * 3 + 2] = -40 + (Math.random() - 0.5) * 30;
            colorsArr[i * 3] = r;
            colorsArr[i * 3 + 1] = g * (0.9 + Math.random() * 0.1);
            colorsArr[i * 3 + 2] = b * (0.6 + Math.random() * 0.2);
            velocities.push({
                x: 0,
                y: -0.1 - Math.random() * 0.2,
                z: 0.05 + Math.random() * 0.1,
            });
        }

        particleGeo.setAttribute('position', new THREE.BufferAttribute(positions, 3));
        particleGeo.setAttribute('color', new THREE.BufferAttribute(colorsArr, 3));

        const particleMat = new THREE.PointsMaterial({
            size: 0.5,
            vertexColors: true,
            transparent: true,
            opacity: 0.8,
            blending: THREE.AdditiveBlending,
        });

        const particleSystem = new THREE.Points(particleGeo, particleMat);
        particleSystem.userData.velocities = velocities;
        particleSystem.userData.particleCount = particleCount;
        particles.push(particleSystem);
        scene.add(particleSystem);

        const beamGeo = new THREE.CylinderGeometry(0.3, 3, 80, 8, 1, true);
        const beamMat = new THREE.MeshBasicMaterial({
            color: beamCol,
            transparent: true,
            opacity: 0.08,
            side: THREE.DoubleSide,
        });
        const beam = new THREE.Mesh(beamGeo, beamMat);
        beam.position.set(15, 40, -15);
        beam.rotation.z = -0.5;
        beam.rotation.x = 0.3;
        guibiaoGroup.add(beam);
    }

    function onWindowResize() {
        const container = document.getElementById('three-container');
        if (!container) return;
        const width = container.clientWidth;
        const height = container.clientHeight;
        camera.aspect = width / height;
        camera.updateProjectionMatrix();
        renderer.setSize(width, height);
    }

    function startAnimation() {
        function animate() {
            animationTimer = requestAnimationFrame(animate);
            const chiScale = window.__CHI_SCALE__ || 0.5;
            if (showParticles && particles.length > 0) {
                const ps = particles[0];
                const positions = ps.geometry.attributes.position.array;
                const velocities = ps.userData.velocities;
                const cnt = ps.userData.particleCount || 0;
                for (let i = 0; i < cnt; i++) {
                    positions[i * 3] += velocities[i].x;
                    positions[i * 3 + 1] += velocities[i].y;
                    positions[i * 3 + 2] += velocities[i].z;
                    if (positions[i * 3 + 1] < 1) {
                        positions[i * 3] = (Math.random() - 0.5) * 60;
                        positions[i * 3 + 1] = 80 + Math.random() * 20;
                        positions[i * 3 + 2] = -30 + (Math.random() - 0.5) * 20;
                    }
                }
                ps.geometry.attributes.position.needsUpdate = true;
            }
            if (guibiaoGroup) {
                guibiaoGroup.rotation.y += 0.0003;
            }
            renderer.render(scene, camera);
        }
        animate();
    }

    function updateSunPosition(altitudeDeg, azimuthDeg) {
        if (!sunLight) return;
        const alt = altitudeDeg * Math.PI / 180;
        const azi = azimuthDeg * Math.PI / 180;
        const distance = 100;
        const x = distance * Math.cos(alt) * Math.sin(azi);
        const y = distance * Math.sin(alt);
        const z = -distance * Math.cos(alt) * Math.cos(azi);
        sunLight.position.set(x, y, z);
        sunLight.intensity = Math.max(0.3, altitudeDeg / 60);
        if (particles.length > 0) {
            particles[0].visible = showParticles;
        }
    }

    function updateShadow(shadowLengthChi) {
        if (!shadowMesh) return;
        const chiScale = window.__CHI_SCALE__ || 0.5;
        const rulerLength = window.__RULER_LENGTH__ || 120;
        const len = Math.min(shadowLengthChi * chiScale, rulerLength * chiScale);
        shadowMesh.geometry.dispose();
        shadowMesh.geometry = new THREE.PlaneGeometry(Math.max(len, 0.1), 5);
        shadowMesh.position.x = Math.max(len / 2, 0.05);
    }

    function setShowParticles(v) {
        showParticles = !!v;
        if (particles.length > 0) {
            particles[0].visible = showParticles;
        }
    }

    function getTier() {
        return tier;
    }

    function dispose() {
        if (animationTimer) cancelAnimationFrame(animationTimer);
        window.removeEventListener('resize', onWindowResize);
    }

    return {
        init,
        updateSunPosition,
        updateShadow,
        setShowParticles,
        getTier,
        dispose,
    };
})();

if (typeof window !== 'undefined') {
    window.Gnomon3D = Gnomon3D;
}
