#!/usr/bin/env bash
set -euo pipefail

IMAGE="ghcr.io/jwthewes/evcc-dashboard:latest"

# --- Defaults (override via environment for non-interactive mode) ---
INSTALL_DIR="${INSTALL_DIR:-$HOME/evcc-dashboard}"
SERVER_PORT="${SERVER_PORT:-3000}"
BASE_PATH="${BASE_PATH:-}"
LOG_LEVEL="${LOG_LEVEL:-info}"
MQTT_HOST="${MQTT_HOST:-192.168.1.100}"
MQTT_PORT="${MQTT_PORT:-1883}"
MQTT_USER="${MQTT_USER:-}"
MQTT_PASS="${MQTT_PASS:-}"
MQTT_PREFIX="${MQTT_PREFIX:-evcc}"
MQTT_CLIENT_ID="${MQTT_CLIENT_ID:-evcc-dashboard}"
SAMPLING_INTERVAL="${SAMPLING_INTERVAL:-5}"
RAW_DAYS="${RAW_DAYS:-7}"
MINUTE_DAYS="${MINUTE_DAYS:-90}"
HOURLY_DAYS="${HOURLY_DAYS:-730}"
API_KEY="${API_KEY:-cst-$(head -c 16 /dev/urandom | xxd -p)}"

# --- Non-interactive mode: set NON_INTERACTIVE=1 or pipe to bash ---
if [ "${NON_INTERACTIVE:-0}" = "1" ] || [ ! -t 0 ]; then
  INTERACTIVE=false
else
  INTERACTIVE=true
fi

# --- Helper ---
ask() {
  local prompt="$1" default="$2" var="$3"
  if [ "$INTERACTIVE" = true ]; then
    if [ -n "$default" ]; then
      read -rp "$prompt [$default]: " input
      eval "$var=\"\${input:-$default}\""
    else
      read -rp "$prompt: " input
      eval "$var=\"\$input\""
    fi
  fi
  # In non-interactive mode, the env/default value is already set
}

echo "=== evcc-dashboard installer ==="
echo ""

if [ "$INTERACTIVE" = false ]; then
  echo "Running in non-interactive mode (using defaults / environment variables)"
  echo ""
fi

# --- Interactive prompts (skipped in non-interactive mode) ---
ask "Install directory" "$INSTALL_DIR" INSTALL_DIR
mkdir -p "$INSTALL_DIR/data"

if [ "$INTERACTIVE" = true ]; then echo ""; echo "--- Server ---"; fi
ask "Server port" "$SERVER_PORT" SERVER_PORT
ask "Base path (leave empty unless behind reverse proxy)" "$BASE_PATH" BASE_PATH
ask "Log level (debug/info/warn/error)" "$LOG_LEVEL" LOG_LEVEL

if [ "$INTERACTIVE" = true ]; then echo ""; echo "--- MQTT ---"; fi
ask "MQTT host" "$MQTT_HOST" MQTT_HOST
ask "MQTT port" "$MQTT_PORT" MQTT_PORT
ask "MQTT username (leave empty for anonymous)" "$MQTT_USER" MQTT_USER
ask "MQTT password" "$MQTT_PASS" MQTT_PASS
ask "MQTT topic prefix" "$MQTT_PREFIX" MQTT_PREFIX
ask "MQTT client ID" "$MQTT_CLIENT_ID" MQTT_CLIENT_ID

if [ "$INTERACTIVE" = true ]; then echo ""; echo "--- Sampling ---"; fi
ask "Sampling interval (seconds)" "$SAMPLING_INTERVAL" SAMPLING_INTERVAL

if [ "$INTERACTIVE" = true ]; then echo ""; echo "--- Retention ---"; fi
ask "Keep raw data (days)" "$RAW_DAYS" RAW_DAYS
ask "Keep 1-minute aggregates (days)" "$MINUTE_DAYS" MINUTE_DAYS
ask "Keep hourly aggregates (days)" "$HOURLY_DAYS" HOURLY_DAYS

if [ "$INTERACTIVE" = true ]; then echo ""; echo "--- API ---"; fi
ask "API key for mobile endpoints (auto-generated)" "$API_KEY" API_KEY

# --- Write config.toml ---
cat > "$INSTALL_DIR/config.toml" <<EOF
[server]
host = "0.0.0.0"
port = $SERVER_PORT
base_path = "$BASE_PATH"
trust_proxy_headers = true

[mqtt]
host = "$MQTT_HOST"
port = $MQTT_PORT
username = "$MQTT_USER"
password = "$MQTT_PASS"
topic_prefix = "$MQTT_PREFIX"
client_id = "$MQTT_CLIENT_ID"

[database]
path = "./data/evcc-dashboard.db"

[sampling]
interval_seconds = $SAMPLING_INTERVAL

[retention]
raw_days = $RAW_DAYS
minute_days = $MINUTE_DAYS
hourly_days = $HOURLY_DAYS

[logging]
level = "$LOG_LEVEL"

[api]
key = "$API_KEY"
EOF

# --- Write docker-compose.yml ---
cat > "$INSTALL_DIR/docker-compose.yml" <<EOF
services:
  evcc-dashboard:
    image: $IMAGE
    container_name: evcc-dashboard
    restart: unless-stopped
    ports:
      - "$SERVER_PORT:3000"
    volumes:
      - ./data:/app/data
      - ./config.toml:/app/config.toml:ro
    environment:
      - RUST_LOG=$LOG_LEVEL
EOF

echo ""
echo "=== Installation complete ==="
echo ""
echo "Files created in $INSTALL_DIR:"
echo "  config.toml        - configuration"
echo "  docker-compose.yml - container setup"
echo "  data/              - database directory"
echo ""
echo "To start:"
echo "  cd $INSTALL_DIR && docker compose up -d"
echo ""
echo "Dashboard will be available at http://localhost:$SERVER_PORT$BASE_PATH"
