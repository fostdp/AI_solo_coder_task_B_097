const MeridianPanel = (function () {
    let API_URL = '';
    let comparisonData = null;

    function init(apiUrl) {
        API_URL = apiUrl;
        document.getElementById('btn-meridian-compare').addEventListener('click', runComparison);
    }

    async function runComparison() {
        const btn = document.getElementById('btn-meridian-compare');
        btn.textContent = '对比中...';
        btn.disabled = true;
        try {
            const alt = parseFloat(document.getElementById('meridian-alt').value) || 26.0;
            const temp = parseFloat(document.getElementById('meridian-temp').value) || 5.0;
            const pressure = parseFloat(document.getElementById('meridian-pressure').value) || 1013.25;

            const resp = await fetch(`${API_URL}/api/meridian/compare`, {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({ sun_altitude: alt, temperature: temp, pressure: pressure }),
            });
            const result = await resp.json();
            if (result.success && result.data) {
                comparisonData = result.data;
                renderComparison(result.data);
            }
        } catch (e) {
            console.error('[MeridianPanel] 对比失败:', e);
        }
        btn.textContent = '跨时代测量对比';
        btn.disabled = false;
    }

    function renderComparison(data) {
        const container = document.getElementById('meridian-results');
        if (!container) return;

        let html = '<table class="meridian-table"><thead><tr>' +
            '<th>仪器</th><th>年代</th><th>高度角误差(")</th><th>影长误差(寸)</th>' +
            '<th>冬至时刻误差(秒)</th><th>技术差距倍数</th><th>蒙气差修正(")</th></tr></thead><tbody>';

        data.forEach(d => {
            const gapColor = d.technology_gap_factor >= 100 ? '#44ff88' :
                             d.technology_gap_factor >= 10 ? '#88ddff' : '#ffaa44';
            html += `<tr class="meridian-row">
                <td class="meridian-name">${d.instrument_name}</td>
                <td>${d.era}</td>
                <td>${d.altitude_error_arcsec.toFixed(2)}</td>
                <td>${d.shadow_error_cun.toFixed(4)}</td>
                <td>${d.solstice_time_error_seconds.toFixed(1)}</td>
                <td><span style="color:${gapColor};font-weight:bold">${d.technology_gap_factor.toFixed(0)}×</span></td>
                <td>${d.refraction_correction_arcsec.toFixed(2)}</td>
            </tr>`;
        });

        html += '</tbody></table>';

        html += '<div class="meridian-chart">';
        const maxError = Math.max(...data.map(d => d.solstice_time_error_seconds));
        data.forEach(d => {
            const pct = (d.solstice_time_error_seconds / maxError) * 100;
            const color = d.technology_gap_factor >= 100 ? '#44ff88' :
                          d.technology_gap_factor >= 10 ? '#88ddff' : '#ffaa44';
            html += `<div class="bar-row">
                <span class="bar-label">${d.instrument_name}</span>
                <div class="bar-track"><div class="bar-fill" style="width:${Math.max(pct,2)}%;background:${color}"></div></div>
                <span class="bar-value">${d.solstice_time_error_seconds.toFixed(1)}s</span>
            </div>`;
        });
        html += '</div>';

        container.innerHTML = html;
    }

    return { init, runComparison };
})();

if (typeof window !== 'undefined') {
    window.MeridianPanel = MeridianPanel;
}
