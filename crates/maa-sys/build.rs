#[cfg(not(feature = "runtime"))]
fn dynamic_link() {
    use std::{
        env::{
            consts::{DLL_PREFIX, DLL_SUFFIX},
            var_os,
        },
        path::PathBuf,
    };

    let core_dir = var_os("MAA_CORE_DIR")
        .map(PathBuf::from)
        .expect("MAA_CORE_DIR not set");
    let core_name = format!("{DLL_PREFIX}MaaCore{DLL_SUFFIX}");
    if !core_dir.join(core_name).exists() {
        panic!("cannot find maa core, make sure you have installed maa core at correct path");
    }
    println!("cargo:rustc-link-search=native={}", core_dir.display());
    println!("cargo:rustc-link-arg=-Wl,-rpath,{}", core_dir.display());
}

fn main() {
    #[cfg(not(feature = "runtime"))]
    dynamic_link();
}
