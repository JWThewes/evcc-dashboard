/**
 * value-animator.js — Numeric value tween animations on morph updates
 * ES module per dec-es-modules-no-bundler
 * Compositor-safe (rAF text updates only, no layout triggers)
 */

const ANIM_DURATION = 300; // Per BR-VALUE-01
let observer = null;
let initialized = false;

/**
 * Animate a numeric value from old to new using requestAnimationFrame.
 */
function animateValue(el, from, to, duration) {
  // Skip if paused (stale state per BR-VALUE-03)
  if (el.dataset.animatePaused === 'true') {
    el.textContent = formatValue(to, el);
    return;
  }

  const suffix = el.dataset.valueSuffix || '';
  const decimals = parseInt(el.dataset.valueDecimals, 10) || 0;
  let startTime = null;

  function step(timestamp) {
    if (!startTime) startTime = timestamp;
    const progress = Math.min((timestamp - startTime) / duration, 1.0);

    // Ease-out cubic for natural deceleration
    const eased = 1 - Math.pow(1 - progress, 3);
    const current = from + (to - from) * eased;

    el.textContent = current.toFixed(decimals) + suffix;

    if (progress < 1.0) {
      requestAnimationFrame(step);
    }
  }

  requestAnimationFrame(step);
}

/**
 * Format a value with its suffix and decimal precision.
 */
function formatValue(value, el) {
  const suffix = el.dataset.valueSuffix || '';
  const decimals = parseInt(el.dataset.valueDecimals, 10) || 0;
  return value.toFixed(decimals) + suffix;
}

/**
 * Parse numeric value from element text, stripping suffix.
 */
function parseNumeric(text) {
  const cleaned = text.replace(/[^0-9.\-]/g, '');
  return parseFloat(cleaned);
}

/**
 * Handle mutations detected by MutationObserver after idiomorph morph.
 */
function onMorphUpdate(mutations) {
  for (const mutation of mutations) {
    let target;
    if (mutation.type === 'characterData') {
      target = mutation.target.parentElement;
    } else if (mutation.type === 'childList') {
      target = mutation.target;
    } else {
      continue;
    }

    // Find closest animatable element
    const el = target && target.closest
      ? target.closest('[data-animate-value]')
      : null;
    if (!el) continue;

    const newText = el.textContent.trim();
    const newValue = parseNumeric(newText);
    if (isNaN(newValue)) continue;

    const oldValue = parseFloat(el.dataset.currentValue);

    // BR-VALUE-05: Skip animation on first load (no previous value recorded)
    if (isNaN(oldValue) || el.dataset.currentValue === undefined) {
      el.dataset.currentValue = String(newValue);
      continue;
    }

    if (oldValue === newValue) continue;

    // Animate the transition
    animateValue(el, oldValue, newValue, ANIM_DURATION);
    el.dataset.currentValue = String(newValue);
  }
}

/**
 * Initialize value animator with MutationObserver.
 */
function init() {
  if (initialized) return;

  const container = document.getElementById('cards-content');
  if (!container) return;

  // Set initial values
  const elements = container.querySelectorAll('[data-animate-value]');
  elements.forEach((el) => {
    const text = el.textContent.trim();
    const value = parseNumeric(text);
    if (!isNaN(value)) {
      el.dataset.currentValue = String(value);
    }
  });

  // Observe text changes within the container
  observer = new MutationObserver(onMorphUpdate);
  observer.observe(container, {
    childList: true,
    characterData: true,
    subtree: true,
  });

  initialized = true;
}

/**
 * Pause all animations (called by freshness monitor on stale).
 */
function pause() {
  const container = document.getElementById('cards-content');
  if (!container) return;
  container.querySelectorAll('[data-animate-value]').forEach((el) => {
    el.dataset.animatePaused = 'true';
  });
}

/**
 * Resume all animations (called by freshness monitor on recovery).
 */
function resume() {
  const container = document.getElementById('cards-content');
  if (!container) return;
  container.querySelectorAll('[data-animate-value]').forEach((el) => {
    delete el.dataset.animatePaused;
  });
}

// Initialize when DOM is ready
if (document.readyState === 'loading') {
  document.addEventListener('DOMContentLoaded', init);
} else {
  init();
}

export { init, pause, resume };
