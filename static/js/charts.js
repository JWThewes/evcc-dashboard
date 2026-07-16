// Chart initialization and management — ECharts theme engine (echarts-integration unit)
// Theme derived from CSS design tokens per contract-css-tokens / contract-echarts-theme-name
"use strict";

var charts = {};
var THEME_NAME = "evcc-energy";
var _themeListenerRegistered = false;

var LABELS = {
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

// --- Fallback colors (dark palette) for when CSS tokens are unavailable ---
var FALLBACK_COLORS = {
    "--color-solar": "#f5a623",
    "--color-grid": "#4a9eff",
    "--color-battery": "#4caf50",
    "--color-ev": "#9c6ade",
    "--color-home": "#d0d0e0",
    "--color-surface-0": "#0f0f1a",
    "--color-surface-1": "#1a1a2e",
    "--color-surface-2": "#2a2a40",
    "--color-surface-3": "#3a3a55",
    "--color-text-primary": "#f5f5fa",
    "--color-text-secondary": "#9a9ab0",
    "--color-text-muted": "#5a5a75",
    "--color-positive": "#4caf50",
    "--color-negative": "#ef5350",
    "--color-neutral": "#9a9ab0"
};

// --- Theme Engine: Token Reader ---
function getComputedTokens() {
    var root = document.documentElement;
    var styles = window.getComputedStyle(root);
    var tokenNames = [
        "--color-solar",
        "--color-grid",
        "--color-battery",
        "--color-ev",
        "--color-home",
        "--color-surface-0",
        "--color-surface-1",
        "--color-surface-2",
        "--color-surface-3",
        "--color-text-primary",
        "--color-text-secondary",
        "--color-text-muted",
        "--color-positive",
        "--color-negative",
        "--color-neutral"
    ];
    var tokens = {};
    for (var i = 0; i < tokenNames.length; i++) {
        var name = tokenNames[i];
        var val = styles.getPropertyValue(name).trim();
        tokens[name] = val || FALLBACK_COLORS[name] || "#666";
    }
    return tokens;
}

// --- Theme Engine: Theme Builder ---
function buildThemeObject(tokens) {
    var fontFamily = window.getComputedStyle(document.documentElement)
        .getPropertyValue("--font-family").trim() ||
        '-apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif';

    return {
        color: [
            tokens["--color-solar"],
            tokens["--color-grid"],
            tokens["--color-battery"],
            tokens["--color-ev"],
            tokens["--color-home"],
            tokens["--color-positive"],
            tokens["--color-negative"]
        ],
        backgroundColor: "transparent",
        textStyle: {
            color: tokens["--color-text-primary"],
            fontFamily: fontFamily
        },
        title: {
            textStyle: { color: tokens["--color-text-primary"] },
            subtextStyle: { color: tokens["--color-text-secondary"] }
        },
        legend: {
            textStyle: { color: tokens["--color-text-secondary"] }
        },
        tooltip: {
            backgroundColor: tokens["--color-surface-1"],
            borderColor: tokens["--color-surface-3"],
            textStyle: { color: tokens["--color-text-primary"] }
        },
        categoryAxis: {
            axisLine: { lineStyle: { color: tokens["--color-surface-3"] } },
            axisTick: { lineStyle: { color: tokens["--color-surface-3"] } },
            axisLabel: { color: tokens["--color-text-muted"] },
            splitLine: { lineStyle: { color: tokens["--color-surface-2"] } }
        },
        valueAxis: {
            axisLine: { lineStyle: { color: tokens["--color-surface-3"] } },
            axisTick: { lineStyle: { color: tokens["--color-surface-3"] } },
            axisLabel: { color: tokens["--color-text-muted"] },
            splitLine: { lineStyle: { color: tokens["--color-surface-2"] } },
            nameTextStyle: { color: tokens["--color-text-secondary"] }
        },
        dataZoom: {
            borderColor: tokens["--color-surface-3"],
            fillerColor: tokens["--color-surface-2"] + "33",
            handleColor: tokens["--color-grid"],
            textStyle: { color: tokens["--color-text-secondary"] }
        }
    };
}

// --- Theme Engine: Registration ---
function registerEnergyTheme() {
    if (typeof echarts === "undefined") {
        console.warn("[evcc-charts] echarts not available, skipping theme registration");
        return;
    }
    try {
        var tokens = getComputedTokens();
        var themeObj = buildThemeObject(tokens);
        echarts.registerTheme(THEME_NAME, themeObj);
    } catch (e) {
        console.warn("[evcc-charts] theme registration failed:", e);
    }
}

// --- Theme Engine: Re-render on toggle ---
function rerenderCharts() {
    try {
        var keys = Object.keys(charts);
        for (var i = 0; i < keys.length; i++) {
            var chartType = keys[i];
            var info = charts[chartType];
            if (!info || !info.element || !info.element.isConnected) {
                delete charts[chartType];
                continue;
            }
            var range = info.element.dataset.range || "24h";
            try { info.instance.dispose(); } catch (e) { /* already disposed */ }
            var newInstance = echarts.init(info.element, THEME_NAME);
            charts[chartType] = { instance: newInstance, element: info.element, baseUrl: info.baseUrl };
            fetchAndRender(newInstance, chartType, info.baseUrl, range);
        }
    } catch (e) {
        console.warn("[evcc-charts] rerender failed:", e);
    }
}

// --- Chart Utilities ---
function rangeToSeconds(range) {
    var map = { '24h': 86400, '7d': 604800, '30d': 2592000, '90d': 7776000 };
    return map[range] || 604800;
}

function initChart(el) {
    if (typeof echarts === "undefined") return;
    var chartType = el.dataset.chartType;
    var baseUrl = el.dataset.chartUrl;
    var range = el.dataset.range || '24h';

    var chart = echarts.init(el, THEME_NAME);
    charts[chartType] = { instance: chart, element: el, baseUrl: baseUrl };

    fetchAndRender(chart, chartType, baseUrl, range);
}

function fetchAndRender(chart, chartType, baseUrl, range) {
    var now = Math.floor(Date.now() / 1000);
    var from = now - rangeToSeconds(range);
    var url = baseUrl + (baseUrl.includes('?') ? '&' : '?') + 'from=' + from + '&to=' + now + '&resolution=auto';

    fetch(url)
        .then(function(r) { return r.json(); })
        .then(function(data) { renderChart(chart, chartType, data); })
        .catch(function(err) { console.error('Failed to load chart ' + chartType + ':', err); });
}

function renderChart(chart, chartType, data) {
    if (!data.timestamps || data.timestamps.length === 0) {
        chart.setOption({
            title: {
                text: 'No data available',
                left: 'center',
                top: 'center'
            }
        });
        return;
    }

    var timestamps = data.timestamps.map(function(ts) { return new Date(ts * 1000); });

    if (chartType === 'energy') {
        renderBarChart(chart, timestamps, data.series);
    } else {
        renderLineChart(chart, chartType, timestamps, data.series);
    }
}

function renderLineChart(chart, chartType, timestamps, series) {
    var seriesConfig = [];
    var yAxes = [];
    var hasSecondAxis = false;

    var keys = Object.keys(series);
    for (var i = 0; i < keys.length; i++) {
        var key = keys[i];
        var values = series[key];
        var isPercentage = key.includes('soc') || key.includes('pct');
        if (isPercentage && !hasSecondAxis) {
            hasSecondAxis = true;
        }

        var config = {
            name: LABELS[key] || key,
            type: 'line',
            data: values,
            smooth: 0.3,
            symbol: 'none',
            lineStyle: { width: 2 },
            yAxisIndex: isPercentage ? 1 : 0
        };

        // PV area gradient using first color in theme palette (solar)
        if (key === 'pv_power') {
            config.areaStyle = {
                color: new echarts.graphic.LinearGradient(0, 0, 0, 1, [
                    { offset: 0, color: 'rgba(245, 166, 35, 0.2)' },
                    { offset: 1, color: 'rgba(245, 166, 35, 0)' }
                ])
            };
        }

        seriesConfig.push(config);
    }

    yAxes.push({
        type: 'value',
        name: 'Power (W)',
        axisLabel: { formatter: '{value} W' }
    });

    if (hasSecondAxis) {
        yAxes.push({
            type: 'value',
            name: '%',
            min: 0,
            max: 100,
            axisLabel: { formatter: '{value}%' }
        });
    }

    chart.setOption({
        tooltip: {
            trigger: 'axis',
            borderWidth: 1
        },
        legend: {
            bottom: 0,
            itemGap: 16
        },
        grid: { left: 60, right: hasSecondAxis ? 60 : 20, bottom: 110, top: 20 },
        xAxis: {
            type: 'category',
            data: timestamps,
            axisLabel: {
                formatter: function(val) {
                    var d = new Date(val);
                    return d.getHours().toString().padStart(2, '0') + ':' +
                           d.getMinutes().toString().padStart(2, '0');
                }
            },
            axisTick: { show: false },
            boundaryGap: false
        },
        yAxis: yAxes.map(function(y) {
            return {
                type: y.type,
                name: y.name,
                min: y.min,
                max: y.max,
                axisLabel: y.axisLabel,
                axisLine: { show: false }
            };
        }),
        series: seriesConfig,
        dataZoom: [
            { type: 'inside' },
            {
                type: 'slider',
                bottom: 28,
                height: 20
            }
        ]
    }, true);
}

function renderBarChart(chart, timestamps, series) {
    var seriesConfig = [];
    var categories = timestamps.map(function(d) { return d.toLocaleDateString(); });

    var keys = Object.keys(series);
    for (var i = 0; i < keys.length; i++) {
        var key = keys[i];
        if (key === 'self_sufficiency_pct') continue;
        seriesConfig.push({
            name: LABELS[key] || key,
            type: 'bar',
            data: series[key].map(function(v) { return v != null ? (v / 1000).toFixed(2) : 0; }),
            itemStyle: { borderRadius: [3, 3, 0, 0] },
            barMaxWidth: 24
        });
    }

    chart.setOption({
        tooltip: {
            trigger: 'axis',
            borderWidth: 1,
            formatter: function(params) {
                var html = '<div style="margin-bottom:6px;font-size:12px">' + params[0].axisValueLabel + '</div>';
                params.forEach(function(p) {
                    html += '<div style="display:flex;justify-content:space-between;gap:16px;line-height:1.7">' +
                        p.marker + ' <span>' + p.seriesName + '</span> <b>' + p.value + ' kWh</b></div>';
                });
                return html;
            }
        },
        legend: {
            bottom: 0,
            itemGap: 16
        },
        grid: { left: 60, right: 20, bottom: 30, top: 20 },
        xAxis: {
            type: 'category',
            data: categories,
            axisTick: { show: false }
        },
        yAxis: {
            type: 'value',
            name: 'Energy (kWh)',
            axisLabel: { formatter: '{value} kWh' },
            axisLine: { show: false }
        },
        series: seriesConfig
    }, true);
}

// Range selector for history page
function updateChartRange(range) {
    document.querySelectorAll('[data-chart-type]').forEach(function(el) {
        el.dataset.range = range;
        var chartType = el.dataset.chartType;
        var info = charts[chartType];
        if (info) {
            fetchAndRender(info.instance, chartType, info.baseUrl, range);
        }
    });
}

// Auto-refresh dashboard charts every 30 seconds
function startChartRefresh() {
    setInterval(function() {
        document.querySelectorAll('[data-chart-type]').forEach(function(el) {
            var chartType = el.dataset.chartType;
            var info = charts[chartType];
            if (info) {
                var range = el.dataset.range || '24h';
                fetchAndRender(info.instance, chartType, info.baseUrl, range);
            }
        });
    }, 30000);
}

// --- Event Coordination ---
function onThemeChanged() {
    registerEnergyTheme();
    rerenderCharts();
}

// Initialize all charts on page load
document.addEventListener('DOMContentLoaded', function() {
    // Register theme BEFORE any chart init (Rule 1)
    registerEnergyTheme();

    document.querySelectorAll('[data-chart-type]').forEach(initChart);
    startChartRefresh();

    // Listen for theme toggles (idempotency guard)
    if (!_themeListenerRegistered) {
        document.addEventListener('theme-changed', onThemeChanged);
        _themeListenerRegistered = true;
    }
});

// Handle window resize
window.addEventListener('resize', function() {
    var keys = Object.keys(charts);
    for (var i = 0; i < keys.length; i++) {
        var info = charts[keys[i]];
        if (info && info.instance) {
            try { info.instance.resize(); } catch (e) { /* disposed */ }
        }
    }
});
