/**
 * hamburger-nav.js — Mobile navigation drawer toggle
 * Budget: ≤1KB gzipped
 */

const hamburger = document.querySelector('[data-testid="hamburger-button"]');
const drawer = document.getElementById('nav-drawer');
const overlay = document.querySelector('.nav-drawer__overlay');

function openDrawer() {
    if (!drawer || !hamburger) return;
    drawer.classList.add('nav-drawer--open');
    drawer.setAttribute('aria-hidden', 'false');
    hamburger.setAttribute('aria-expanded', 'true');
    document.body.classList.add('no-scroll');
}

function closeDrawer() {
    if (!drawer || !hamburger) return;
    drawer.classList.remove('nav-drawer--open');
    drawer.setAttribute('aria-hidden', 'true');
    hamburger.setAttribute('aria-expanded', 'false');
    document.body.classList.remove('no-scroll');
}

function toggleDrawer() {
    if (!drawer) return;
    if (drawer.classList.contains('nav-drawer--open')) {
        closeDrawer();
    } else {
        openDrawer();
    }
}

// Hamburger click
if (hamburger) {
    hamburger.addEventListener('click', toggleDrawer);
}

// Overlay click closes drawer
if (overlay) {
    overlay.addEventListener('click', closeDrawer);
}

// Escape key closes drawer
document.addEventListener('keydown', (event) => {
    if (event.key === 'Escape' && drawer && drawer.classList.contains('nav-drawer--open')) {
        closeDrawer();
        hamburger?.focus();
    }
});

// Nav link click closes drawer and navigates
if (drawer) {
    drawer.addEventListener('click', (event) => {
        if (event.target.closest('.nav-drawer__link')) {
            closeDrawer();
        }
    });
}
