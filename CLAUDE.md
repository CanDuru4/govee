@AGENTS.md

## Claude Code
- After Rust edits, run the three `pr.yml` commands from AGENTS.md.
  No Rust toolchain may be on PATH on this Mac; if `cargo` is missing, say so instead of skipping silently.
- Snapshot tests write `src/__k9_snapshots__/*.snap`; review any `.snap` diff before staging it.
- Use plan mode for changes to `src/service/quirks.rs`, `.github/workflows/build.yml`, or
  `addon/` release files; a bad tag or add-on version breaks the live HA add-on update.
