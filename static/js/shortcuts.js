// Keyboard shortcuts handler
(function() {
    let pendingKey = null;
    let pendingTimer = null;

    function isInputFocused() {
        var el = document.activeElement;
        if (!el) return false;
        var tag = el.tagName.toLowerCase();
        return tag === 'input' || tag === 'textarea' || tag === 'select';
    }

    function getBasePath() {
        var meta = document.querySelector('meta[name="base-path"]');
        return meta ? meta.getAttribute('content') : '';
    }

    function forceRefresh() {
        document.querySelectorAll('[hx-trigger]').forEach(function(el) {
            if (typeof htmx !== 'undefined') {
                htmx.trigger(el, 'htmx:trigger');
            }
        });
    }

    document.addEventListener('keydown', function(e) {
        if (isInputFocused()) return;

        var key = e.key.toLowerCase();

        if (pendingKey === 'g') {
            clearTimeout(pendingTimer);
            pendingKey = null;
            var base = getBasePath();
            if (key === 'd') { window.location.href = base + '/'; }
            else if (key === 'h') { window.location.href = base + '/history'; }
            else if (key === 's') { window.location.href = base + '/settings'; }
            return;
        }

        if (key === 'g') {
            pendingKey = 'g';
            pendingTimer = setTimeout(function() { pendingKey = null; }, 500);
            return;
        }

        if (key === 'r') {
            forceRefresh();
        }
    });
})();
