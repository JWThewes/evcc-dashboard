#!/usr/bin/env bash
# validate-schematic.sh — CI validation for energy-flow-schematic unit
# Enforces: CSS budget, compositor-only properties, no |safe filter, no external URLs
set -euo pipefail

ERRORS=0

echo "=== Schematic Unit Validation ==="

# 1. CSS gzip size budget: schematic.css <= 3KB
CSS_FILE="static/css/schematic.css"
if [ -f "$CSS_FILE" ]; then
    GZIP_SIZE=$(gzip -c "$CSS_FILE" | wc -c)
    MAX_SIZE=3072  # 3KB
    if [ "$GZIP_SIZE" -gt "$MAX_SIZE" ]; then
        echo "FAIL: $CSS_FILE gzipped size ${GZIP_SIZE}B exceeds ${MAX_SIZE}B budget"
        ERRORS=$((ERRORS + 1))
    else
        echo "PASS: $CSS_FILE gzipped size ${GZIP_SIZE}B <= ${MAX_SIZE}B"
    fi
else
    echo "FAIL: $CSS_FILE not found"
    ERRORS=$((ERRORS + 1))
fi

# 2. Compositor-only animation properties (only allow transform, opacity, stroke-dashoffset in animations)
if [ -f "$CSS_FILE" ]; then
    # Check for non-compositor properties in animation/keyframe contexts
    # Exclude will-change declarations (those are promotion hints, not animated props)
    BAD_PROPS=$(grep -n "animation\|@keyframes" "$CSS_FILE" -A 5 | \
        grep -E "^\s*(top|left|right|bottom|width|height|margin|padding|font-size|color|background|border)" || true)
    if [ -n "$BAD_PROPS" ]; then
        echo "FAIL: Non-compositor properties found near animation contexts in $CSS_FILE:"
        echo "$BAD_PROPS"
        ERRORS=$((ERRORS + 1))
    else
        echo "PASS: All animations use compositor-safe properties"
    fi
fi

# 3. No |safe filter in schematic template
TMPL_FILE="templates/partials/schematic.html"
if [ -f "$TMPL_FILE" ]; then
    SAFE_USAGE=$(grep -n "|safe" "$TMPL_FILE" || true)
    if [ -n "$SAFE_USAGE" ]; then
        echo "FAIL: |safe filter found in $TMPL_FILE (security risk):"
        echo "$SAFE_USAGE"
        ERRORS=$((ERRORS + 1))
    else
        echo "PASS: No |safe filter in schematic template"
    fi
else
    echo "FAIL: $TMPL_FILE not found"
    ERRORS=$((ERRORS + 1))
fi

# 4. No external URLs in schematic CSS
if [ -f "$CSS_FILE" ]; then
    EXTERNAL_URLS=$(grep -nE "url\s*\(\s*['\"]?https?://" "$CSS_FILE" || true)
    if [ -n "$EXTERNAL_URLS" ]; then
        echo "FAIL: External URL references found in $CSS_FILE:"
        echo "$EXTERNAL_URLS"
        ERRORS=$((ERRORS + 1))
    else
        echo "PASS: No external URL references in schematic CSS"
    fi
fi

# 5. No inline event handlers in template
if [ -f "$TMPL_FILE" ]; then
    INLINE_HANDLERS=$(grep -nE "on(click|load|error|mouseover|mouseout|focus|blur)=" "$TMPL_FILE" || true)
    if [ -n "$INLINE_HANDLERS" ]; then
        echo "FAIL: Inline event handlers found in $TMPL_FILE:"
        echo "$INLINE_HANDLERS"
        ERRORS=$((ERRORS + 1))
    else
        echo "PASS: No inline event handlers in schematic template"
    fi
fi

echo ""
if [ "$ERRORS" -gt 0 ]; then
    echo "FAILED: $ERRORS validation error(s)"
    exit 1
else
    echo "ALL CHECKS PASSED"
    exit 0
fi
