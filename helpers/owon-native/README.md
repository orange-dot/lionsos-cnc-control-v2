# OWON Native Helper Pack

This is the vendored OWON native helper pack for
`lionsos-cnc-control-v2/helpers`.

It includes:

- the Rust native app workspace under `crates/`
- saved capture sessions in the helper-pack root
- bench notes in `SCOPE_UART_SESSION_*.md`
- `scripts/` and `udev/` support files

It intentionally excludes `target/` and other bulky build outputs so the pack
stays publishable.

## Running

From this directory:

```bash
cargo run -p owon-ui
```

Useful commands:

```bash
cargo check
cargo test -p owon-analysis -p owon-ui
sudo bash scripts/install-udev-rule.sh
```

## Session Layout

Saved-session discovery is rooted at this helper-pack directory. Session groups
live beside `Cargo.toml` as:

- `*-meta.json`
- `*-debug.json`
- `*-ch*.bin`

Session metadata in this vendored pack uses relative sample file paths so the
tree can move without breaking capture loading.
