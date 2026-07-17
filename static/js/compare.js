/**
 * Compare view controller — comparison-feature unit.
 *
 * Manages a client-side state machine for the period comparison view:
 * - Fetches summary and chart data from /api/compare/ endpoints
 * - Renders comparison table with delta indicators
 * - Manages ECharts overlay chart lifecycle (contract-echarts-theme-name: "evcc-energy")
 * - Handles window/metric switching and chart toggle
 *
 * No eval, no innerHTML, no external network requests.
 * Graceful degradation: fetch failures retain last DOM state with console.warn.
 */
(function () {
    'use strict';

    // State
    var activeWindow = 'day';
    var activeMetric = 'solar';
    var chartVisible = false;
    var chartInstance = null;
    var isLoading = false;

    // Metric display config
    var METRICS = [
        { key: 'solar_kwh', label: 'Solar Production', unit: 'kWh' },
        { key: 'consumption_kwh', label: 'Consumption', unit: 'kWh' },
        { key: 'grid_import_kwh', label: 'Grid Import', unit: 'kWh' },
        { key: 'grid_export_kwh', label: 'Grid Export', unit: 'kWh' },
        { key: 'ev_charging_kwh', label: 'EV Charging', unit: 'kWh' },
        { key: 'self_sufficiency_pct', label: 'Self-Sufficiency', unit: '%' }
    ];

    // DOM references
    var tableBody = null;
    var chartContainer = null;
    var chartSection = null;
    var insufficientNotice = null;

    /**
     * Initialize the compare controller on DOMContentLoaded.
     */
    function init() {
        tableBody = document.getElementById('compare-table-body');
        chartContainer = document.getElementById('compare-chart');
        chartSection = document.querySelector('[data-testid="compare-chart-section"]');
        insufficientNotice = document.querySelector('[data-testid="compare-insufficient"]');

        if (!tableBody) return; // Not on compare view

        bindWindowButtons();
        bindChartToggle();
        bindMetricButtons();

        // Initial data fetch
        fetchSummary(activeWindow);
    }

    /**
     * Bind window selector button clicks.
     */
    function bindWindowButtons() {
        var buttons = document.querySelectorAll('[data-testid="compare-window-selector"] button');
        for (var i = 0; i < buttons.length; i++) {
            buttons[i].addEventListener('click', function (e) {
                var btn = e.currentTarget;
                var newWindow = btn.getAttribute('data-window');
                if (newWindow === activeWindow) return;

                // Update active state
                var siblings = btn.parentElement.querySelectorAll('button');
                for (var j = 0; j < siblings.length; j++) {
                    siblings[j].classList.remove('active');
                    siblings[j].setAttribute('aria-selected', 'false');
                }
                btn.classList.add('active');
                btn.setAttribute('aria-selected', 'true');

                activeWindow = newWindow;
                fetchSummary(activeWindow);
                if (chartVisible) {
                    fetchChart(activeWindow, activeMetric);
                }
            });
        }
    }

    /**
     * Bind chart toggle button.
     */
    function bindChartToggle() {
        var toggle = document.querySelector('[data-testid="compare-chart-toggle"]');
        if (!toggle) return;

        toggle.addEventListener('click', function () {
            chartVisible = !chartVisible;
            toggle.setAttribute('aria-pressed', chartVisible ? 'true' : 'false');
            toggle.textContent = chartVisible ? 'Hide Chart' : 'Show Chart';

            if (chartVisible) {
                chartSection.classList.remove('hidden');
                fetchChart(activeWindow, activeMetric);
            } else {
                chartSection.classList.add('hidden');
                destroyChart();
            }
        });
    }

    /**
     * Bind metric selector button clicks.
     */
    function bindMetricButtons() {
        var buttons = document.querySelectorAll('[data-testid="compare-metric-selector"] button');
        for (var i = 0; i < buttons.length; i++) {
            buttons[i].addEventListener('click', function (e) {
                var btn = e.currentTarget;
                var newMetric = btn.getAttribute('data-metric');
                if (newMetric === activeMetric) return;

                var siblings = btn.parentElement.querySelectorAll('button');
                for (var j = 0; j < siblings.length; j++) {
                    siblings[j].classList.remove('active');
                    siblings[j].setAttribute('aria-selected', 'false');
                }
                btn.classList.add('active');
                btn.setAttribute('aria-selected', 'true');

                activeMetric = newMetric;
                if (chartVisible) {
                    fetchChart(activeWindow, activeMetric);
                }
            });
        }
    }

    /**
     * Fetch summary data and populate the comparison table.
     */
    function fetchSummary(window) {
        isLoading = true;
        showLoading();

        fetch('/api/compare/summary?window=' + encodeURIComponent(window))
            .then(function (res) {
                if (!res.ok) throw new Error('HTTP ' + res.status);
                return res.json();
            })
            .then(function (data) {
                renderTable(data);
                isLoading = false;
            })
            .catch(function (err) {
                console.warn('[compare] Summary fetch failed:', err.message);
                isLoading = false;
            });
    }

    /**
     * Fetch chart data and render the overlay chart.
     */
    function fetchChart(window, metric) {
        var url = '/api/compare/chart?window=' + encodeURIComponent(window) +
                  '&metric=' + encodeURIComponent(metric);

        fetch(url)
            .then(function (res) {
                if (!res.ok) throw new Error('HTTP ' + res.status);
                return res.json();
            })
            .then(function (data) {
                renderChart(data, metric);
            })
            .catch(function (err) {
                console.warn('[compare] Chart fetch failed:', err.message);
            });
    }

    /**
     * Show loading state in the table body.
     */
    function showLoading() {
        if (!tableBody) return;
        tableBody.textContent = '';
        var tr = document.createElement('tr');
        var td = document.createElement('td');
        td.setAttribute('colspan', '4');
        td.className = 'compare-loading';
        td.textContent = 'Loading...';
        tr.appendChild(td);
        tableBody.appendChild(tr);
    }

    /**
     * Render the comparison table from summary response data.
     */
    function renderTable(data) {
        if (!tableBody) return;
        tableBody.textContent = '';

        var current = data.current_period;
        var previous = data.previous_period;
        var allZero = true;

        for (var i = 0; i < METRICS.length; i++) {
            var m = METRICS[i];
            var curVal = current[m.key] || 0;
            var prevVal = previous[m.key] || 0;

            if (curVal !== 0 || prevVal !== 0) allZero = false;

            var tr = document.createElement('tr');

            // Metric label
            var tdLabel = document.createElement('td');
            tdLabel.className = 'compare-metric-label';
            tdLabel.textContent = m.label;
            tr.appendChild(tdLabel);

            // Current value
            var tdCurrent = document.createElement('td');
            tdCurrent.className = 'compare-value';
            tdCurrent.textContent = formatValue(curVal, m.unit);
            tr.appendChild(tdCurrent);

            // Previous value
            var tdPrevious = document.createElement('td');
            tdPrevious.className = 'compare-value compare-col-previous';
            tdPrevious.textContent = formatValue(prevVal, m.unit);
            tr.appendChild(tdPrevious);

            // Delta
            var tdDelta = document.createElement('td');
            var delta = computeDelta(curVal, prevVal);
            tdDelta.className = 'compare-delta ' + deltaClass(delta);
            tdDelta.textContent = formatDelta(delta);
            tdDelta.setAttribute('aria-label', m.label + ' change: ' + formatDelta(delta));
            tr.appendChild(tdDelta);

            tableBody.appendChild(tr);
        }

        // Show/hide insufficient data notice
        if (insufficientNotice) {
            if (allZero) {
                insufficientNotice.classList.remove('hidden');
            } else {
                insufficientNotice.classList.add('hidden');
            }
        }
    }

    /**
     * Render the overlay chart using ECharts.
     */
    function renderChart(data, metric) {
        if (!chartContainer) return;
        if (typeof echarts === 'undefined') {
            console.warn('[compare] ECharts not available');
            return;
        }

        destroyChart();

        chartInstance = echarts.init(chartContainer, 'evcc-energy');

        // Shift previous series timestamps forward for overlay alignment
        var periodOffset = 0;
        if (data.current_series.length > 0 && data.previous_series.length > 0) {
            periodOffset = data.current_series[0].timestamp - data.previous_series[0].timestamp;
        }

        var currentData = data.current_series.map(function (p) {
            return [p.timestamp * 1000, p.value];
        });

        var previousData = data.previous_series.map(function (p) {
            return [(p.timestamp + periodOffset) * 1000, p.value];
        });

        var metricLabels = {
            solar: 'Solar Production',
            consumption: 'Consumption',
            grid_import: 'Grid Import',
            grid_export: 'Grid Export'
        };

        var option = {
            tooltip: {
                trigger: 'axis',
                axisPointer: { type: 'cross' }
            },
            legend: {
                data: ['Current', 'Previous'],
                bottom: 0
            },
            grid: {
                left: 50,
                right: 20,
                top: 30,
                bottom: 40
            },
            xAxis: {
                type: 'time',
                axisLabel: {
                    formatter: function (val) {
                        var d = new Date(val);
                        if (activeWindow === 'day') {
                            return d.getHours() + ':' + String(d.getMinutes()).padStart(2, '0');
                        } else if (activeWindow === 'week') {
                            var days = ['Sun', 'Mon', 'Tue', 'Wed', 'Thu', 'Fri', 'Sat'];
                            return days[d.getDay()];
                        } else {
                            return d.getDate() + '/' + (d.getMonth() + 1);
                        }
                    }
                }
            },
            yAxis: {
                type: 'value',
                name: 'Wh',
                axisLabel: {
                    formatter: function (val) {
                        if (val >= 1000) return (val / 1000).toFixed(1) + 'k';
                        return val.toFixed(0);
                    }
                }
            },
            series: [
                {
                    name: 'Current',
                    type: 'line',
                    data: currentData,
                    smooth: true,
                    showSymbol: false,
                    lineStyle: { width: 2 }
                },
                {
                    name: 'Previous',
                    type: 'line',
                    data: previousData,
                    smooth: true,
                    showSymbol: false,
                    lineStyle: { width: 2, type: 'dashed', opacity: 0.6 }
                }
            ]
        };

        chartInstance.setOption(option);

        // Resize handling
        window.addEventListener('resize', handleResize);
    }

    /**
     * Destroy the current chart instance and clean up.
     */
    function destroyChart() {
        if (chartInstance) {
            chartInstance.dispose();
            chartInstance = null;
            window.removeEventListener('resize', handleResize);
        }
    }

    function handleResize() {
        if (chartInstance) {
            chartInstance.resize();
        }
    }

    // ---------------------------------------------------------------------------
    // Utility functions
    // ---------------------------------------------------------------------------

    function formatValue(val, unit) {
        if (unit === '%') return val.toFixed(1) + '%';
        if (val >= 100) return val.toFixed(1) + ' ' + unit;
        return val.toFixed(2) + ' ' + unit;
    }

    function computeDelta(current, previous) {
        if (previous > 0) return ((current - previous) / previous) * 100;
        if (current > 0) return 100;
        return 0;
    }

    function formatDelta(delta) {
        if (delta === 0) return '—';
        var sign = delta > 0 ? '+' : '';
        return sign + delta.toFixed(1) + '%';
    }

    function deltaClass(delta) {
        if (delta > 0.5) return 'positive';
        if (delta < -0.5) return 'negative';
        return 'neutral';
    }

    // Initialize on DOM ready
    if (document.readyState === 'loading') {
        document.addEventListener('DOMContentLoaded', init);
    } else {
        init();
    }
})();
