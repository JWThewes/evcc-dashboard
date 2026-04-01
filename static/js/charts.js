// Chart initialization and management
const charts = {};

const COLORS = {
    grid_power: '#e74c3c',
    pv_power: '#f39c12',
    home_power: '#3498db',
    battery_power: '#2ecc71',
    battery_soc: '#9b59b6',
    grid_import_wh: '#e74c3c',
    grid_export_wh: '#e67e22',
    pv_production_wh: '#f39c12',
    home_consumption_wh: '#3498db',
    self_sufficiency_pct: '#2ecc71',
    charge_power: '#1abc9c',
};

const LABELS = {
    grid_power: 'Grid Power',
    pv_power: 'PV Power',
    home_power: 'Home Consumption',
    battery_power: 'Battery Power',
    battery_soc: 'Battery SoC (%)',
    grid_import_wh: 'Grid Import',
    grid_export_wh: 'Grid Export',
    pv_production_wh: 'PV Production',
    home_consumption_wh: 'Home Consumption',
    self_sufficiency_pct: 'Self Sufficiency (%)',
    charge_power: 'Charge Power',
};

function rangeToSeconds(range) {
    const map = { '24h': 86400, '7d': 604800, '30d': 2592000, '90d': 7776000 };
    return map[range] || 604800;
}

function initChart(el) {
    const chartType = el.dataset.chartType;
    const baseUrl = el.dataset.chartUrl;
    const range = el.dataset.range || '24h';

    const chart = echarts.init(el);
    charts[chartType] = { instance: chart, element: el, baseUrl: baseUrl };

    fetchAndRender(chart, chartType, baseUrl, range);
}

function fetchAndRender(chart, chartType, baseUrl, range) {
    const now = Math.floor(Date.now() / 1000);
    const from = now - rangeToSeconds(range);
    const url = baseUrl + (baseUrl.includes('?') ? '&' : '?') + `from=${from}&to=${now}&resolution=auto`;

    fetch(url)
        .then(r => r.json())
        .then(data => renderChart(chart, chartType, data))
        .catch(err => console.error(`Failed to load chart ${chartType}:`, err));
}

function renderChart(chart, chartType, data) {
    if (!data.timestamps || data.timestamps.length === 0) {
        chart.setOption({
            title: { text: 'No data available', left: 'center', top: 'center', textStyle: { color: '#999' } }
        });
        return;
    }

    const timestamps = data.timestamps.map(ts => new Date(ts * 1000));

    if (chartType === 'energy') {
        renderBarChart(chart, timestamps, data.series);
    } else {
        renderLineChart(chart, chartType, timestamps, data.series);
    }
}

function renderLineChart(chart, chartType, timestamps, series) {
    const seriesConfig = [];
    const yAxes = [];
    let hasSecondAxis = false;

    for (const [key, values] of Object.entries(series)) {
        const isPercentage = key.includes('soc') || key.includes('pct');
        if (isPercentage && !hasSecondAxis) {
            hasSecondAxis = true;
        }

        seriesConfig.push({
            name: LABELS[key] || key,
            type: 'line',
            data: values,
            smooth: true,
            symbol: 'none',
            lineStyle: { width: 2 },
            itemStyle: { color: COLORS[key] || '#666' },
            yAxisIndex: isPercentage ? 1 : 0,
            areaStyle: key === 'pv_power' ? { opacity: 0.15 } : undefined,
        });
    }

    yAxes.push({
        type: 'value',
        name: 'Power (W)',
        axisLabel: { formatter: '{value} W' },
    });

    if (hasSecondAxis) {
        yAxes.push({
            type: 'value',
            name: '%',
            min: 0,
            max: 100,
            axisLabel: { formatter: '{value}%' },
        });
    }

    chart.setOption({
        tooltip: {
            trigger: 'axis',
            backgroundColor: '#1a1d27',
            borderColor: '#3b3f54',
            textStyle: { color: '#e4e4e7' },
            formatter: function (params) {
                let html = params[0].axisValueLabel + '<br/>';
                params.forEach(p => {
                    const val = p.value != null ? p.value.toFixed(0) : '--';
                    const unit = p.seriesName.includes('%') ? '%' : ' W';
                    html += `${p.marker} ${p.seriesName}: <b>${val}${unit}</b><br/>`;
                });
                return html;
            }
        },
        legend: {
            bottom: 0,
            textStyle: { color: '#9ca3af' },
        },
        grid: { left: 60, right: hasSecondAxis ? 60 : 20, bottom: 110, top: 20 },
        xAxis: {
            type: 'category',
            data: timestamps,
            axisLabel: {
                color: '#9ca3af',
                formatter: function (val) {
                    const d = new Date(val);
                    return d.getHours().toString().padStart(2, '0') + ':' +
                           d.getMinutes().toString().padStart(2, '0');
                }
            },
            axisLine: { lineStyle: { color: '#3b3f54' } },
            boundaryGap: false,
        },
        yAxis: yAxes.map(y => ({
            ...y,
            axisLabel: { ...y.axisLabel, color: '#9ca3af' },
            axisLine: { lineStyle: { color: '#3b3f54' } },
            splitLine: { lineStyle: { color: '#2d3140' } },
            nameTextStyle: { color: '#9ca3af' },
        })),
        series: seriesConfig,
        dataZoom: [
            { type: 'inside' },
            {
                type: 'slider',
                bottom: 28,
                height: 20,
                borderColor: '#3b3f54',
                fillerColor: 'rgba(59,130,246,0.15)',
                dataBackground: {
                    lineStyle: { color: '#4b5563' },
                    areaStyle: { color: 'rgba(75,85,99,0.3)' },
                },
                selectedDataBackground: {
                    lineStyle: { color: '#6b7280' },
                    areaStyle: { color: 'rgba(107,114,128,0.3)' },
                },
                handleStyle: { color: '#6b7280', borderColor: '#9ca3af' },
                moveHandleStyle: { color: '#6b7280' },
                textStyle: { color: '#9ca3af' },
            },
        ],
    }, true);
}

function renderBarChart(chart, timestamps, series) {
    const seriesConfig = [];
    const categories = timestamps.map(d => d.toLocaleDateString());

    for (const [key, values] of Object.entries(series)) {
        if (key === 'self_sufficiency_pct') continue;
        seriesConfig.push({
            name: LABELS[key] || key,
            type: 'bar',
            data: values.map(v => v != null ? (v / 1000).toFixed(2) : 0), // Wh -> kWh
            itemStyle: { color: COLORS[key] || '#666' },
        });
    }

    chart.setOption({
        tooltip: {
            trigger: 'axis',
            backgroundColor: '#1a1d27',
            borderColor: '#3b3f54',
            textStyle: { color: '#e4e4e7' },
            formatter: function (params) {
                let html = params[0].axisValueLabel + '<br/>';
                params.forEach(p => {
                    html += `${p.marker} ${p.seriesName}: <b>${p.value} kWh</b><br/>`;
                });
                return html;
            }
        },
        legend: { bottom: 0, textStyle: { color: '#9ca3af' } },
        grid: { left: 60, right: 20, bottom: 30, top: 20 },
        xAxis: {
            type: 'category',
            data: categories,
            axisLabel: { color: '#9ca3af' },
            axisLine: { lineStyle: { color: '#3b3f54' } },
        },
        yAxis: {
            type: 'value',
            name: 'Energy (kWh)',
            nameTextStyle: { color: '#9ca3af' },
            axisLabel: { color: '#9ca3af', formatter: '{value} kWh' },
            axisLine: { lineStyle: { color: '#3b3f54' } },
            splitLine: { lineStyle: { color: '#2d3140' } },
        },
        series: seriesConfig,
    }, true);
}

// Range selector for history page
function updateChartRange(range) {
    document.querySelectorAll('[data-chart-type]').forEach(el => {
        el.dataset.range = range;
        const chartType = el.dataset.chartType;
        const info = charts[chartType];
        if (info) {
            fetchAndRender(info.instance, chartType, info.baseUrl, range);
        }
    });
}

// Auto-refresh dashboard charts every 30 seconds
function startChartRefresh() {
    setInterval(() => {
        document.querySelectorAll('[data-chart-type]').forEach(el => {
            const chartType = el.dataset.chartType;
            const info = charts[chartType];
            if (info) {
                const range = el.dataset.range || '24h';
                fetchAndRender(info.instance, chartType, info.baseUrl, range);
            }
        });
    }, 30000);
}

// Initialize all charts on page load
document.addEventListener('DOMContentLoaded', () => {
    document.querySelectorAll('[data-chart-type]').forEach(initChart);
    startChartRefresh();
});

// Handle window resize
window.addEventListener('resize', () => {
    Object.values(charts).forEach(c => c.instance.resize());
});
