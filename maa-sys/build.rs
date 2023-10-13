#[cfg(not(feature = "runtime"))]
fn static_link() {
    use std::env::{
        consts::{DLL_PREFIX, DLL_SUFFIX},
        var_os,
    };
    use std::path::PathBuf;

    let core_dir = var_os("MAA_CORE_DIR")
        .map(PathBuf::from)
        .expect("MAA_CORE_DIR not set");
    let core_name = format!("{}MaaCore{}", DLL_PREFIX, DLL_SUFFIX);
    if !core_dir.join(core_name).exists() {
        panic!("cannot find maa core, make sure you have installed maa core at correct path");
    }
    // Setup linker flags
    if cfg!(unix) {
        println!("cargo:rustc-link-search=native={}", core_dir.display());
        println!("cargo:rustc-link-arg=-Wl,-rpath,{}", core_dir.display());
    } else if cfg!(windows) {
        println!("cargo:rustc-link-search=native={}", core_name);
    }
}

fn main() {
    #[cfg(not(feature = "runtime"))]
    static_link();
}
