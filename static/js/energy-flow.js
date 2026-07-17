/**
 * Energy Flow — Live Data Controller
 * Owner: live-data-energy-flow unit
 * 
 * Listens for htmx afterSwap events and updates particle animation speeds
 * based on power magnitude data from OOB swap fragments.
 * 
 * No globals exported. IIFE with zero dependencies beyond htmx.
 */
(function () {
  'use strict';

  var DURATION_MIN = 1500;
  var DURATION_MAX = 6000;
  var POWER_MAX = 10000;
  var NOISE_FLOOR = 10;

  /**
   * Calculate animation duration from power (inverse linear).
   * Higher power = faster (shorter duration).
   */
  function calcDuration(power) {
    if (power <= NOISE_FLOOR) return DURATION_MAX;
    var clamped = Math.min(power, POWER_MAX);
    var ratio = (clamped - NOISE_FLOOR) / (POWER_MAX - NOISE_FLOOR);
    return Math.round(DURATION_MAX - ratio * (DURATION_MAX - DURATION_MIN));
  }

  /**
   * Update particle animation speeds after an OOB swap.
   */
  function updateAnimations() {
    var paths = ['pv', 'grid', 'battery', 'ev'];
    for (var i = 0; i < paths.length; i++) {
      var pathId = paths[i];
      var dataEl = document.getElementById('ef-' + pathId + '-path-power');
      if (!dataEl) continue;

      var power = parseFloat(dataEl.getAttribute('data-power')) || 0;
      var duration = calcDuration(power);

      // Update all particles on this path
      var particles = document.querySelectorAll('.ef-particle[data-path="' + pathId + '"]');
      for (var j = 0; j < particles.length; j++) {
        var particle = particles[j];
        // Update the animateMotion dur attribute for SVG SMIL animations
        var motion = particle.querySelector('animateMotion');
        if (motion) {
          motion.setAttribute('dur', duration + 'ms');
        }
        particle.style.setProperty('--ef-duration', duration + 'ms');
      }

      // Update path line direction classes
      var dirEl = document.getElementById('ef-' + pathId + '-path-dir');
      if (dirEl) {
        var active = dirEl.getAttribute('data-active') === 'true';
        var direction = dirEl.getAttribute('data-direction') || 'ef-dir-idle';
        var pathLine = document.querySelector('.ef-path-line--' + pathId);
        if (pathLine) {
          pathLine.classList.remove('ef-dir-forward', 'ef-dir-reverse', 'ef-dir-idle');
          pathLine.classList.add(active ? direction : 'ef-dir-idle');
        }
      }
    }
  }

  /**
   * Pause polling when tab is hidden, resume on visibility.
   */
  function handleVisibility() {
    var container = document.querySelector('.ef-container');
    if (!container) return;

    if (document.hidden) {
      container.removeAttribute('hx-trigger');
    } else {
      container.setAttribute('hx-trigger', 'every 3s');
      // Trigger immediate refresh on return
      if (window.htmx) {
        window.htmx.trigger(container, 'htmx:trigger');
      }
    }
  }

  // Initialize on DOMContentLoaded
  document.addEventListener('DOMContentLoaded', function () {
    // Listen for htmx afterSwap to update animations
    document.body.addEventListener('htmx:afterSwap', function (evt) {
      // Only react to energy flow OOB swaps
      var target = evt.detail.target;
      if (target && target.id && target.id.startsWith('ef-')) {
        updateAnimations();
      }
    });

    // Also listen for OOB swaps specifically
    document.body.addEventListener('htmx:oobAfterSwap', function () {
      updateAnimations();
    });

    // Visibility change handler
    document.addEventListener('visibilitychange', handleVisibility);
  });
})();
