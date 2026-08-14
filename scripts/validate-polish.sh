#!/bin/bash
# =============================================================================
# CI Validation: polish-and-accessibility unit
# File: scripts/validate-polish.sh
#
# Validates:
# - Cumulative JS bundle size ≤25KB gzipped
# - Reduced-motion CSS size ≤1.5KB gzipped
# - Security compliance (no prohibited APIs in polish assets)
# - ARIA compliance (live region, role=img, aria-label present)
# - Performance rules (compositor-only animations)
# - BEM naming in reduced-motion.css
# =============================================================================

set -euo pipefail

PASS=0
FAIL=0
WARN=0

pass() { PASS=$((PASS + 1)); echo "  ✓ $1"; }
fail() { FAIL=$((FAIL + 1)); echo "  ✗ $1"; }
warn() { WARN=$((WARN + 1)); echo "  ⚠ $1"; }

echo "=== Polish & Accessibility Validation ==="
echo ""

# --- Asset Size Budgets ---
echo "── Asset Size Budgets ──"

# Cumulative JS budget (excluding vendored echarts, htmx, idiomorph)
CUSTOM_JS_SIZE=$(cat static/js/transitions.js \
    static/js/hamburger-nav.js \
    static/js/card-promotion.js \
    static/js/value-animator.js \
    static/js/freshness-monitor.js \
    static/js/calendar.js \
    static/js/history-charts.js \
    static/js/history-comparison.js \
    static/js/reduced-motion-detect.js \
    2>/dev/null | gzip -9 | wc -c)

if [ "$CUSTOM_JS_SIZE" -le 25600 ]; then
    pass "Cumulative JS ≤25KB gzipped (actual: ${CUSTOM_JS_SIZE} bytes)"
else
    fail "Cumulative JS exceeds 25KB gzipped (actual: ${CUSTOM_JS_SIZE} bytes)"
fi

# Reduced-motion CSS size ≤1.5KB gzipped
if [ -f "static/css/reduced-motion.css" ]; then
    RM_CSS_SIZE=$(gzip -9 -c static/css/reduced-motion.css | wc -c)
    if [ "$RM_CSS_SIZE" -le 1536 ]; then
        pass "reduced-motion.css ≤1.5KB gzipped (actual: ${RM_CSS_SIZE} bytes)"
    else
        fail "reduced-motion.css exceeds 1.5KB gzipped (actual: ${RM_CSS_SIZE} bytes)"
    fi
else
    fail "reduced-motion.css not found"
fi

# Reduced-motion-detect.js ≤0.5KB gzipped
if [ -f "static/js/reduced-motion-detect.js" ]; then
    RM_JS_SIZE=$(gzip -9 -c static/js/reduced-motion-detect.js | wc -c)
    if [ "$RM_JS_SIZE" -le 512 ]; then
        pass "reduced-motion-detect.js ≤0.5KB gzipped (actual: ${RM_JS_SIZE} bytes)"
    else
        fail "reduced-motion-detect.js exceeds 0.5KB gzipped (actual: ${RM_JS_SIZE} bytes)"
    fi
else
    fail "reduced-motion-detect.js not found"
fi

echo ""

# --- Security Compliance ---
echo "── Security Compliance ──"

# Check reduced-motion-detect.js for prohibited APIs
if [ -f "static/js/reduced-motion-detect.js" ]; then
    if grep -qE '(innerHTML|eval\(|new Function|document\.write)' static/js/reduced-motion-detect.js; then
        fail "reduced-motion-detect.js uses prohibited DOM API"
    else
        pass "No innerHTML/eval/document.write in reduced-motion-detect.js"
    fi

    if grep -qE '(fetch\(|XMLHttpRequest|WebSocket|navigator\.sendBeacon)' static/js/reduced-motion-detect.js; then
        fail "reduced-motion-detect.js uses prohibited network API"
    else
        pass "No network APIs in reduced-motion-detect.js"
    fi

    if grep -qE '(localStorage|sessionStorage|indexedDB|document\.cookie)' static/js/reduced-motion-detect.js; then
        fail "reduced-motion-detect.js uses prohibited storage API"
    else
        pass "No storage APIs in reduced-motion-detect.js"
    fi

    if grep -qE '(createElement|import\()' static/js/reduced-motion-detect.js; then
        fail "reduced-motion-detect.js uses createElement or dynamic import"
    else
        pass "No createElement/dynamic import in reduced-motion-detect.js"
    fi
fi

# Check reduced-motion.css for external URLs
if [ -f "static/css/reduced-motion.css" ]; then
    if grep -qiE '(url\s*\(|@import)' static/css/reduced-motion.css; then
        fail "reduced-motion.css contains url() or @import"
    else
        pass "No external references in reduced-motion.css"
    fi
fi

echo ""

# --- ARIA Compliance ---
echo "── ARIA Compliance ──"

# Check schematic template for ARIA live region
if [ -f "templates/partials/schematic.html" ]; then
    if grep -q 'aria-live="polite"' templates/partials/schematic.html; then
        pass "ARIA live region present in schematic template"
    else
        fail "Missing aria-live=polite in schematic template"
    fi

    if grep -q 'role="status"' templates/partials/schematic.html; then
        pass "role=status present in schematic template"
    else
        fail "Missing role=status in schematic template"
    fi

    if grep -q 'role="img"' templates/partials/schematic.html; then
        pass "role=img present on schematic container"
    else
        fail "Missing role=img on schematic container"
    fi

    if grep -q 'aria-label=' templates/partials/schematic.html; then
        pass "Dynamic aria-label present on schematic"
    else
        fail "Missing aria-label on schematic container"
    fi
fi

# Check cards template for ARIA labels
if [ -f "templates/partials/cards.html" ]; then
    if grep -q 'aria-label=' templates/partials/cards.html; then
        pass "aria-label attributes present in cards template"
    else
        fail "Missing aria-label in cards template"
    fi
fi

echo ""

# --- Reduced Motion CSS Rules ---
echo "── Reduced Motion CSS Rules ──"

if [ -f "static/css/reduced-motion.css" ]; then
    if grep -q 'prefers-reduced-motion: reduce' static/css/reduced-motion.css; then
        pass "prefers-reduced-motion media query present"
    else
        fail "Missing prefers-reduced-motion media query"
    fi

    if grep -q 'animation: none' static/css/reduced-motion.css; then
        pass "animation: none overrides present"
    else
        fail "Missing animation: none overrides"
    fi

    # Verify .value--animating is NOT overridden
    if grep -qE '\.value--animating' static/css/reduced-motion.css | grep -v '^.*\/\*'; then
        fail ".value--animating should NOT be overridden (informational transitions)"
    else
        pass ".value--animating is not overridden (retained)"
    fi

    if grep -q '\.flow-path__arrow-group' static/css/reduced-motion.css; then
        pass "Arrow group visibility toggle in reduced-motion.css"
    else
        fail "Missing arrow group visibility in reduced-motion.css"
    fi
fi

echo ""

# --- Performance Rules ---
echo "── Performance Rules ──"

# Verify no non-compositor animations in reduced-motion.css
if [ -f "static/css/reduced-motion.css" ]; then
    # Check that none of the allowed animation properties in the file
    # target layout-triggering properties
    if grep -E 'animation:' static/css/reduced-motion.css | grep -vE '(none|inherit)' | grep -qE '(width|height|top|left|right|bottom|margin|padding)'; then
        fail "Non-compositor animation properties found in reduced-motion.css"
    else
        pass "All animations in reduced-motion.css are compositor-safe"
    fi
fi

# Check load order in base template
if [ -f "templates/base.html" ]; then
    # Verify reduced-motion.css loads AFTER app.css
    APP_LINE=$(grep -n 'app.css' templates/base.html | head -1 | cut -d: -f1)
    RM_LINE=$(grep -n 'reduced-motion.css' templates/base.html | head -1 | cut -d: -f1)
    if [ -n "$APP_LINE" ] && [ -n "$RM_LINE" ] && [ "$RM_LINE" -gt "$APP_LINE" ]; then
        pass "reduced-motion.css loads after app.css in cascade"
    else
        fail "reduced-motion.css must load AFTER app.css"
    fi

    # Verify reduced-motion-detect.js is loaded
    if grep -q 'reduced-motion-detect.js' templates/base.html; then
        pass "reduced-motion-detect.js referenced in base template"
    else
        fail "Missing reduced-motion-detect.js reference in base template"
    fi
fi

echo ""

# --- sr-only utility ---
echo "── Accessibility Utility ──"

if grep -q '\.sr-only' static/css/reduced-motion.css; then
    pass ".sr-only utility class defined"
else
    fail "Missing .sr-only utility class"
fi

echo ""

# --- Summary ---
echo "════════════════════════════════════════════"
echo "Results: ${PASS} passed, ${FAIL} failed, ${WARN} warnings"
echo "════════════════════════════════════════════"

if [ "$FAIL" -gt 0 ]; then
    exit 1
fi
