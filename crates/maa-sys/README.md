# maa-sys

Raw FFI bindings for [MaaAssistantArknights](https://github.com/MaaAssistantArknights/MaaAssistantArknights) (MaaCore).

This crate exposes the C API of MaaCore as-is, with no safety guarantees.
All functions in `maa_sys::binding` are `unsafe`. If you want a safe,
idiomatic Rust API, use [`maa-core`](../maa-core) instead.

## Loading MaaCore

### Dynamic linking (default)

`maa-sys` dynamically links against `libMaaCore` at build time. The linker
searches its default library paths, which is sufficient for system-wide or
package-manager installations. If MaaCore is in a non-standard location, pass
the path via `RUSTFLAGS`:

```sh
RUSTFLAGS="-L /path/to/maa" cargo build
```

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
