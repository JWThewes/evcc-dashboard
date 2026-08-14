/**
 * history-charts.js — ECharts management for interactive history page.
 * Native ES module per dec-es-modules-no-bundler.
 * Provides dataZoom, brush selection, and crosshair tooltips.
 */

// Theme constants (shared with existing charts.js conventions)
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

// Module state (closure-scoped)
let charts = {};       // { power: ECharts, battery: ECharts, energy: ECharts }
let containers = {};   // { power: HTMLElement, ... }
let abortControllers = {};
let currentRange = null;
let comparisonState = null; // set by comparison module
let resizeTimer = null;

/**
 * Initialize chart instances.
 * @param {{ power: HTMLElement, battery?: HTMLElement, energy?: HTMLElement }} containerMap
 */
export function init(containerMap) {
  containers = containerMap;
  for (const [type, el] of Object.entries(containerMap)) {
    if (el && typeof echarts !== 'undefined') {
      charts[type] = echarts.init(el);
    }
  }
}

/**
 * Load chart data for a given date range.
 * @param {{ from: number, to: number, label?: string }} dateRange
 * @param {string[]|null} metrics - if null, show all available
 */
export async function loadData(dateRange, metrics) {
  currentRange = dateRange;

  // Show loading, hide empty
  toggleLoading(true);
  toggleEmpty(false);

  try {
    const data = await fetchChartData('power', dateRange);
    if (!data || !data.timestamps || data.timestamps.length === 0) {
      toggleLoading(false);
      toggleEmpty(true);
      return;
    }

    renderPowerChart(data, metrics);
    toggleLoading(false);

    // Load secondary charts if available
    if (charts.battery) {
      const batteryData = await fetchChartData('battery', dateRange);
      if (batteryData && batteryData.timestamps && batteryData.timestamps.length > 0) {
        renderBatteryChart(batteryData);
      }
    }

    if (charts.energy) {
      const energyData = await fetchChartData('energy', dateRange);
      if (energyData && energyData.timestamps && energyData.timestamps.length > 0) {
        renderEnergyChart(energyData);
      }
    }
  } catch (err) {
    if (err.name === 'AbortError') return; // Cancelled — another load in progress
    toggleLoading(false);
    toggleEmpty(true, true); // show with retry
    console.error('History chart load error:', err);
  }
}

/**
 * Enable day-over-day comparison.
 * @param {{ from: number, to: number }} baseDate
 * @param {{ from: number, to: number }} compareDate
 */
export async function enableComparison(baseDate, compareDate) {
  comparisonState = { type: 'day-over-day', baseDate, compareDate };
  toggleLoading(true);

  try {
    const [baseData, compareData] = await Promise.all([
      fetchChartData('power', baseDate),
      fetchChartData('power', compareDate),
    ]);

    if (!baseData || !baseData.timestamps || baseData.timestamps.length === 0) {
      toggleLoading(false);
      toggleEmpty(true);
      return;
    }

    renderComparisonChart(baseData, compareData, baseDate, compareDate);
    toggleLoading(false);
  } catch (err) {
    if (err.name === 'AbortError') return;
    toggleLoading(false);
    showToast('Comparison data unavailable');
    // Fall back to single day
    if (currentRange) loadData(currentRange, null);
  }
}

/**
 * Enable multi-metric view.
 * @param {string[]} metricKeys
 */
export async function enableMultiMetric(metricKeys) {
  comparisonState = { type: 'multi-metric', metrics: metricKeys };
  if (currentRange) {
    await loadData(currentRange, metricKeys);
  }
}

/**
 * Disable comparison mode and reload normal view.
 */
export async function disableComparison() {
  comparisonState = null;
  if (currentRange) {
    await loadData(currentRange, null);
  }
}

/**
 * Handle window resize (debounced 150ms).
 */
export function onResize() {
  clearTimeout(resizeTimer);
  resizeTimer = setTimeout(() => {
    Object.values(charts).forEach(c => { if (c) c.resize(); });
  }, 150);
}

/**
 * Dispose all chart instances.
 */
export function dispose() {
  Object.values(charts).forEach(c => { if (c) c.dispose(); });
  charts = {};
  Object.values(abortControllers).forEach(ac => ac.abort());
  abortControllers = {};
}

/**
 * Reset zoom to full range.
 */
export function resetZoom() {
  const chart = charts.power;
  if (!chart) return;
  chart.dispatchAction({ type: 'dataZoom', start: 0, end: 100 });
  hideResetButton();
}

// --- Internal functions ---

async function fetchChartData(chartType, range) {
  // Abort previous request for this chart type
  if (abortControllers[chartType]) {
    abortControllers[chartType].abort();
  }
  const controller = new AbortController();
  abortControllers[chartType] = controller;

  const el = containers[chartType] || containers.power;
  const baseUrl = el.dataset.chartUrl;
  const separator = baseUrl.includes('?') ? '&' : '?';
  const url = `${baseUrl}${separator}from=${range.from}&to=${range.to}&resolution=auto`;

  const response = await fetch(url, { signal: controller.signal });
  if (!response.ok) throw new Error(`HTTP ${response.status}`);
  return response.json();
}

function renderPowerChart(data, filterMetrics) {
  const chart = charts.power;
  if (!chart) return;

  const timestamps = data.timestamps.map(ts => new Date(ts * 1000));
  const seriesConfig = [];
  let hasSecondAxis = false;

  for (const [key, values] of Object.entries(data.series)) {
    if (filterMetrics && !filterMetrics.includes(key)) continue;
    const isPercentage = key.includes('soc') || key.includes('pct');
    if (isPercentage) hasSecondAxis = true;

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
        color: { type: 'linear', x: 0, y: 0, x2: 0, y2: 1, colorStops: [
          { offset: 0, color: 'rgba(251, 191, 36, 0.2)' },
          { offset: 1, color: 'rgba(251, 191, 36, 0)' },
        ]}
      } : undefined,
    });
  }

  const yAxes = [buildYAxis('Power (W)', '{value} W')];
  if (hasSecondAxis) {
    yAxes.push(buildYAxis('%', '{value}%', 0, 100));
  }

  chart.setOption({
    tooltip: buildTooltip(),
    legend: buildLegend(),
    grid: { left: 60, right: hasSecondAxis ? 60 : 20, bottom: 80, top: 20 },
    xAxis: buildXAxis(timestamps),
    yAxis: yAxes,
    series: seriesConfig,
    dataZoom: buildDataZoom(),
    brush: buildBrush(),
    toolbox: { show: false },
  }, true);

  // Listen for brush events
  chart.off('brushEnd');
  chart.on('brushEnd', handleBrushEnd);

  chart.off('dataZoom');
  chart.on('dataZoom', handleDataZoom);
}

function renderComparisonChart(baseData, compareData, baseRange, compareRange) {
  const chart = charts.power;
  if (!chart) return;

  const baseMidnight = baseRange.from;
  const compareMidnight = compareRange.from;

  // Normalise to hours from midnight
  const baseTimestamps = baseData.timestamps.map(ts => (ts - baseMidnight) / 3600);
  const compareTimestamps = compareData ? compareData.timestamps.map(ts => (ts - compareMidnight) / 3600) : [];

  const baseLabel = new Date(baseMidnight * 1000).toLocaleDateString(undefined, { day: 'numeric', month: 'short' });
  const compareLabel = compareData ? new Date(compareMidnight * 1000).toLocaleDateString(undefined, { day: 'numeric', month: 'short' }) : '';

  const seriesConfig = [];

  for (const [key, values] of Object.entries(baseData.series)) {
    if (key.includes('soc') || key.includes('pct')) continue;
    // Base series
    seriesConfig.push({
      name: `${LABELS[key] || key} (${baseLabel})`,
      type: 'line',
      data: baseTimestamps.map((t, i) => [t, values[i]]),
      smooth: 0.3,
      symbol: 'none',
      lineStyle: { width: 2, type: 'solid' },
      itemStyle: { color: COLORS[key] || '#666' },
      areaStyle: key === 'pv_power' ? {
        color: { type: 'linear', x: 0, y: 0, x2: 0, y2: 1, colorStops: [
          { offset: 0, color: 'rgba(251, 191, 36, 0.15)' },
          { offset: 1, color: 'rgba(251, 191, 36, 0)' },
        ]}
      } : undefined,
    });

    // Compare series
    if (compareData && compareData.series[key]) {
      seriesConfig.push({
        name: `${LABELS[key] || key} (${compareLabel})`,
        type: 'line',
        data: compareTimestamps.map((t, i) => [t, compareData.series[key][i]]),
        smooth: 0.3,
        symbol: 'none',
        lineStyle: { width: 2, type: 'dashed', opacity: 0.6 },
        itemStyle: { color: COLORS[key] || '#666', opacity: 0.6 },
      });
    }
  }

  chart.setOption({
    tooltip: buildTooltip(),
    legend: buildLegend(),
    grid: { left: 60, right: 20, bottom: 80, top: 20 },
    xAxis: {
      type: 'value',
      min: 0,
      max: 24,
      axisLabel: {
        color: THEME.textMuted,
        fontSize: 11,
        fontFamily: 'Inter, sans-serif',
        formatter: v => `${Math.floor(v)}:00`
      },
      axisLine: { lineStyle: { color: THEME.axisLine } },
      axisTick: { show: false },
    },
    yAxis: [buildYAxis('Power (W)', '{value} W')],
    series: seriesConfig,
    dataZoom: buildDataZoom(),
  }, true);
}

function renderBatteryChart(data) {
  const chart = charts.battery;
  if (!chart) return;

  const timestamps = data.timestamps.map(ts => new Date(ts * 1000));
  const seriesConfig = [];

  for (const [key, values] of Object.entries(data.series)) {
    seriesConfig.push({
      name: LABELS[key] || key,
      type: 'line',
      data: values,
      smooth: 0.3,
      symbol: 'none',
      lineStyle: { width: 2 },
      itemStyle: { color: COLORS[key] || '#666' },
    });
  }

  chart.setOption({
    tooltip: buildTooltip(),
    legend: buildLegend(),
    grid: { left: 50, right: 20, bottom: 60, top: 20 },
    xAxis: buildXAxis(timestamps),
    yAxis: [buildYAxis('%', '{value}%', 0, 100)],
    series: seriesConfig,
    dataZoom: [{ type: 'inside' }],
  }, true);
}

function renderEnergyChart(data) {
  const chart = charts.energy;
  if (!chart) return;

  const timestamps = data.timestamps.map(ts => new Date(ts * 1000).toLocaleDateString());
  const seriesConfig = [];

  for (const [key, values] of Object.entries(data.series)) {
    if (key === 'self_sufficiency_pct') continue;
    seriesConfig.push({
      name: LABELS[key] || key,
      type: 'bar',
      data: values.map(v => v != null ? (v / 1000).toFixed(2) : 0),
      itemStyle: { color: COLORS[key] || '#666', borderRadius: [3, 3, 0, 0] },
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
    },
    legend: buildLegend(),
    grid: { left: 60, right: 20, bottom: 30, top: 20 },
    xAxis: {
      type: 'category',
      data: timestamps,
      axisLabel: { color: THEME.textMuted, fontSize: 11, fontFamily: 'Inter, sans-serif' },
      axisLine: { lineStyle: { color: THEME.axisLine } },
      axisTick: { show: false },
    },
    yAxis: [{
      type: 'value',
      name: 'Energy (kWh)',
      nameTextStyle: { color: THEME.textSecondary, fontSize: 12, fontFamily: 'Inter, sans-serif' },
      axisLabel: { color: THEME.textMuted, formatter: '{value} kWh', fontSize: 11, fontFamily: 'Inter, sans-serif' },
      axisLine: { show: false },
      splitLine: { lineStyle: { color: THEME.gridLine } },
    }],
    series: seriesConfig,
  }, true);
}

// --- Chart option builders ---

function buildTooltip() {
  return {
    trigger: 'axis',
    backgroundColor: THEME.bg,
    borderColor: THEME.border,
    borderWidth: 1,
    textStyle: { color: THEME.text, fontSize: 13, fontFamily: 'Inter, sans-serif' },
    axisPointer: { type: 'cross', crossStyle: { color: THEME.textMuted } },
  };
}

function buildLegend() {
  return {
    bottom: 0,
    textStyle: { color: THEME.textSecondary, fontSize: 12, fontFamily: 'Inter, sans-serif' },
    itemGap: 16,
    type: 'scroll',
    pageTextStyle: { color: THEME.textSecondary },
  };
}

function buildXAxis(timestamps) {
  return {
    type: 'category',
    data: timestamps,
    axisLabel: {
      color: THEME.textMuted,
      fontSize: 11,
      fontFamily: 'Inter, sans-serif',
      formatter: val => {
        const d = new Date(val);
        return d.getHours().toString().padStart(2, '0') + ':' +
               d.getMinutes().toString().padStart(2, '0');
      }
    },
    axisLine: { lineStyle: { color: THEME.axisLine } },
    axisTick: { show: false },
    boundaryGap: false,
  };
}

function buildYAxis(name, formatter, min, max) {
  const config = {
    type: 'value',
    name,
    axisLabel: { color: THEME.textMuted, formatter, fontSize: 11, fontFamily: 'Inter, sans-serif' },
    nameTextStyle: { color: THEME.textSecondary, fontSize: 12, fontFamily: 'Inter, sans-serif' },
    axisLine: { show: false },
    splitLine: { lineStyle: { color: THEME.gridLine } },
  };
  if (min !== undefined) config.min = min;
  if (max !== undefined) config.max = max;
  return config;
}

function buildDataZoom() {
  return [
    { type: 'inside', minSpan: 5 },
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
  ];
}

function buildBrush() {
  return {
    toolbox: ['lineX'],
    xAxisIndex: 0,
    brushStyle: { color: 'rgba(59,130,246,0.1)', borderColor: '#3b82f6' },
    throttleType: 'debounce',
    throttleDelay: 300,
  };
}

function handleBrushEnd(params) {
  if (!params.areas || params.areas.length === 0) return;
  const area = params.areas[0];
  if (!area.coordRange) return;

  const chart = charts.power;
  if (!chart) return;

  const option = chart.getOption();
  const dataLen = option.xAxis[0].data ? option.xAxis[0].data.length : 100;
  const start = (area.coordRange[0] / dataLen) * 100;
  const end = (area.coordRange[1] / dataLen) * 100;

  chart.dispatchAction({ type: 'dataZoom', start: Math.max(0, start), end: Math.min(100, end) });
  chart.dispatchAction({ type: 'brush', command: 'clear', areas: [] });

  showResetButton();
}

function handleDataZoom(params) {
  // Show reset button if zoomed in
  if (params.start !== undefined && params.end !== undefined) {
    if (params.start > 0 || params.end < 100) {
      showResetButton();
    } else {
      hideResetButton();
    }
  }
}

// --- UI helpers ---

function toggleLoading(show) {
  const el = document.querySelector('[data-testid="chart-loading"]');
  if (el) el.hidden = !show;
}

function toggleEmpty(show, withRetry = false) {
  const el = document.querySelector('[data-testid="chart-empty"]');
  if (!el) return;
  el.hidden = !show;
  if (show && withRetry) {
    el.innerHTML = '<p>Failed to load data</p><button class="history-chart__retry-btn" data-testid="chart-retry">Retry</button>';
    el.querySelector('[data-testid="chart-retry"]')?.addEventListener('click', () => {
      if (currentRange) loadData(currentRange, null);
    });
  } else if (show) {
    el.innerHTML = '<p>No data for this period</p>';
  }
}

function showResetButton() {
  let btn = document.querySelector('[data-testid="chart-reset-zoom"]');
  if (!btn) {
    const toolbar = document.querySelector('[data-testid="chart-toolbar"]');
    if (!toolbar) return;
    btn = document.createElement('button');
    btn.className = 'history-chart__toolbar-btn';
    btn.setAttribute('data-testid', 'chart-reset-zoom');
    btn.textContent = 'Reset Zoom';
    btn.addEventListener('click', resetZoom);
    toolbar.prepend(btn);
  }
  btn.hidden = false;
}

function hideResetButton() {
  const btn = document.querySelector('[data-testid="chart-reset-zoom"]');
  if (btn) btn.hidden = true;
}

function showToast(message) {
  const chartEl = containers.power?.closest('.history-chart');
  if (!chartEl) return;

  let toast = chartEl.querySelector('.history-chart__toast');
  if (!toast) {
    toast = document.createElement('div');
    toast.className = 'history-chart__toast';
    toast.setAttribute('role', 'alert');
    chartEl.appendChild(toast);
  }
  toast.textContent = message;
  toast.classList.remove('history-chart__toast--hidden');

  setTimeout(() => {
    toast.classList.add('history-chart__toast--hidden');
  }, 4000);
}
