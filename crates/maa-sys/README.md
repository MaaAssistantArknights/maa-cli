# maa-sys

Raw FFI bindings for [MaaAssistantArknights](https://github.com/MaaAssistantArknights/MaaAssistantArknights) (MaaCore).

This crate exposes the C API of MaaCore as-is, with no safety guarantees.
All functions in `maa_sys::binding` are `unsafe`. If you want a safe,
idiomatic Rust API, use [`maa-core`](../maa-core) instead.

## Loading MaaCore

### Linking (default)

`maa-sys` links against `libMaaCore` at link time. If MaaCore is installed
in a non-standard location, set `MAA_CORE_DIR` to the directory containing the
shared library:

```sh
MAA_CORE_DIR=/path/to/maa cargo build
```

If `MAA_CORE_DIR` is not set, the linker searches its default library paths,
which is sufficient for system-wide or package-manager installations.

### Runtime loading

Enable the `runtime` feature to load MaaCore dynamically at startup:

```toml
maa-sys = { version = "...", features = ["runtime"] }
```

Then call `maa_sys::binding::load(path)` before using any other API:

```rust
// Load by absolute path
maa_sys::binding::load("/path/to/libMaaCore.so").unwrap();

// Or by name, searched in system library paths
maa_sys::binding::load("MaaCore").unwrap();
```

<!-- markdownlint-disable-file MD013 -->
