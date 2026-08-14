/**
 * history-comparison.js — Day-over-day and multi-metric comparison for history charts.
 * Native ES module per dec-es-modules-no-bundler.
 */

const MAX_METRICS = 4;

const METRIC_OPTIONS = [
  { key: 'pv_power', label: 'PV Power' },
  { key: 'grid_power', label: 'Grid Power' },
  { key: 'battery_power', label: 'Battery Power' },
  { key: 'home_power', label: 'Home Power' },
  { key: 'battery_soc', label: 'Battery SoC' },
];

/**
 * Initialize comparison controls in the toolbar.
 * @param {HTMLElement} toolbarEl
 * @param {{ enableComparison: Function, enableMultiMetric: Function, disableComparison: Function }} chartManager
 * @returns {{ destroy: Function }}
 */
export function initComparison(toolbarEl, chartManager) {
  if (!toolbarEl) return { destroy() {} };

  let compareActive = false;
  let multiMetricActive = false;
  let currentBaseRange = null;

  // Listen for date-change to track current range
  const calendarEl = document.getElementById('history-calendar');
  if (calendarEl) {
    calendarEl.addEventListener('date-change', (e) => {
      currentBaseRange = e.detail;
      // If comparison is active, deactivate on date change
      if (compareActive) deactivateCompare();
    });
  }

  // Build comparison toggle
  const compareBtn = createToggle('Compare Days', 'compare-toggle');
  compareBtn.addEventListener('click', () => {
    if (compareActive) {
      deactivateCompare();
    } else {
      activateCompare();
    }
  });
  toolbarEl.appendChild(compareBtn);

  // Build multi-metric toggle
  const multiBtn = createToggle('Multi-Metric', 'multi-metric-toggle');
  multiBtn.addEventListener('click', () => {
    if (multiMetricActive) {
      deactivateMultiMetric();
    } else {
      activateMultiMetric();
    }
  });
  toolbarEl.appendChild(multiBtn);

  // Comparison date input (hidden until activated)
  const dateInputContainer = document.createElement('div');
  dateInputContainer.hidden = true;
  dateInputContainer.setAttribute('data-testid', 'compare-date-container');
  const dateInput = document.createElement('input');
  dateInput.type = 'date';
  dateInput.className = 'history-comparison__date-input';
  dateInput.setAttribute('data-testid', 'compare-date-input');
  dateInput.setAttribute('aria-label', 'Comparison date');
  dateInput.addEventListener('change', handleCompareDateChange);
  dateInputContainer.appendChild(dateInput);
  toolbarEl.appendChild(dateInputContainer);

  // Multi-metric selector (hidden until activated)
  const metricSelector = document.createElement('fieldset');
  metricSelector.className = 'history-comparison__metrics';
  metricSelector.setAttribute('data-testid', 'metric-selector');
  metricSelector.hidden = true;
  const legend = document.createElement('legend');
  legend.className = 'visually-hidden';
  legend.textContent = 'Select metrics to compare';
  metricSelector.appendChild(legend);

  METRIC_OPTIONS.forEach(opt => {
    const label = document.createElement('label');
    const checkbox = document.createElement('input');
    checkbox.type = 'checkbox';
    checkbox.value = opt.key;
    checkbox.checked = opt.key === 'pv_power';
    checkbox.setAttribute('data-testid', `metric-checkbox-${opt.key}`);
    checkbox.addEventListener('change', handleMetricChange);
    const span = document.createElement('span');
    span.textContent = opt.label;
    label.append(checkbox, span);
    metricSelector.appendChild(label);
  });
  toolbarEl.appendChild(metricSelector);

  function activateCompare() {
    // Deactivate multi-metric if active
    if (multiMetricActive) deactivateMultiMetric();

    compareActive = true;
    compareBtn.classList.add('history-comparison__toggle--active');
    compareBtn.setAttribute('aria-pressed', 'true');
    dateInputContainer.hidden = false;

    // Set default compare date to yesterday relative to current selection
    if (currentBaseRange) {
      const baseDate = new Date(currentBaseRange.from * 1000);
      const yesterday = new Date(baseDate);
      yesterday.setDate(yesterday.getDate() - 1);
      dateInput.value = formatISO(yesterday);
      // Trigger comparison immediately
      handleCompareDateChange();
    }
  }

  function deactivateCompare() {
    compareActive = false;
    compareBtn.classList.remove('history-comparison__toggle--active');
    compareBtn.setAttribute('aria-pressed', 'false');
    dateInputContainer.hidden = true;
    dateInput.value = '';

    if (chartManager.disableComparison) {
      chartManager.disableComparison();
    }
  }

  function activateMultiMetric() {
    // Deactivate compare if active
    if (compareActive) deactivateCompare();

    multiMetricActive = true;
    multiBtn.classList.add('history-comparison__toggle--active');
    multiBtn.setAttribute('aria-pressed', 'true');
    metricSelector.hidden = false;

    // Trigger with initially checked metrics
    handleMetricChange();
  }

  function deactivateMultiMetric() {
    multiMetricActive = false;
    multiBtn.classList.remove('history-comparison__toggle--active');
    multiBtn.setAttribute('aria-pressed', 'false');
    metricSelector.hidden = true;

    if (chartManager.disableComparison) {
      chartManager.disableComparison();
    }
  }

  function handleCompareDateChange() {
    if (!dateInput.value || !currentBaseRange) return;

    const compareDate = new Date(dateInput.value + 'T00:00:00');
    const compareFrom = Math.floor(compareDate.getTime() / 1000);
    const compareTo = compareFrom + 86400;

    if (chartManager.enableComparison) {
      chartManager.enableComparison(
        currentBaseRange,
        { from: compareFrom, to: compareTo }
      );
    }
  }

  function handleMetricChange() {
    const checked = Array.from(metricSelector.querySelectorAll('input[type="checkbox"]'));
    const selectedKeys = checked.filter(cb => cb.checked).map(cb => cb.value);

    // Enforce max 4 metrics
    const unchecked = checked.filter(cb => !cb.checked);
    unchecked.forEach(cb => {
      if (selectedKeys.length >= MAX_METRICS) {
        cb.disabled = true;
        cb.parentElement.title = `Maximum ${MAX_METRICS} metrics at once`;
      } else {
        cb.disabled = false;
        cb.parentElement.title = '';
      }
    });

    if (selectedKeys.length > 0 && chartManager.enableMultiMetric) {
      chartManager.enableMultiMetric(selectedKeys);
    }
  }

  function destroy() {
    compareBtn.remove();
    multiBtn.remove();
    dateInputContainer.remove();
    metricSelector.remove();
  }

  return { destroy };
}

// --- Utilities ---

function createToggle(text, testId) {
  const btn = document.createElement('button');
  btn.className = 'history-comparison__toggle';
  btn.textContent = text;
  btn.setAttribute('data-testid', testId);
  btn.setAttribute('aria-pressed', 'false');
  return btn;
}

function formatISO(d) {
  return `${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, '0')}-${String(d.getDate()).padStart(2, '0')}`;
}
