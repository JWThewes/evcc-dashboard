/* ============================================
   EVCC DASHBOARD — Mobile Navigation Controller
   Handles hamburger toggle, backdrop close,
   Escape key, and focus trap.
   ============================================ */

(function () {
    'use strict';

    var hamburger = document.querySelector('.site-header__hamburger');
    var overlay = document.querySelector('.mobile-nav');
    var backdrop = document.querySelector('.mobile-nav__backdrop');
    var panel = document.querySelector('.mobile-nav__panel');

    if (!hamburger || !overlay) return;

    var focusableElements;
    var firstFocusable;
    var lastFocusable;

    function openMenu() {
        overlay.classList.add('mobile-nav--open');
        overlay.setAttribute('aria-hidden', 'false');
        hamburger.setAttribute('aria-expanded', 'true');
        hamburger.setAttribute('aria-label', 'Close navigation menu');
        document.body.style.overflow = 'hidden';

        focusableElements = panel.querySelectorAll('a[href], button');
        firstFocusable = focusableElements[0];
        lastFocusable = focusableElements[focusableElements.length - 1];
        if (firstFocusable) firstFocusable.focus();
    }

    function closeMenu() {
        overlay.classList.remove('mobile-nav--open');
        overlay.setAttribute('aria-hidden', 'true');
        hamburger.setAttribute('aria-expanded', 'false');
        hamburger.setAttribute('aria-label', 'Open navigation menu');
        document.body.style.overflow = '';
        hamburger.focus();
    }

    hamburger.addEventListener('click', function () {
        var isOpen = overlay.classList.contains('mobile-nav--open');
        if (isOpen) {
            closeMenu();
        } else {
            openMenu();
        }
    });

    if (backdrop) {
        backdrop.addEventListener('click', closeMenu);
    }

    document.addEventListener('keydown', function (e) {
        if (e.key === 'Escape' && overlay.classList.contains('mobile-nav--open')) {
            closeMenu();
        }
    });

    overlay.addEventListener('keydown', function (e) {
        if (e.key !== 'Tab') return;
        if (!focusableElements || focusableElements.length === 0) return;

        if (e.shiftKey) {
            if (document.activeElement === firstFocusable) {
                e.preventDefault();
                lastFocusable.focus();
            }
        } else {
            if (document.activeElement === lastFocusable) {
                e.preventDefault();
                firstFocusable.focus();
            }
        }
    });
})();
