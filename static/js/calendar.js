/**
 * calendar.js — Interactive calendar date picker for history page.
 * Native ES module per dec-es-modules-no-bundler.
 * Provides contract-date-change-event.
 */

const WEEKDAYS = ['Mon', 'Tue', 'Wed', 'Thu', 'Fri', 'Sat', 'Sun'];
const PRESETS = {
  today: { label: 'Today', days: 0 },
  yesterday: { label: 'Yesterday', days: 1 },
  '7d': { label: '7 Days', days: 6 },
  '30d': { label: '30 Days', days: 29 },
};

/**
 * @param {HTMLElement} containerEl
 * @param {{ initialDate: string, minDate?: string, maxDate?: string, presets?: string[] }} options
 * @returns {{ navigate: Function, selectDate: Function, selectRange: Function, destroy: Function }}
 */
export function init(containerEl, options = {}) {
  const state = {
    focusMonth: null,  // { year, month } 0-indexed month
    selectedDate: null, // 'YYYY-MM-DD'
    activePreset: null,
    collapsed: false,
  };

  const maxDate = options.maxDate || todayISO();
  const minDate = options.minDate || '1970-01-01';
  const presets = options.presets || ['today', 'yesterday', '7d', '30d'];

  // Parse initial date
  const initial = options.initialDate || todayISO();
  const [iy, im] = initial.split('-').map(Number);
  state.focusMonth = { year: iy, month: im - 1 };
  state.selectedDate = initial;

  // Build DOM
  render();
  setupResponsive();

  function render() {
    containerEl.innerHTML = '';
    containerEl.appendChild(buildHeader());
    containerEl.appendChild(buildWeekdayRow());
    containerEl.appendChild(buildMonthGrid());
    containerEl.appendChild(buildPresets());
    containerEl.appendChild(buildWeekstrip());
    containerEl.addEventListener('keydown', handleKeydown);
  }

  function buildHeader() {
    const header = el('div', 'history-calendar__header');
    const nav = el('div', 'history-calendar__nav');

    const prevBtn = el('button', 'history-calendar__nav-btn');
    prevBtn.setAttribute('aria-label', 'Previous month');
    prevBtn.setAttribute('data-testid', 'calendar-prev-month');
    prevBtn.textContent = '\u2039';
    prevBtn.addEventListener('click', () => navigate(-1));

    const label = el('span', 'history-calendar__month-label');
    label.setAttribute('data-testid', 'calendar-month-label');
    label.textContent = formatMonthYear(state.focusMonth.year, state.focusMonth.month);

    const nextBtn = el('button', 'history-calendar__nav-btn');
    nextBtn.setAttribute('aria-label', 'Next month');
    nextBtn.setAttribute('data-testid', 'calendar-next-month');
    nextBtn.textContent = '\u203A';
    nextBtn.addEventListener('click', () => navigate(1));

    nav.append(prevBtn, label, nextBtn);
    header.appendChild(nav);
    return header;
  }

  function buildWeekdayRow() {
    const row = el('div', 'history-calendar__grid');
    row.setAttribute('role', 'row');
    WEEKDAYS.forEach(day => {
      const cell = el('span', 'history-calendar__weekday');
      cell.textContent = day;
      row.appendChild(cell);
    });
    return row;
  }

  function buildMonthGrid() {
    const grid = el('div', 'history-calendar__grid');
    grid.setAttribute('role', 'grid');
    grid.setAttribute('data-testid', 'calendar-grid');

    const { year, month } = state.focusMonth;
    const firstDay = new Date(year, month, 1).getDay();
    // Convert Sunday=0 to Monday-based: Mon=0..Sun=6
    const startOffset = (firstDay + 6) % 7;
    const daysInMonth = new Date(year, month + 1, 0).getDate();

    // Empty cells for offset
    for (let i = 0; i < startOffset; i++) {
      const empty = el('button', 'history-calendar__cell history-calendar__cell--empty');
      empty.setAttribute('aria-hidden', 'true');
      empty.disabled = true;
      grid.appendChild(empty);
    }

    // Day cells
    for (let d = 1; d <= daysInMonth; d++) {
      const dateStr = `${year}-${String(month + 1).padStart(2, '0')}-${String(d).padStart(2, '0')}`;
      const cell = el('button', 'history-calendar__cell');
      cell.textContent = d;
      cell.setAttribute('role', 'gridcell');
      cell.setAttribute('data-date', dateStr);
      cell.setAttribute('data-testid', `calendar-cell-${dateStr}`);

      const isDisabled = dateStr < minDate || dateStr > maxDate;
      const isToday = dateStr === todayISO();
      const isSelected = dateStr === state.selectedDate;

      if (isDisabled) {
        cell.classList.add('history-calendar__cell--disabled');
        cell.setAttribute('aria-disabled', 'true');
        cell.disabled = true;
      }
      if (isToday) cell.classList.add('history-calendar__cell--today');
      if (isSelected) {
        cell.classList.add('history-calendar__cell--selected');
        cell.setAttribute('aria-selected', 'true');
        cell.setAttribute('tabindex', '0');
      } else {
        cell.setAttribute('tabindex', '-1');
      }

      if (!isDisabled) {
        cell.addEventListener('click', () => handleCellClick(dateStr));
      }

      grid.appendChild(cell);
    }

    return grid;
  }

  function buildPresets() {
    const container = el('div', 'history-calendar__presets');
    container.setAttribute('data-testid', 'calendar-presets');

    presets.forEach(key => {
      const preset = PRESETS[key];
      if (!preset) return;
      const btn = el('button', 'history-calendar__preset-btn');
      btn.textContent = preset.label;
      btn.setAttribute('data-testid', `calendar-preset-${key}`);
      btn.setAttribute('data-preset', key);
      if (state.activePreset === key) {
        btn.classList.add('history-calendar__preset-btn--active');
      }
      btn.addEventListener('click', () => handlePresetClick(key));
      container.appendChild(btn);
    });

    return container;
  }

  function buildWeekstrip() {
    const strip = el('div', 'history-calendar__weekstrip');
    strip.setAttribute('data-testid', 'calendar-weekstrip');

    // Show 7 days around selected date
    const sel = state.selectedDate ? new Date(state.selectedDate + 'T00:00:00') : new Date();
    const startOfWeek = new Date(sel);
    startOfWeek.setDate(sel.getDate() - 3);

    for (let i = 0; i < 7; i++) {
      const d = new Date(startOfWeek);
      d.setDate(startOfWeek.getDate() + i);
      const dateStr = formatISO(d);

      const cell = el('button', 'history-calendar__cell');
      cell.textContent = d.getDate();
      cell.setAttribute('data-date', dateStr);

      const isDisabled = dateStr < minDate || dateStr > maxDate;
      if (isDisabled) {
        cell.classList.add('history-calendar__cell--disabled');
        cell.disabled = true;
      }
      if (dateStr === state.selectedDate) {
        cell.classList.add('history-calendar__cell--selected');
      }
      if (!isDisabled) {
        cell.addEventListener('click', () => handleCellClick(dateStr));
      }

      strip.appendChild(cell);
    }

    return strip;
  }

  function handleCellClick(dateStr) {
    state.selectedDate = dateStr;
    state.activePreset = null;

    const d = new Date(dateStr + 'T00:00:00');
    const from = Math.floor(d.getTime() / 1000);
    const to = from + 86400;
    emitDateChange(from, to, formatLabel(d));
    render();
  }

  function handlePresetClick(key) {
    state.activePreset = key;
    const now = new Date();
    const todayMidnight = new Date(now.getFullYear(), now.getMonth(), now.getDate());
    let from, to, label;

    switch (key) {
      case 'today':
        from = Math.floor(todayMidnight.getTime() / 1000);
        to = Math.floor(now.getTime() / 1000);
        label = 'Today';
        state.selectedDate = todayISO();
        break;
      case 'yesterday': {
        const yest = new Date(todayMidnight);
        yest.setDate(yest.getDate() - 1);
        from = Math.floor(yest.getTime() / 1000);
        to = Math.floor(todayMidnight.getTime() / 1000);
        label = 'Yesterday';
        state.selectedDate = formatISO(yest);
        break;
      }
      case '7d': {
        const start = new Date(todayMidnight);
        start.setDate(start.getDate() - 6);
        from = Math.floor(start.getTime() / 1000);
        to = Math.floor(now.getTime() / 1000);
        label = '7d';
        state.selectedDate = todayISO();
        break;
      }
      case '30d': {
        const start = new Date(todayMidnight);
        start.setDate(start.getDate() - 29);
        from = Math.floor(start.getTime() / 1000);
        to = Math.floor(now.getTime() / 1000);
        label = '30d';
        state.selectedDate = todayISO();
        break;
      }
      default:
        return;
    }

    emitDateChange(from, to, label);
    // Refocus to month of selected date
    const [y, m] = state.selectedDate.split('-').map(Number);
    state.focusMonth = { year: y, month: m - 1 };
    render();
  }

  function navigate(direction) {
    let { year, month } = state.focusMonth;
    month += direction;
    if (month < 0) { month = 11; year--; }
    if (month > 11) { month = 0; year++; }

    // Clamp to data range
    const focusStr = `${year}-${String(month + 1).padStart(2, '0')}-01`;
    const lastOfMonth = `${year}-${String(month + 1).padStart(2, '0')}-${new Date(year, month + 1, 0).getDate()}`;
    if (lastOfMonth < minDate || focusStr > maxDate) return;

    state.focusMonth = { year, month };
    render();
  }

  function handleKeydown(event) {
    const active = containerEl.querySelector('[aria-selected="true"]');
    if (!active) return;

    const date = active.dataset.date;
    if (!date) return;

    const d = new Date(date + 'T00:00:00');
    let newDate = null;

    switch (event.key) {
      case 'ArrowLeft': d.setDate(d.getDate() - 1); newDate = d; break;
      case 'ArrowRight': d.setDate(d.getDate() + 1); newDate = d; break;
      case 'ArrowUp': d.setDate(d.getDate() - 7); newDate = d; break;
      case 'ArrowDown': d.setDate(d.getDate() + 7); newDate = d; break;
      case 'Enter':
      case ' ':
        handleCellClick(date);
        event.preventDefault();
        return;
      default:
        return;
    }

    event.preventDefault();
    if (newDate) {
      const newStr = formatISO(newDate);
      if (newStr >= minDate && newStr <= maxDate) {
        state.selectedDate = newStr;
        // Update focus month if needed
        const [ny, nm] = newStr.split('-').map(Number);
        if (ny !== state.focusMonth.year || (nm - 1) !== state.focusMonth.month) {
          state.focusMonth = { year: ny, month: nm - 1 };
        }
        render();
        // Focus the new cell
        const newCell = containerEl.querySelector(`[data-date="${newStr}"]`);
        if (newCell) newCell.focus();
      }
    }
  }

  function emitDateChange(from, to, label) {
    containerEl.dispatchEvent(new CustomEvent('date-change', {
      bubbles: true,
      detail: { from, to, label }
    }));
  }

  function setupResponsive() {
    const mq = window.matchMedia('(max-width: 767px)');
    function handleMQ(e) {
      state.collapsed = e.matches;
      containerEl.setAttribute('data-collapsed', e.matches ? 'true' : 'false');
    }
    handleMQ(mq);
    mq.addEventListener('change', handleMQ);
  }

  // Public API
  return {
    navigate,
    selectDate(dateStr) { handleCellClick(dateStr); },
    selectRange(key) { handlePresetClick(key); },
    destroy() {
      containerEl.removeEventListener('keydown', handleKeydown);
      containerEl.innerHTML = '';
    }
  };
}

// --- Utilities ---

function el(tag, className) {
  const e = document.createElement(tag);
  if (className) e.className = className;
  return e;
}

function todayISO() {
  const d = new Date();
  return formatISO(d);
}

function formatISO(d) {
  return `${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, '0')}-${String(d.getDate()).padStart(2, '0')}`;
}

function formatMonthYear(year, month) {
  const months = ['January', 'February', 'March', 'April', 'May', 'June',
                  'July', 'August', 'September', 'October', 'November', 'December'];
  return `${months[month]} ${year}`;
}

function formatLabel(d) {
  const months = ['Jan', 'Feb', 'Mar', 'Apr', 'May', 'Jun',
                  'Jul', 'Aug', 'Sep', 'Oct', 'Nov', 'Dec'];
  return `${d.getDate()} ${months[d.getMonth()]}`;
}
