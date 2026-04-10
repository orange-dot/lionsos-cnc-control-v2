# Helpers Workspace

This subtree carries publish-friendly helper artifacts that support hardware
bring-up, bench capture, and analysis work around `lionsos-cnc-control-v2`.

Current helper packs:

- `owon-native/`
  - vendored OWON native Rust app
  - saved capture sessions and bench notes
  - udev rule and helper scripts
  - offline analysis GUI and CLI tooling

The intent is that helper content needed for publication lives in this repo and
does not require a second checkout.
