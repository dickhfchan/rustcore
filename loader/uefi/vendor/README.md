# Vendored Dependencies

Populate this directory with `cargo vendor` to cache the loader's firmware
dependencies locally:

```
cargo vendor --manifest-path loader/uefi/Cargo.toml loader/uefi/vendor
```

Once filled, builds can run with `--offline` or in CI environments without
network access.
