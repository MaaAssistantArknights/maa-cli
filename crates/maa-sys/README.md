# maa-sys

[MaaAssistantArknights](https://github.com/MaaAssistantArknights/MaaAssistantArknights) raw binding and safe wrapper for Rust.

## Load MaaCore

This crate depends on the shared library MaaCore, which can be linked at compile time or loaded at runtime.

### Compile time Linking

`maa-sys` will link to `libMaaCore` at compile time by default. To find the shared library, you need to set the environment variable `MAA_CORE_DIR` to the directory containing the shared library.

### Runtime loading

You can also load the shared library at runtime by enabling the `runtime` feature. In this case, you need to call `maa_sys::binding::load(path)` to load the shared library at runtime. The `path` can be the library name `MaaCore` or an absolute path to the shared library. If the `path` is a name, the shared library will be searched in the system library paths.

<!-- markdownlint-disable-file MD013 -->
