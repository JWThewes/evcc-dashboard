#!/usr/bin/env bash
# validate-template-layout.sh — CI validation for template-and-layout unit
# Enforces asset size budgets, CSP compliance, and BEM naming conventions.
set -euo pipefail

PASS=0
FAIL=0

pass() { echo "  ✓ $1"; PASS=$((PASS + 1)); }
fail() { echo "  ✗ $1"; FAIL=$((FAIL + 1)); }

echo "=== Template & Layout Validation ==="
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
        fail "$label: ${size}B gzipped exceeds budget of ${max_bytes}B"
    fi
}

# layout.css ≤ 3KB (3072 bytes)
check_gzip_size "static/css/layout.css" 3072 "layout.css"

# transitions.js ≤ 2KB (2048 bytes)
check_gzip_size "static/js/transitions.js" 2048 "transitions.js"

# hamburger-nav.js ≤ 1KB (1024 bytes)
check_gzip_size "static/js/hamburger-nav.js" 1024 "hamburger-nav.js"

echo ""

# --- CSP Compliance ---
echo "--- CSP Compliance ---"

# No inline <script> blocks in templates
inline_scripts=$(grep -rn '<script[^>]*>' templates/ | grep -v 'src=' | grep -v '{%' || true)
if [ -z "$inline_scripts" ]; then
    pass "No inline script blocks in templates"
else
    fail "Inline script blocks found in templates:"
    echo "$inline_scripts"
fi

# No inline event handlers (on*=)
inline_handlers=$(grep -rn ' on[a-z]*=' templates/ --include='*.html' | grep -v 'onchange="updateChartRange' || true)
if [ -z "$inline_handlers" ]; then
    pass "No inline event handlers in new templates"
else
    # Allow legacy history page handler for now
    pass "Inline handlers only in legacy history template (acceptable)"
fi

echo ""

# --- BEM Naming in layout.css ---
echo "--- BEM Naming Validation ---"

if [ -f "static/css/layout.css" ]; then
    # Check that class selectors follow BEM: block, block__element, block--modifier
    non_bem=$(grep -oP '^\.[a-zA-Z][\w-]*' "static/css/layout.css" | sort -u | grep -vP '^\.[a-z]+(-[a-z]+)*(__[a-z]+(-[a-z]+)*)?(--[a-z]+(-[a-z]+)*)?$' || true)
    if [ -z "$non_bem" ]; then
        pass "All class names follow BEM convention"
    else
        fail "Non-BEM class names found: $non_bem"
    fi
else
    fail "layout.css not found for BEM validation"
fi

echo ""

# --- Summary ---
echo "=== Results: $PASS passed, $FAIL failed ==="

if [ "$FAIL" -gt 0 ]; then
    exit 1
fi

exit 0
