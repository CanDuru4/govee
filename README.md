# Govee to MQTT bridge for Home Assistant

[![Build](https://github.com/CanDuru4/govee/actions/workflows/build.yml/badge.svg)](https://github.com/CanDuru4/govee/actions/workflows/build.yml)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE.md)
[![Rust 2021](https://img.shields.io/badge/rust-2021%20edition-orange.svg)](Cargo.toml)
[![Container](https://img.shields.io/badge/image-ghcr.io%2Fcanduru4%2Fgovee-2496ED?logo=docker&logoColor=white)](https://github.com/CanDuru4/govee/pkgs/container/govee)
[![Home Assistant add-on](https://img.shields.io/badge/Home%20Assistant-add--on-41BDF5?logo=homeassistant&logoColor=white)](docs/ADDON.md)

This repo provides a `govee` executable whose primary purpose is to act
as a bridge between [Govee](https://govee.com) devices and Home Assistant,
via the [Home Assistant MQTT Integration](https://www.home-assistant.io/integrations/mqtt/).

It is for anyone running Home Assistant who wants their Govee lights,
humidifiers, air purifiers and other devices to appear as native entities
without going through the Govee cloud app. The bridge talks to devices over
the Govee LAN API where possible, falls back to Govee's undocumented AWS IoT
service for low-latency status, and finally to the public Platform API. It
ships as a multi-arch container image and as a Home Assistant add-on.

## About this fork

This project is derived from and heavily based on the upstream project
[wez/govee2mqtt](https://github.com/wez/govee2mqtt). This fork adds support and
enhancements specifically for the Govee H7126 air purifier.

The upstream project by Wez is excellent and remains the canonical source for
broader device support and documentation. Please consider starring and supporting
the upstream project if you find this useful.

## Features

* Robust LAN-first design. Not all of Govee's devices support LAN control,
  but for those that do, you'll have the lowest latency and ability to
  control them even when your primary internet connection is offline.
* Support for per-device modes and scenes.
* Support for the undocumented AWS IoT interface to your devices, provides
  low latency status updates.
* Support for the new [Platform
  API](https://developer.govee.com/reference/get-you-devices) in case the AWS
  IoT or LAN control is unavailable.

|Feature|Requires|Notes|
|-------|--------|-------------|
|DIY Scenes|API Key|Find in the list of Effects for the light in Home Assistant|
|Music Modes|API Key|Find in the list of Effects for the light in Home Assistant|
|Tap-to-Run / One Click Scene|IoT|Find in the overall list of Scenes in Home Assistant, as well as under the `Govee to MQTT` device|
|Live Device Status Updates|LAN and/or IoT|Devices typically report most changes within a couple of seconds.|
|Segment Color|API Key|Find the `Segment 00X` light entities associated with your main light device in Home Assistant|

* `API Key` means that you have [applied for a key from Govee](https://developer.govee.com/reference/apply-you-govee-api-key)
  and have configured it for use in govee2mqtt
* `IoT` means that you have configured your Govee account email and password for
  use in govee2mqtt, which will then attempt to use the
  *undocumented and likely unsupported* AWS MQTT-based IoT service
* `LAN` means that you have enabled the [Govee LAN API](https://app-h5.govee.com/user-manual/wlan-guide)
  on supported devices and that the LAN API protocol is functional on your network

### H7126 Air Purifier additions in this fork

This fork focuses on adding support for the Govee H7126 air purifier. It
enables Home Assistant discovery and control via MQTT for the H7126 model.
Additional improvements may be included over time.

Other fork-specific changes:

* MQTT client id is `govee2mqtt-<uuid>` (upstream uses `govee2mqtt/<uuid>`).
  Mosquitto >= 2.1 (Home Assistant Mosquitto add-on 7.x) rejects client ids
  containing `/` as "dangerous" and silently denies every publish/subscribe
  after a successful CONNACK, which makes all entities unavailable. See
  [wez/govee2mqtt#659](https://github.com/wez/govee2mqtt/issues/659).

* The `H600B` Smart LED Bulb has a quirk (`Quirk::light("H600B", BULB)`,
  2700-6500 K). Upstream has none for this SKU, so the bulb fell back to the
  Platform API for control and to a 5-second-delayed poll for state; that poll
  returns the *previous* state, which made Home Assistant flicker between the
  old and the new brightness after every change. With the quirk the bulb uses
  the AWS IoT path: instant control and push state updates.

## Tech stack

|Layer|Choice|
|-----|------|
|Language|Rust (2021 edition), async on [tokio](https://tokio.rs)|
|CLI|`clap` (derive), `dotenvy` for `.env` loading|
|MQTT|`mosquitto-rs` (vendored OpenSSL)|
|HTTP client / server|`reqwest` for Govee APIs, `axum` + `tower-http` for the built-in web UI|
|Local cache|`sqlite-cache` on bundled `rusqlite`|
|Device transports|Govee LAN API (UDP), AWS IoT over MQTT, Govee Platform REST API|
|Packaging|Distroless container image, Home Assistant add-on (amd64, aarch64, armv7)|

## Getting started

Most people should use one of the packaged paths rather than building from
source:

* [Installing the Home Assistant add-on](docs/ADDON.md) - for HAOS and
  Supervised Home Assistant users
* [Running it in Docker](docs/DOCKER.md) - `ghcr.io/canduru4/govee:latest`,
  see also `docker-compose.yml` in the repo root
* [Full configuration reference](docs/CONFIG.md)

### Prerequisites

* An MQTT broker already configured in Home Assistant
  ([instructions](https://www.home-assistant.io/integrations/mqtt/#configuration))
* Host networking, because Govee LAN discovery uses multicast UDP
* Optionally a [Govee API key](https://developer.govee.com/reference/apply-you-govee-api-key)
  for scenes, music modes and segment colour
* To build from source: a stable Rust toolchain and the system dependencies
  needed by `mosquitto-rs`

### Build and run from source

```bash
git clone https://github.com/CanDuru4/govee.git
cd govee

cargo build --release
cargo test --all

# Makefile shortcuts: `make check` (cargo check), `make test` (cargo nextest
# run), `make fmt` (nightly rustfmt), `make docker`, `make addon`

# List the devices your account can see
./target/release/govee list

# Run the bridge; the web UI is then on http://localhost:8056/assets/index.html
./target/release/govee serve
```

Other subcommands: `lan-disco`, `lan-control`, `list-http`, `http-control`,
`undoc`. Run `govee --help` for the full set.

### Environment variables

Configuration is read from flags, from the environment, or from a `.env` file
in the working directory. Names only below - never commit real values; `.env`
is git-ignored.

|Variable|Purpose|
|--------|-------|
|`GOVEE_EMAIL`|Govee account email, enables the AWS IoT path and room names|
|`GOVEE_PASSWORD`|Govee account password|
|`GOVEE_API_KEY`|Govee Platform API key|
|`GOVEE_MQTT_HOST`|MQTT broker host|
|`GOVEE_MQTT_PORT`|MQTT broker port (default `1883`)|
|`GOVEE_MQTT_USER`|MQTT username, if the broker requires auth|
|`GOVEE_MQTT_PASSWORD`|MQTT password, if the broker requires auth|
|`GOVEE_TEMPERATURE_SCALE`|`C` or `F`|
|`GOVEE_LAN_NO_MULTICAST`|Disable multicast discovery|
|`GOVEE_LAN_BROADCAST_ALL`|Broadcast discovery on every non-loopback interface|
|`GOVEE_LAN_BROADCAST_GLOBAL`|Broadcast discovery to `255.255.255.255`|
|`GOVEE_LAN_SCAN`|Comma-separated addresses to probe directly|
|`GOVEE_LAN_DISCO_TIMEOUT`|LAN discovery timeout|
|`GOVEE_CACHE_DIR`|Override the on-disk cache location|
|`RUST_LOG`|Log filter, for example `govee=trace`|

`GOVEE_LOG_SENSITIVE_DATA` exists for debugging only. It causes credentials and
tokens to be written to the log; leave it unset.

## Project structure

```
src/
  main.rs             CLI entry point and argument wiring
  commands/           one module per subcommand (serve, list, lan-*, http-*, undoc)
  service/            coordinator, device model, HTTP/web UI, AWS IoT client,
                      shared state, and quirks.rs (per-SKU overrides)
  hass_mqtt/          Home Assistant MQTT discovery per entity type
                      (light, air_purifier, humidifier, climate, sensor, ...)
  lan_api.rs          Govee LAN protocol
  platform_api.rs     documented Govee Platform API
  undoc_api.rs        undocumented Govee app API
  rest_api.rs         legacy Govee REST API
addon/                Home Assistant add-on (config.yaml, build.yaml, Dockerfile, run.sh)
assets/               static files for the built-in web UI
docs/                 ADDON, DOCKER, CONFIG, LAN, SKUS, FAQ, PRIVACY, H7126_SUPPORT
scripts/              cross-compilation, docker build and release tagging helpers
test-data/            recorded API payloads used by the snapshot tests
```

## Continuous integration & supply-chain security

Three GitHub Actions workflows live in `.github/workflows/`:

|Workflow|Trigger|What it does|
|--------|-------|------------|
|`pr.yml`|Pull requests to `main`|`cargo build --all`, `cargo test --all`, `cargo fmt --check`|
|`build.yml`|Push to `main`, `20*` tags, pull requests|Cross-compiles for `linux/amd64`, `linux/arm/v7` and `linux/arm64`, pushes per-arch digests to `ghcr.io/canduru4/govee`, merges them into a multi-arch manifest, and on a tag builds the Home Assistant add-on images|
|`no-response.yml`|Daily cron + issue comments|Closes issues left waiting on the reporter|

Hardening applied to all three:

* **Every third-party action is pinned to a full commit SHA**, with the
  human-readable tag kept in a trailing comment (for example
  `actions/checkout@11d5960... # v4.4.0`). A mutable tag such as `@v4` can be
  repointed by the action's owner at any time; a SHA cannot.
* **`permissions: contents: read` is declared at the top level** of each
  workflow, so every job starts read-only. Jobs re-declare only the extra
  scopes they genuinely need: `packages: write` for the jobs that push images
  to GHCR, `id-token: write` for the add-on build, and `issues: write` for the
  no-response bot.
* **Dependabot** (`.github/dependabot.yml`) watches four manifests weekly and
  groups each into one pull request: Cargo crates, the pinned GitHub Actions,
  the root `Dockerfile` base images, and the add-on `Dockerfile` base images.
  Because Dependabot understands the `<sha> # <tag>` form, pinning to a SHA
  does not leave the actions stranded on a stale release.

## Have a question?

* [Is my device supported?](docs/SKUS.md)
* [Check out the FAQ](docs/FAQ.md)

## Credits

* [Wez Furlong](https://github.com/wez) wrote
  [govee2mqtt](https://github.com/wez/govee2mqtt), the upstream project this
  fork is built on, which in turn grew out of his earlier
  [Govee LAN Control](https://github.com/wez/govee-lan-hass/) work.
* AWS IoT support was made possible by the work of @bwp91 in
  [homebridge-govee](https://github.com/bwp91/homebridge-govee/).

## Attribution & License

This project is based on and includes substantial portions of
[wez/govee2mqtt](https://github.com/wez/govee2mqtt), which is licensed under the
MIT License. This fork keeps the same MIT License. See `LICENSE.md` for the
full text. All original copyrights remain with their respective owners; any
modifications in this fork are provided under the same MIT terms.

## Author

Can Duru - [canduru.net](https://canduru.net)
