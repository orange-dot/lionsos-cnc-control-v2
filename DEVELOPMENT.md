# Development

Use this file as the local implementation entrypoint for the repo.

## Canonical Smoke Path

```bash
python3 -m py_compile meta.py
cargo check --locked --manifest-path helpers/owon-native/Cargo.toml
```

This is the shortest truthful public proof path today. It validates the top
level generator script and the native helper workspace without pretending the
full Pi-side or MCU-side toolchains are available.

## Full Pi-side Build

Pi-side builds require:

- `LIONSOS=/path/to/lionsos`
- `MICROKIT_SDK=/path/to/microkit-sdk`

Then run:

```bash
make
```

## MCU-side Build

The MCU executor build lives in `mcu/` and requires an AVR toolchain.

## Reading Order

1. Root `README.md`
2. `docs/README.md`
3. `docs/ARCHITECTURE.md`
4. `docs/RPI3B_MCU_HW_TEST_PLAYBOOK.md` when you need bench detail
