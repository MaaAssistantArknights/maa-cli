#[cfg(not(feature = "runtime"))]
fn dynamic_link() {
    use std::{
        env::{
            consts::{DLL_PREFIX, DLL_SUFFIX},
            var_os,
        },
        path::PathBuf,
    };

    println!("cargo:rerun-if-env-changed=MAA_CORE_DIR");
    if let Some(core_dir) = var_os("MAA_CORE_DIR").map(PathBuf::from) {
        let core_name = format!("{DLL_PREFIX}MaaCore{DLL_SUFFIX}");
        if !core_dir.join(core_name).exists() {
            panic!(
                "libMaaCore not found in MAA_CORE_DIR ({}), make sure MaaCore is installed at that path",
                core_dir.display()
            );
        }
        println!("cargo:rustc-link-search=native={}", core_dir.display());
        println!("cargo:rustc-link-arg=-Wl,-rpath,{}", core_dir.display());
    }
}

fn main() {
    #[cfg(not(feature = "runtime"))]
    dynamic_link();
}
