# AGENTS.md

Govee-to-MQTT bridge for Home Assistant: a single Rust binary (`govee`) that exposes Govee
lights, air purifiers, humidifiers etc. as HA entities via MQTT discovery. Fork of
[wez/govee2mqtt](https://github.com/wez/govee2mqtt) adding H7126 air-purifier support, a
Mosquitto-safe client id, and an H600B bulb quirk. Active, in daily use by the owner as a
Home Assistant add-on. Rust 2021, tokio, axum, mosquitto-rs, reqwest, rusqlite.

## Repo map
- `src/main.rs` - clap CLI; subcommands `serve`, `list`, `lan-disco`, `lan-control`,
  `list-http`, `http-control`, `undoc` (one module each in `src/commands/`).
- `src/service/` - runtime: `coordinator.rs`, `device.rs`, `state.rs`, `iot.rs` (AWS IoT),
  `http.rs` (web UI on :8056), `hass.rs`, and `quirks.rs` (per-SKU overrides table).
- `src/hass_mqtt/` - one file per HA entity type (light, air_purifier, humidifier, work_mode, ...).
- `src/lan_api.rs`, `src/platform_api.rs`, `src/undoc_api.rs`, `src/rest_api.rs` - the four
  Govee transports (LAN UDP, public Platform API, undocumented app API, legacy REST).
- `test-data/*.json` - recorded API payloads; `src/__k9_snapshots__/` - k9 snapshot files.
- `assets/` - static web UI (served from `/assets/index.html`).
- `addon/` - HA add-on (`config.yaml`, `build.yaml`, `Dockerfile`, `run.sh`, `CHANGELOG.md`).
- `repository.yaml` - HA add-on repository manifest. `Dockerfile` - distroless runtime image.
- `scripts/` - `build-cross.sh`, `build-docker.sh`, `apply-tag.sh`, `tag-release.sh`, `cliff.toml`.
- `docs/` - user docs: CONFIG, ADDON, DOCKER, LAN, SKUS, FAQ, PRIVACY, H7126_SUPPORT.

## Commands
- Build: `cargo build --release` (CI: `cargo build --all`)
- Test: `cargo test --all` (CI adds `-- --show-output`); `make test` uses `cargo nextest run`
- Format check: `cargo fmt --all -- --check` (CI gate); `make fmt` = `cargo +nightly fmt`
  (`.rustfmt.toml` uses the nightly-only `imports_granularity = "Module"`)
- Check: `make check`
- Run locally: `./target/release/govee list`, `./target/release/govee serve` (config from flags,
  env, or a git-ignored `.env`; see `docs/CONFIG.md`)
- Cross-build one arch: `./scripts/build-cross.sh linux/arm64` (needs `cross`; output in `docker-target/`)
- Test-build the add-on locally: `make addon` (privileged HA builder container)

## CI / release
- `.github/workflows/pr.yml` - build, test, fmt check on PRs to `main`.
- `.github/workflows/build.yml` - cross-compiles amd64/armv7/arm64; pushes to `main` publish
  `ghcr.io/canduru4/govee:latest`; `20*` tags also run the `addon` job, which builds
  `ghcr.io/canduru4/govee-{arch}` images (amd64 and aarch64 only) with the HA builder action.
- Release: the add-on version is a date-hash tag (`YYYY.MM.DD-<8-char sha>`). `scripts/tag-release.sh`
  creates the git tag, runs `apply-tag.sh` (rewrites `version:` in `addon/config.yaml`) and
  regenerates `addon/CHANGELOG.md` via git-cliff in Docker, then commits. Recent history instead
  bumps `addon/config.yaml` in a separate `chore(addon): bump version to ...` commit. The
  add-on Dockerfile copies the binary from `ghcr.io/canduru4/govee:latest`, so the main image
  must be published before the add-on build.
- If GitHub Actions is unavailable, run the three `pr.yml` commands locally before merging.

## Gotchas
- HA builder action: `2026.06.0` was found to ship no builder image, so it was pinned to
  `2026.02.1` (commit 88526d5). Dependabot PR #15 bumped it back to `2026.06.0`; check the
  `addon` job on the next tag and re-pin if it fails.
- Do not re-add a `cosign:` block to `addon/build.yaml`: it forces base-image signature
  verification and the floating `bookworm` base tags are unsigned (removed in db1621e).
- The MQTT client id must stay `govee2mqtt-<uuid>`; Mosquitto >= 2.1 silently denies all
  pub/sub for ids containing `/` (upstream's form).
- Per-SKU behaviour lives in `src/service/quirks.rs`. A SKU with no quirk (or no
  `iot_api_supported`) falls back to Platform API control plus a delayed poll that returns
  stale state, which shows up in HA as brightness "bounce". Fix by adding a quirk, not in HA.
- Devices added to the Govee account after `serve` starts are polled but get no HA discovery
  until the bridge restarts.
- Platform API has a daily request quota; avoid designs that poll it frequently.
- `GOVEE_LOG_SENSITIVE_DATA` logs credentials and tokens; never enable it in shared logs.
  `addon/run.sh` redacts `*_EMAIL/_KEY/_PASSWORD` when echoing env.
- `AmazonRootCA1.pem` is Amazon's public root CA and is intentionally tracked; every other
  `*.pem`/`*.key`/`*.cert`/`.env` is git-ignored and must never be committed.
- `build.rs` embeds the version from `GOVEE_CI_TAG`, else `.tag`, else `git show`.

## Conventions
- Tests are unit tests inside each module: parse a `test-data/` payload with `from_json` and
  `k9::assert_matches_snapshot!`. Adding a payload for a new SKU/issue means a new JSON in
  `test-data/` plus a new `.snap`.
- Most code is upstream's. Keep diffs against upstream small: do not reformat or retro-document
  upstream files; the owner's docblock rule applies to new code and fork-specific changes.
- Fork-specific changes are listed in the README "Fork-specific changes" section; update it
  when adding one.
