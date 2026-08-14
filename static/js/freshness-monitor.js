/**
 * freshness-monitor.js — Data freshness tracking, stale/disconnect indicators
 * ES module per dec-es-modules-no-bundler
 * Thresholds per BR-FRESH-01: 10s stale
 */

import { pause as pauseAnimator, resume as resumeAnimator } from './value-animator.js';

const STALE_THRESHOLD = 10000; // 10 seconds per BR-FRESH-01
const CHECK_INTERVAL = 1000;   // 1 second checks

let lastSuccessTimestamp = Date.now();
let isStale = false;
let isDisconnected = false;
let checkIntervalId = null;

/**
 * Called on successful poll response.
 */
function onPollSuccess() {
  lastSuccessTimestamp = Date.now();

  if (isStale) clearStale();
  if (isDisconnected) clearDisconnected();
}

/**
 * Called on network error (htmx:sendError).
 */
function onSendError() {
  if (!isDisconnected) {
    markDisconnected();
  }
}

/**
 * Periodic freshness check (every 1s).
 */
function checkFreshness() {
  const elapsed = Date.now() - lastSuccessTimestamp;

  if (elapsed > STALE_THRESHOLD && !isStale && !isDisconnected) {
    markStale();
  }
}

/**
 * Mark cards container as stale (per BR-FRESH-02).
 */
function markStale() {
  isStale = true;
  const container = document.getElementById('cards-content');
  if (!container) return;

  container.classList.add('freshness--stale');

  // Show freshness badges on all cards
  container.querySelectorAll('.freshness-badge').forEach((badge) => {
    badge.hidden = false;
  });

  // Pause value animations per BR-VALUE-03
  pauseAnimator();
}

/**
 * Mark as disconnected (per BR-FRESH-03).
 */
function markDisconnected() {
  isDisconnected = true;
  const banner = document.getElementById('disconnect-banner');
  if (banner) {
    banner.hidden = false;
    banner.setAttribute('role', 'alert');
  }

  const container = document.getElementById('cards-content');
  if (container) {
    container.classList.add('freshness--disconnected');
  }

  // Pause animations
  pauseAnimator();
}

/**
 * Clear stale state (per BR-FRESH-04).
 */
function clearStale() {
  isStale = false;
  const container = document.getElementById('cards-content');
  if (!container) return;

  container.classList.remove('freshness--stale');

  // Hide freshness badges
  container.querySelectorAll('.freshness-badge').forEach((badge) => {
    badge.hidden = true;
  });

  // Resume animations
  resumeAnimator();
}

/**
 * Clear disconnected state (per BR-FRESH-04).
 */
function clearDisconnected() {
  isDisconnected = false;
  const banner = document.getElementById('disconnect-banner');
  if (banner) {
    banner.hidden = true;
  }

  const container = document.getElementById('cards-content');
  if (container) {
    container.classList.remove('freshness--disconnected');
  }

  // Resume animations
  resumeAnimator();
}

/**
 * Initialize freshness monitor.
 */
function init() {
  const container = document.getElementById('cards-content');
  if (!container) return;

  // The HTMX swap target is #cards-container (the parent <section>)
  const swapTarget = document.getElementById('cards-container');

  // Listen for successful HTMX poll completions on swap target
  if (swapTarget) {
    swapTarget.addEventListener('htmx:afterSwap', onPollSuccess);
  }

  // Listen for network errors (bubbles to body)
  document.body.addEventListener('htmx:sendError', onSendError);

  // Start periodic freshness check
  checkIntervalId = setInterval(checkFreshness, CHECK_INTERVAL);
}

// Initialize when DOM is ready
if (document.readyState === 'loading') {
  document.addEventListener('DOMContentLoaded', init);
} else {
  init();
}

export { init, isStale, isDisconnected };
