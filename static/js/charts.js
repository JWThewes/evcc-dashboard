// Chart initialization and management
const charts = {};

const COLORS = {
    grid_power: '#f87171',
    pv_power: '#fbbf24',
    home_power: '#60a5fa',
    battery_power: '#34d399',
    battery_soc: '#a78bfa',
    grid_import_wh: '#f87171',
    grid_export_wh: '#fb923c',
    pv_production_wh: '#fbbf24',
    home_consumption_wh: '#60a5fa',
    self_sufficiency_pct: '#34d399',
    charge_power: '#2dd4bf',
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

// Shared theme constants
const THEME = {
    bg: 'rgba(22, 27, 45, 0.85)',
    border: 'rgba(255, 255, 255, 0.08)',
    text: '#eaecf0',
    textMuted: '#6b7280',
    textSecondary: '#a1a7b4',
    gridLine: 'rgba(255, 255, 255, 0.04)',
    axisLine: 'rgba(255, 255, 255, 0.08)',
    accentFill: 'rgba(59, 130, 246, 0.12)',
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
            title: {
                text: 'No data available',
                left: 'center',
                top: 'center',
                textStyle: { color: THEME.textMuted, fontSize: 14, fontFamily: 'Inter, sans-serif' }
            }
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
            smooth: 0.3,
            symbol: 'none',
            lineStyle: { width: 2 },
            itemStyle: { color: COLORS[key] || '#666' },
            yAxisIndex: isPercentage ? 1 : 0,
            areaStyle: key === 'pv_power' ? {
                color: new echarts.graphic.LinearGradient(0, 0, 0, 1, [
                    { offset: 0, color: 'rgba(251, 191, 36, 0.2)' },
                    { offset: 1, color: 'rgba(251, 191, 36, 0)' },
                ])
            } : undefined,
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
            backgroundColor: THEME.bg,
            borderColor: THEME.border,
            borderWidth: 1,
            textStyle: { color: THEME.text, fontSize: 13, fontFamily: 'Inter, sans-serif' },
            formatter: function (params) {
                let html = `<div style="margin-bottom:6px;color:${THEME.textSecondary};font-size:12px">${params[0].axisValueLabel}</div>`;
                params.forEach(p => {
                    const val = p.value != null ? p.value.toFixed(0) : '--';
                    const unit = p.seriesName.includes('%') ? '%' : ' W';
                    html += `<div style="display:flex;justify-content:space-between;gap:16px;line-height:1.7">${p.marker} <span>${p.seriesName}</span> <b>${val}${unit}</b></div>`;
                });
                return html;
            }
        },
        legend: {
            bottom: 0,
            textStyle: { color: THEME.textSecondary, fontSize: 12, fontFamily: 'Inter, sans-serif' },
            itemGap: 16,
        },
        grid: { left: 60, right: hasSecondAxis ? 60 : 20, bottom: 110, top: 20 },
        xAxis: {
            type: 'category',
            data: timestamps,
            axisLabel: {
                color: THEME.textMuted,
                fontSize: 11,
                fontFamily: 'Inter, sans-serif',
                formatter: function (val) {
                    const d = new Date(val);
                    return d.getHours().toString().padStart(2, '0') + ':' +
                           d.getMinutes().toString().padStart(2, '0');
                }
            },
            axisLine: { lineStyle: { color: THEME.axisLine } },
            axisTick: { show: false },
            boundaryGap: false,
        },
        yAxis: yAxes.map(y => ({
            ...y,
            axisLabel: { ...y.axisLabel, color: THEME.textMuted, fontSize: 11, fontFamily: 'Inter, sans-serif' },
            axisLine: { show: false },
            splitLine: { lineStyle: { color: THEME.gridLine } },
            nameTextStyle: { color: THEME.textSecondary, fontSize: 12, fontFamily: 'Inter, sans-serif' },
        })),
        series: seriesConfig,
        dataZoom: [
            { type: 'inside' },
            {
                type: 'slider',
                bottom: 28,
                height: 20,
                borderColor: THEME.border,
                fillerColor: THEME.accentFill,
                dataBackground: {
                    lineStyle: { color: 'rgba(255,255,255,0.08)' },
                    areaStyle: { color: 'rgba(255,255,255,0.03)' },
                },
                selectedDataBackground: {
                    lineStyle: { color: 'rgba(255,255,255,0.12)' },
                    areaStyle: { color: 'rgba(255,255,255,0.05)' },
                },
                handleStyle: { color: '#3b82f6', borderColor: 'rgba(59,130,246,0.4)' },
                moveHandleStyle: { color: 'rgba(255,255,255,0.1)' },
                textStyle: { color: THEME.textSecondary, fontSize: 11, fontFamily: 'Inter, sans-serif' },
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
            data: values.map(v => v != null ? (v / 1000).toFixed(2) : 0),
            itemStyle: {
                color: COLORS[key] || '#666',
                borderRadius: [3, 3, 0, 0],
            },
            barMaxWidth: 24,
        });
    }

    chart.setOption({
        tooltip: {
            trigger: 'axis',
            backgroundColor: THEME.bg,
            borderColor: THEME.border,
            borderWidth: 1,
            textStyle: { color: THEME.text, fontSize: 13, fontFamily: 'Inter, sans-serif' },
            formatter: function (params) {
                let html = `<div style="margin-bottom:6px;color:${THEME.textSecondary};font-size:12px">${params[0].axisValueLabel}</div>`;
                params.forEach(p => {
                    html += `<div style="display:flex;justify-content:space-between;gap:16px;line-height:1.7">${p.marker} <span>${p.seriesName}</span> <b>${p.value} kWh</b></div>`;
                });
                return html;
            }
        },
        legend: {
            bottom: 0,
            textStyle: { color: THEME.textSecondary, fontSize: 12, fontFamily: 'Inter, sans-serif' },
            itemGap: 16,
        },
        grid: { left: 60, right: 20, bottom: 30, top: 20 },
        xAxis: {
            type: 'category',
            data: categories,
            axisLabel: { color: THEME.textMuted, fontSize: 11, fontFamily: 'Inter, sans-serif' },
            axisLine: { lineStyle: { color: THEME.axisLine } },
            axisTick: { show: false },
        },
        yAxis: {
            type: 'value',
            name: 'Energy (kWh)',
            nameTextStyle: { color: THEME.textSecondary, fontSize: 12, fontFamily: 'Inter, sans-serif' },
            axisLabel: { color: THEME.textMuted, formatter: '{value} kWh', fontSize: 11, fontFamily: 'Inter, sans-serif' },
            axisLine: { show: false },
            splitLine: { lineStyle: { color: THEME.gridLine } },
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
