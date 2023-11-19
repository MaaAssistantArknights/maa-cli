pub const MAA_CLI_EXE: &str = if cfg!(windows) { "maa.exe" } else { "maa" };

pub const MAA_CORE_LIB: &str = if cfg!(unix) {
    if cfg!(target_os = "macos") {
        "libMaaCore.dylib"
    } else {
        "libMaaCore.so"
    }
} else {
    "MaaCore.dll"
};

pub const MAA_CLI_VERSION: &str = env!("CARGO_PKG_VERSION");
