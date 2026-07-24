#!/usr/bin/env bash
# validate-foundation.sh — CI-time validation for foundation static assets.
# Enforces budgets from performance-design and security-design artifacts.
# Exit 0 = all checks pass; non-zero = one or more violations.

set -euo pipefail

ERRORS=0
ROOT="$(cd "$(dirname "$0")/.." && pwd)"

echo "=== Foundation Asset Validation ==="
echo ""

# --- Asset size budgets (gzipped) ---
check_gzip_size() {
  local file="$1"
  local max_bytes="$2"
  local label="$3"

  if [ ! -f "$ROOT/$file" ]; then
    echo "FAIL: $file does not exist"
    ERRORS=$((ERRORS + 1))
    return
  fi

  local size
  size=$(gzip -c "$ROOT/$file" | wc -c | tr -d ' ')

  if [ "$size" -gt "$max_bytes" ]; then
    echo "FAIL: $label — ${size} bytes gzipped (budget: ${max_bytes})"
    ERRORS=$((ERRORS + 1))
  else
    echo "PASS: $label — ${size} bytes gzipped (budget: ${max_bytes})"
  fi
}

echo "--- Size Budgets ---"
check_gzip_size "static/css/tokens.css" 2048 "tokens.css <= 2KB"
check_gzip_size "static/css/animations.css" 2048 "animations.css <= 2KB"
check_gzip_size "static/js/idiomorph.min.js" 5632 "idiomorph.min.js <= 5.5KB"
echo ""

# --- Compositor-only property enforcement in animations.css ---
echo "--- Compositor-Only Properties ---"
# Allowed properties inside @keyframes: transform, opacity, stroke-dashoffset,
# stroke-dasharray (not animated, just set as context)
# Forbidden: width, height, top, left, right, bottom, margin, padding, color,
# background, border, font-size, line-height

FORBIDDEN_PROPS="width|height|\\btop\\b|\\bleft\\b|\\bright\\b|\\bbottom\\b|margin|padding|\\bcolor\\b|background|border-width|font-size|line-height"

# Extract content inside @keyframes blocks and check for forbidden properties
if grep -P "^\s*(${FORBIDDEN_PROPS})\s*:" "$ROOT/static/css/animations.css" 2>/dev/null; then
  echo "FAIL: animations.css contains non-compositor properties"
  ERRORS=$((ERRORS + 1))
else
  echo "PASS: animations.css uses only compositor-safe properties"
fi
echo ""

# --- SRI hash verification ---
echo "--- SRI Integrity ---"
if [ ! -f "$ROOT/.sri-hashes" ]; then
  echo "FAIL: .sri-hashes file not found"
  ERRORS=$((ERRORS + 1))
else
  while IFS=' ' read -r expected_hash file_path; do
    # Skip comments and empty lines
    [[ "$expected_hash" =~ ^#.*$ ]] && continue
    [[ -z "$expected_hash" ]] && continue

    if [ ! -f "$ROOT/$file_path" ]; then
      echo "FAIL: $file_path referenced in .sri-hashes does not exist"
      ERRORS=$((ERRORS + 1))
      continue
    fi

    # Extract algorithm and expected digest
    local_algo="${expected_hash%%-*}"
    local_digest="${expected_hash#*-}"

    # Compute actual hash
    actual_digest=$(openssl dgst -"$local_algo" -binary "$ROOT/$file_path" | openssl base64 -A)

    if [ "$local_digest" = "$actual_digest" ]; then
      echo "PASS: $file_path SRI hash matches ($local_algo)"
    else
      echo "FAIL: $file_path SRI hash mismatch"
      echo "  Expected: $local_digest"
      echo "  Actual:   $actual_digest"
      ERRORS=$((ERRORS + 1))
    fi
  done < "$ROOT/.sri-hashes"
fi
echo ""

# --- No external URL references in foundation assets ---
echo "--- No External References ---"
FOUNDATION_FILES=(
  "static/css/tokens.css"
  "static/css/animations.css"
  "static/js/idiomorph.min.js"
)

for f in "${FOUNDATION_FILES[@]}"; do
  if [ ! -f "$ROOT/$f" ]; then
    continue
  fi
  if grep -qiP 'https?://' "$ROOT/$f" 2>/dev/null; then
    echo "FAIL: $f contains external URL reference"
    ERRORS=$((ERRORS + 1))
  else
    echo "PASS: $f has no external URL references"
  fi
done
echo ""

# --- Token count ceiling (max 80 :root custom properties) ---
echo "--- Token Count ---"
if [ -f "$ROOT/static/css/tokens.css" ]; then
  TOKEN_COUNT=$(grep -cP '^\s*--' "$ROOT/static/css/tokens.css" || true)
  if [ "$TOKEN_COUNT" -gt 80 ]; then
    echo "FAIL: tokens.css has $TOKEN_COUNT custom properties (max: 80)"
    ERRORS=$((ERRORS + 1))
  else
    echo "PASS: tokens.css has $TOKEN_COUNT custom properties (max: 80)"
  fi
fi

# --- Keyframe count ceiling (max 20) ---
KEYFRAME_COUNT=$(grep -c '@keyframes' "$ROOT/static/css/animations.css" || true)
if [ "$KEYFRAME_COUNT" -gt 20 ]; then
  echo "FAIL: animations.css has $KEYFRAME_COUNT keyframes (max: 20)"
  ERRORS=$((ERRORS + 1))
else
  echo "PASS: animations.css has $KEYFRAME_COUNT keyframes (max: 20)"
fi
echo ""

# --- Summary ---
echo "=== Summary ==="
if [ "$ERRORS" -eq 0 ]; then
  echo "All foundation validation checks passed."
  exit 0
else
  echo "$ERRORS validation check(s) FAILED."
  exit 1
fi
