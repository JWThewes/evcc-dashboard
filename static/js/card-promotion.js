/**
 * card-promotion.js — Card promotion state machine, FLIP reorder, expand/collapse
 * ES module per dec-es-modules-no-bundler
 * Compositor-only animations per dec-compositor-only-animations
 */

const DEMOTION_DELAY = 30000; // 30 seconds per BR-PROMO-02
const demotionTimers = new WeakMap();

/**
 * Evaluate promotion state for all cards after each morph cycle.
 * Reads data-activity attribute (server-rendered per contract-card-data-attributes).
 */
function evaluatePromotions(container) {
  const cards = container.querySelectorAll('[data-card-id]');

  cards.forEach((card) => {
    const activity = card.getAttribute('data-activity');
    const wasPromoted = card.classList.contains('card--promoted');
    const isDemoting = card.classList.contains('card--demoting');

    if (activity === 'active' && !wasPromoted && !isDemoting) {
      // New promotion
      clearDemotionTimer(card);
      promote(card, container);
    } else if (activity === 'active' && isDemoting) {
      // Re-promotion during demotion countdown
      clearDemotionTimer(card);
      card.classList.remove('card--demoting');
      card.classList.add('card--promoted');
    } else if (activity === 'inactive' && wasPromoted) {
      // Event resolved — begin demotion countdown
      startDemotionTimer(card, container);
      card.classList.remove('card--promoted');
      card.classList.add('card--demoting');
    }
  });
}

/**
 * FLIP animate card to top promoted position.
 */
function promote(card, container) {
  const allCards = Array.from(container.querySelectorAll('[data-card-id]'));

  // FIRST: Record current positions
  const positions = allCards.map((c) => c.getBoundingClientRect());

  // DOM MUTATION: Move to first position
  container.prepend(card);
  card.classList.remove('card--dormant', 'card--demoting');
  card.classList.add('card--promoted');

  // LAST + INVERT + PLAY
  allCards.forEach((c, i) => {
    const newRect = c.getBoundingClientRect();
    const deltaX = positions[i].left - newRect.left;
    const deltaY = positions[i].top - newRect.top;

    if (deltaX === 0 && deltaY === 0) return;

    c.style.transform = `translate(${deltaX}px, ${deltaY}px)`;
    c.style.transition = 'none';

    requestAnimationFrame(() => {
      c.style.transition = 'transform 300ms var(--anim-ease-out)';
      c.style.transform = '';

      c.addEventListener('transitionend', function cleanup(e) {
        if (e.propertyName === 'transform') {
          c.style.transition = '';
          c.removeEventListener('transitionend', cleanup);
        }
      });
    });
  });
}

/**
 * FLIP animate card back to natural grid position on demotion.
 */
function demote(card, container) {
  const allCards = Array.from(container.querySelectorAll('[data-card-id]'));
  const positions = allCards.map((c) => c.getBoundingClientRect());

  // Reset classes
  card.classList.remove('card--promoted', 'card--demoting');
  card.classList.add('card--dormant');

  // Cards already in correct DOM order (grid handles natural position)
  allCards.forEach((c, i) => {
    const newRect = c.getBoundingClientRect();
    const deltaX = positions[i].left - newRect.left;
    const deltaY = positions[i].top - newRect.top;

    if (deltaX === 0 && deltaY === 0) return;

    c.style.transform = `translate(${deltaX}px, ${deltaY}px)`;
    c.style.transition = 'none';

    requestAnimationFrame(() => {
      c.style.transition = 'transform 300ms var(--anim-ease-out)';
      c.style.transform = '';

      c.addEventListener('transitionend', function cleanup(e) {
        if (e.propertyName === 'transform') {
          c.style.transition = '';
          c.removeEventListener('transitionend', cleanup);
        }
      });
    });
  });
}

function startDemotionTimer(card, container) {
  const timerId = setTimeout(() => {
    demote(card, container);
    demotionTimers.delete(card);
  }, DEMOTION_DELAY);
  demotionTimers.set(card, timerId);
}

function clearDemotionTimer(card) {
  if (demotionTimers.has(card)) {
    clearTimeout(demotionTimers.get(card));
    demotionTimers.delete(card);
  }
}

/**
 * Expand/collapse toggle logic.
 */
function toggleExpand(card) {
  const isExpanded = card.classList.contains('card--expanded');
  const detail = card.querySelector('.card__detail');
  const btn = card.querySelector('.card__expand-btn');

  if (!detail || !btn) return;

  if (isExpanded) {
    // Collapse
    detail.style.maxHeight = detail.scrollHeight + 'px';
    requestAnimationFrame(() => {
      detail.style.maxHeight = '0';
      detail.style.opacity = '0';
    });
    card.classList.remove('card--expanded');
    btn.setAttribute('aria-expanded', 'false');
  } else {
    // Expand
    card.classList.add('card--expanded');
    btn.setAttribute('aria-expanded', 'true');
    detail.style.maxHeight = '0';
    detail.style.opacity = '0';
    requestAnimationFrame(() => {
      detail.style.maxHeight = detail.scrollHeight + 'px';
      detail.style.opacity = '1';
    });
    detail.addEventListener('transitionend', function cleanup(e) {
      if (e.propertyName === 'max-height') {
        detail.style.maxHeight = '';
        detail.removeEventListener('transitionend', cleanup);
      }
    });
  }
}

/**
 * Re-apply max-height after morph for expanded cards (idiomorph may strip inline styles).
 */
function restoreExpandState(container) {
  const expandedCards = container.querySelectorAll('.card--expanded .card__detail');
  expandedCards.forEach((detail) => {
    if (!detail.style.maxHeight || detail.style.maxHeight === '0px') {
      detail.style.maxHeight = detail.scrollHeight + 'px';
      detail.style.opacity = '1';
    }
  });
}

/**
 * Initialize card interactions on the cards container.
 */
function init() {
  const container = document.getElementById('cards-content');
  if (!container) return;

  // The HTMX swap target is #cards-container (the parent <section>),
  // so we listen for htmx:afterSettle on the parent which fires on the swap target
  const swapTarget = document.getElementById('cards-container');

  // Expand/collapse click handler (delegated)
  container.addEventListener('click', (e) => {
    const btn = e.target.closest('.card__expand-btn');
    if (!btn) return;
    const card = btn.closest('.card');
    if (card) toggleExpand(card);
  });

  // Evaluate promotions after each morph cycle
  if (swapTarget) {
    swapTarget.addEventListener('htmx:afterSettle', () => {
      evaluatePromotions(container);
      restoreExpandState(container);
    });
  }

  // Initial evaluation
  evaluatePromotions(container);
}

// Initialize when DOM is ready
if (document.readyState === 'loading') {
  document.addEventListener('DOMContentLoaded', init);
} else {
  init();
}

export { evaluatePromotions, toggleExpand, init };
