#!/bin/bash
# validate-cards.sh — Validate detail-cards-and-interactions unit assets
# Checks: file existence, size budgets, syntax, data-testid coverage

set -euo pipefail

RED='\033[0;31m'
GREEN='\033[0;32m'
NC='\033[0m'
ERRORS=0

check_exists() {
  if [ ! -f "$1" ]; then
    echo -e "${RED}FAIL${NC}: $1 does not exist"
    ERRORS=$((ERRORS + 1))
  else
    echo -e "${GREEN}OK${NC}: $1 exists"
  fi
}

check_size_under() {
  local file="$1"
  local max_kb="$2"
  if [ -f "$file" ]; then
    local size
    size=$(wc -c < "$file")
    local max_bytes=$((max_kb * 1024))
    if [ "$size" -gt "$max_bytes" ]; then
      echo -e "${RED}FAIL${NC}: $file is $(( size / 1024 ))KB (budget: ${max_kb}KB)"
      ERRORS=$((ERRORS + 1))
    else
      echo -e "${GREEN}OK${NC}: $file is $(( size / 1024 ))KB (budget: ${max_kb}KB)"
    fi
  fi
}

check_testid() {
  local file="$1"
  local testid="$2"
  if [ -f "$file" ] && grep -q "data-testid=\"$testid\"" "$file"; then
    echo -e "${GREEN}OK${NC}: data-testid=\"$testid\" found in $file"
  else
    echo -e "${RED}FAIL${NC}: data-testid=\"$testid\" NOT found in $file"
    ERRORS=$((ERRORS + 1))
  fi
}

echo "=== Detail Cards & Interactions Validation ==="
echo ""

echo "--- File Existence ---"
check_exists "static/css/cards.css"
check_exists "static/css/freshness.css"
check_exists "static/css/micro-interactions.css"
check_exists "static/js/card-promotion.js"
check_exists "static/js/value-animator.js"
check_exists "static/js/freshness-monitor.js"
echo ""

echo "--- Size Budgets ---"
check_size_under "static/css/cards.css" 7
check_size_under "static/css/freshness.css" 3
check_size_under "static/css/micro-interactions.css" 3
check_size_under "static/js/card-promotion.js" 8
check_size_under "static/js/value-animator.js" 5
check_size_under "static/js/freshness-monitor.js" 5
echo ""

echo "--- JS Syntax Check ---"
if command -v node &> /dev/null; then
  for f in static/js/card-promotion.js static/js/value-animator.js static/js/freshness-monitor.js; do
    if node --check "$f" 2>/dev/null; then
      echo -e "${GREEN}OK${NC}: $f passes syntax check"
    else
      echo -e "${RED}FAIL${NC}: $f has syntax errors"
      ERRORS=$((ERRORS + 1))
    fi
  done
else
  echo "SKIP: node not available for syntax check"
fi
echo ""

echo "--- data-testid Coverage ---"
check_testid "templates/partials/cards.html" "cards-zone"
check_testid "templates/partials/cards.html" "card-energy-flow"
check_testid "templates/partials/cards.html" "card-battery"
check_testid "templates/partials/cards.html" "disconnect-banner"
check_testid "templates/partials/cards.html" "expand-energy-flow"
check_testid "templates/partials/cards.html" "expand-battery"
echo ""

echo "--- Template Integrity ---"
if grep -q "data-card-id" templates/partials/cards.html; then
  echo -e "${GREEN}OK${NC}: data-card-id attributes present"
else
  echo -e "${RED}FAIL${NC}: data-card-id attributes missing"
  ERRORS=$((ERRORS + 1))
fi

if grep -q "data-activity" templates/partials/cards.html; then
  echo -e "${GREEN}OK${NC}: data-activity attributes present"
else
  echo -e "${RED}FAIL${NC}: data-activity attributes missing"
  ERRORS=$((ERRORS + 1))
fi

if grep -q "data-animate-value" templates/partials/cards.html; then
  echo -e "${GREEN}OK${NC}: data-animate-value attributes present"
else
  echo -e "${RED}FAIL${NC}: data-animate-value attributes missing"
  ERRORS=$((ERRORS + 1))
fi
echo ""

echo "=== Results: $ERRORS errors ==="
exit $ERRORS
