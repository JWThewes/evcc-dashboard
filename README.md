# evcc-dashboard

A lightweight dashboard for [evcc](https://evcc.io/) that collects MQTT data and stores it in a local SQLite database with configurable retention and aggregation.

## Quick Install

```bash
curl -fsSL https://raw.githubusercontent.com/JWThewes/evcc-dashboard/master/install.sh | bash
```

This will interactively guide you through configuration and set up a Docker Compose deployment. A random password is auto-generated during installation.

### Non-interactive install

Override defaults via environment variables:

```bash
curl -fsSL https://raw.githubusercontent.com/JWThewes/evcc-dashboard/master/install.sh \
  | NON_INTERACTIVE=1 MQTT_HOST=192.168.1.50 AUTH_PASSWORD=my-secret bash
```

### Authentication

The dashboard and API are protected by a single password (configured in `[auth]` in `config.toml`):

- **Dashboard** (browser): cookie-based session after login at `/login`
- **API** (mobile/programmatic): `Authorization: Bearer <password>` header

### Configuration

The installer creates a `config.toml` and `docker-compose.yml` in the install directory (`~/evcc-dashboard` by default). See [`config.example.toml`](config.example.toml) for all available options.

### Start / Stop

```bash
cd ~/evcc-dashboard
docker compose up -d    # start
docker compose down      # stop
docker compose pull      # update to latest image
```
