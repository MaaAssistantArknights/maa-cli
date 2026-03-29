# maa-sys

Raw FFI bindings for [MaaAssistantArknights](https://github.com/MaaAssistantArknights/MaaAssistantArknights) (MaaCore).

This crate exposes the C API of MaaCore as-is, with no safety guarantees.
All functions in `maa_sys::binding` are `unsafe`. If you want a safe,
idiomatic Rust API, use [`maa-core`](../maa-core) instead.

## Loading MaaCore

### Runtime loading (default)

By default, `maa-sys` enables the `runtime` feature and loads MaaCore
dynamically at startup:

```toml
maa-sys = { version = "...", features = ["runtime"] }
```

Call `maa_sys::binding::load(path)` before using any other API:

```rust
// Load by absolute path
maa_sys::binding::load("/path/to/libMaaCore.so").unwrap();

// Or by name, searched in system library paths
maa_sys::binding::load("MaaCore").unwrap();
```

### Link-time linking

Without the `runtime` feature, `maa-sys` links against `libMaaCore` at link
time. The linker searches its default library paths, which is sufficient for
system-wide or package-manager installations. If MaaCore is in a non-standard
location, set `MAA_CORE_DIR` to the directory containing the shared library:

```sh
MAA_CORE_DIR=/path/to/maa cargo build --no-default-features
```

<!-- markdownlint-disable-file MD013 -->
