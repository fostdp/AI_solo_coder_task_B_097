const DynastyPanel = (function () {
    let API_URL = '';
    let comparisonData = null;

    function init(apiUrl) {
        API_URL = apiUrl;
        document.getElementById('btn-dynasty-compare').addEventListener('click', runComparison);
    }

    async function runComparison() {
        const btn = document.getElementById('btn-dynasty-compare');
        btn.textContent = '对比中...';
        btn.disabled = true;
        try {
            const alt = parseFloat(document.getElementById('dynasty-alt').value) || 26.0;
            const temp = parseFloat(document.getElementById('dynasty-temp').value) || 5.0;
            const pressure = parseFloat(document.getElementById('dynasty-pressure').value) || 1013.25;
            const humidity = parseFloat(document.getElementById('dynasty-humidity').value) || 50.0;

            const resp = await fetch(`${API_URL}/api/dynasty/compare`, {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({ sun_altitude: alt, temperature: temp, pressure: pressure, humidity: humidity }),
            });
            const result = await resp.json();
            if (result.success && result.data) {
                comparisonData = result.data;
                renderComparison(result.data);
            }
        } catch (e) {
            console.error('[DynastyPanel] 对比失败:', e);
        }
        btn.textContent = '跨朝代精度对比';
        btn.disabled = false;
    }

    function renderComparison(data) {
        const container = document.getElementById('dynasty-results');
        if (!container) return;

        const maxShadow = Math.max(...data.map(d => d.refracted_shadow_chi));
        const maxPrecision = Math.max(...data.map(d => d.solstice_precision_seconds));

        let html = '<table class="dynasty-table"><thead><tr>' +
            '<th>朝代</th><th>表高(尺)</th><th>材质</th><th>理论影长(尺)</th>' +
            '<th>折射影长(尺)</th><th>蒙气差(")</th><th>影长精度(寸)</th>' +
            '<th>冬至精度(秒)</th><th>角分辨率(\')</th></tr></thead><tbody>';

        data.forEach(d => {
            const precisionPct = (1 - d.solstice_precision_seconds / maxPrecision) * 100;
            html += `<tr class="dynasty-row">
                <td class="dynasty-name">${d.dynasty_name}</td>
                <td>${d.gauge_height_chi}</td>
                <td>${d.gauge_material}</td>
                <td>${d.theoretical_shadow_chi.toFixed(2)}</td>
                <td>${d.refracted_shadow_chi.toFixed(2)}</td>
                <td>${d.refraction_correction_arcsec.toFixed(2)}</td>
                <td>${d.shadow_precision_cun.toFixed(2)}</td>
                <td>
                    <div class="precision-bar-container">
                        <div class="precision-bar" style="width:${Math.max(precisionPct, 5)}%;background:${precisionPct > 80 ? '#44ff88' : precisionPct > 40 ? '#ffaa44' : '#ff6666'}"></div>
                        <span class="precision-text">${d.solstice_precision_seconds.toFixed(0)}</span>
                    </div>
                </td>
                <td>${d.altitude_resolution_arcmin.toFixed(2)}</td>
            </tr>`;
        });

        html += '</tbody></table>';
        container.innerHTML = html;
    }

    return { init, runComparison };
})();

if (typeof window !== 'undefined') {
    window.DynastyPanel = DynastyPanel;
}
