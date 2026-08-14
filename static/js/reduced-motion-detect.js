/**
 * Reduced Motion Detection — toggles SVG particles/arrows
 * static/js/reduced-motion-detect.js | Budget: ≤0.5KB gzip
 */
(function() {
  'use strict';
  try {
    if (typeof window.matchMedia !== 'function') return;
    var mq = window.matchMedia('(prefers-reduced-motion: reduce)');
    function apply(isReduced) {
      var particles = document.querySelectorAll('.flow-path__particle-group');
      var arrows = document.querySelectorAll('.flow-path__arrow-group');
      if (isReduced) {
        particles.forEach(function(el) { el.style.display = 'none'; });
        arrows.forEach(function(el) { el.style.display = 'block'; });
        document.documentElement.classList.add('reduced-motion');
      } else {
        particles.forEach(function(el) { el.style.display = ''; });
        arrows.forEach(function(el) { el.style.display = ''; });
        document.documentElement.classList.remove('reduced-motion');
      }
    }
    apply(mq.matches);
    mq.addEventListener('change', function(e) { apply(e.matches); });
  } catch (_) {}
})();
