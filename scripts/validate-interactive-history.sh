#!/usr/bin/env bash
# validate-interactive-history.sh — CI validation for interactive-history unit
# Enforces asset size budgets, CSP compliance, and module syntax.
set -euo pipefail

PASS=0
FAIL=0

pass() { echo "  ✓ $1"; PASS=$((PASS + 1)); }
fail() { echo "  ✗ $1"; FAIL=$((FAIL + 1)); }

echo "=== Interactive History Validation ==="
echo ""

# --- Asset Size Budgets ---
echo "--- Asset Size Budgets ---"

check_gzip_size() {
    local file="$1"
    local max_bytes="$2"
    local label="$3"

    if [ ! -f "$file" ]; then
        fail "$label: file not found ($file)"
        return
    fi

    local size
    size=$(gzip -c "$file" | wc -c)

    if [ "$size" -le "$max_bytes" ]; then
        pass "$label: ${size}B gzipped (budget: ${max_bytes}B)"
    else
        fail "$label: ${size}B gzipped EXCEEDS budget of ${max_bytes}B"
    fi
}

check_raw_size() {
    local file="$1"
    local max_bytes="$2"
    local label="$3"

    if [ ! -f "$file" ]; then
        fail "$label: file not found ($file)"
        return
    fi

    local size
    size=$(wc -c < "$file")

    if [ "$size" -le "$max_bytes" ]; then
        pass "$label: ${size}B raw (budget: ${max_bytes}B)"
    else
        fail "$label: ${size}B raw EXCEEDS budget of ${max_bytes}B"
    fi
}

# JS gzip budgets (from performance-design-interactive-history — adjusted for unminified source)
check_gzip_size "static/js/calendar.js" 4096 "calendar.js gzip ≤4KB"
check_gzip_size "static/js/history-charts.js" 5632 "history-charts.js gzip ≤5.5KB"
check_gzip_size "static/js/history-comparison.js" 2560 "history-comparison.js gzip ≤2.5KB"

# Total JS gzip budget: 12KB (unminified source; production minified target is 9KB)
total_js_gzip=0
for f in static/js/calendar.js static/js/history-charts.js static/js/history-comparison.js; do
    if [ -f "$f" ]; then
        total_js_gzip=$((total_js_gzip + $(gzip -c "$f" | wc -c)))
    fi
done
if [ "$total_js_gzip" -le 12288 ]; then
    pass "Total JS gzip: ${total_js_gzip}B (budget: 12288B)"
else
    fail "Total JS gzip: ${total_js_gzip}B EXCEEDS budget of 12288B"
fi

# CSS raw size budgets (unminified source)
check_raw_size "static/css/history-calendar.css" 6144 "history-calendar.css raw ≤6KB"
check_raw_size "static/css/history-charts.css" 6144 "history-charts.css raw ≤6KB"

echo ""

# --- Security / CSP Compliance ---
echo "--- CSP Compliance ---"

JS_FILES="static/js/calendar.js static/js/history-charts.js static/js/history-comparison.js"

# No inline event handlers (onclick, onload, etc.)
for f in $JS_FILES; do
    if [ -f "$f" ]; then
        if grep -qiE '(onclick|onload|onerror|onchange|onsubmit|onfocus|onblur)=' "$f"; then
            fail "$f: contains inline event handler attributes"
        else
            pass "$f: no inline event handlers"
        fi
    fi
done

# No eval or Function constructor
for f in $JS_FILES; do
    if [ -f "$f" ]; then
        if grep -qE '\beval\s*\(|new\s+Function\s*\(' "$f"; then
            fail "$f: contains eval() or new Function()"
        else
            pass "$f: no eval/Function"
        fi
    fi
done

# No localStorage/sessionStorage/indexedDB
for f in $JS_FILES; do
    if [ -f "$f" ]; then
        if grep -qiE '(localStorage|sessionStorage|indexedDB|document\.cookie)' "$f"; then
            fail "$f: contains storage API usage"
        else
            pass "$f: no storage APIs"
        fi
    fi
done

# No external URLs or CDN references
for f in $JS_FILES; do
    if [ -f "$f" ]; then
        if grep -qE 'https?://' "$f"; then
            fail "$f: contains external URL reference"
        else
            pass "$f: no external URLs"
        fi
    fi
done

echo ""

# --- Module Syntax Check ---
echo "--- Module Syntax ---"

# Check that JS files use export (are proper ES modules)
for f in static/js/calendar.js static/js/history-charts.js static/js/history-comparison.js; do
    if [ -f "$f" ]; then
        if grep -q '^export ' "$f"; then
            pass "$f: has ES module exports"
        else
            fail "$f: missing ES module exports"
        fi
    fi
done

echo ""

# --- CSS Validation ---
echo "--- CSS Validation ---"

CSS_FILES="static/css/history-calendar.css static/css/history-charts.css"

# No non-compositor animations (check for animating width, height, margin, padding, left, top, etc.)
for f in $CSS_FILES; do
    if [ -f "$f" ]; then
        # Allow transform, opacity in @keyframes; flag anything else
        if grep -E '@keyframes' "$f" | grep -qvE '(transform|opacity|rotate)'; then
            # Deeper check: look inside keyframes for non-compositor properties
            if grep -A5 '@keyframes' "$f" | grep -qiE '(width|height|margin|padding|left|top|right|bottom|font-size)'; then
                fail "$f: keyframe animates non-compositor property"
            else
                pass "$f: compositor-only keyframes"
            fi
        else
            pass "$f: compositor-only animations"
        fi
    fi
done

# BEM naming check (classes should start with history- or be known utility classes)
for f in $CSS_FILES; do
    if [ -f "$f" ]; then
        non_bem=$(grep -oE '\.[a-zA-Z][a-zA-Z0-9_-]*\s*[{,:]' "$f" | grep -oE '\.[a-zA-Z][a-zA-Z0-9_-]*' | sort -u | grep -v '\.history-' | grep -v '\.grid-' | grep -v '\.spinner' | grep -v '\.visually-hidden' | head -5 || true)
        if [ -z "$non_bem" ]; then
            pass "$f: follows BEM naming (history-* prefix)"
        else
            fail "$f: non-BEM class names found: $non_bem"
        fi
    fi
done

echo ""
echo "=== Results ==="
echo "  Passed: $PASS"
echo "  Failed: $FAIL"
echo ""

if [ "$FAIL" -gt 0 ]; then
    echo "VALIDATION FAILED"
    exit 1
fi

echo "ALL CHECKS PASSED"
exit 0
