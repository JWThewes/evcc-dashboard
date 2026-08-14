/**
 * transitions.js — View Transitions API integration
 * Progressive enhancement: falls back to standard navigation when unsupported.
 * Respects prefers-reduced-motion.
 * Budget: ≤2KB gzipped
 */

const REDUCED_MOTION = window.matchMedia('(prefers-reduced-motion: reduce)').matches;
const SUPPORTS_VT = 'startViewTransition' in document;

/**
 * Check if a link is eligible for view transition interception.
 * @param {HTMLAnchorElement} anchor
 * @returns {boolean}
 */
function isEligibleLink(anchor) {
    if (!anchor || !anchor.href) return false;
    // Same origin only
    if (anchor.origin !== location.origin) return false;
    // Skip links with explicit targets or downloads
    if (anchor.target === '_blank' || anchor.hasAttribute('download')) return false;
    // Skip hash-only links
    if (anchor.pathname === location.pathname && anchor.hash) return false;
    return true;
}

/**
 * Perform a view transition navigation.
 * @param {string} url
 * @param {boolean} pushState
 */
async function navigateWithTransition(url, pushState = true) {
    try {
        const response = await fetch(url, {
            headers: { 'Accept': 'text/html' },
            signal: AbortSignal.timeout(500),
        });

        if (!response.ok) {
            window.location.href = url;
            return;
        }

        const html = await response.text();
        const parser = new DOMParser();
        const doc = parser.parseFromString(html, 'text/html');
        const newMain = doc.querySelector('#main-content');
        const newTitle = doc.querySelector('title');

        if (!newMain) {
            window.location.href = url;
            return;
        }

        const updateDOM = () => {
            const currentMain = document.querySelector('#main-content');
            if (currentMain) {
                currentMain.innerHTML = newMain.innerHTML;
                // Process new content for htmx
                if (window.htmx) {
                    window.htmx.process(currentMain);
                }
            }
            if (newTitle) {
                document.title = newTitle.textContent;
            }
            if (pushState) {
                history.pushState({}, '', url);
            }
        };

        if (SUPPORTS_VT && !REDUCED_MOTION) {
            document.startViewTransition(updateDOM);
        } else {
            updateDOM();
        }
    } catch (_err) {
        // Fetch timeout or network error — fall back to standard navigation
        window.location.href = url;
    }
}

/**
 * Click handler for navigation links.
 * @param {MouseEvent} event
 */
function handleClick(event) {
    // Skip if modifier keys held (user wants new tab/window)
    if (event.metaKey || event.ctrlKey || event.shiftKey || event.altKey) return;

    const anchor = event.target.closest('a');
    if (!isEligibleLink(anchor)) return;

    // Skip if same page
    if (anchor.href === location.href) {
        event.preventDefault();
        return;
    }

    event.preventDefault();
    navigateWithTransition(anchor.href);
}

/**
 * Handle browser back/forward navigation.
 */
function handlePopState() {
    navigateWithTransition(location.href, false);
}

// Initialize
document.addEventListener('click', handleClick);
window.addEventListener('popstate', handlePopState);
