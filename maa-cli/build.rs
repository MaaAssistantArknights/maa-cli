use directories::ProjectDirs;
use std::env::{
    consts::{DLL_PREFIX, DLL_SUFFIX},
    var_os,
};
use std::path::PathBuf;

fn get_data_dir(proj: Option<ProjectDirs>) -> PathBuf {
    if let Some(maa_dir) = var_os("MAA_DATA_DIR") {
        PathBuf::from(maa_dir)
    } else if let Some(xdg_dir) = var_os("XDG_DATA_HOME") {
        PathBuf::from(xdg_dir).join("maa")
    } else if let Some(dirs) = proj {
        dirs.data_dir().to_path_buf()
    } else {
        panic!("Failed to get data directory!")
    }
}

fn main() {
    let proj = ProjectDirs::from("com", "loong", "maa");
    let data_dir = get_data_dir(proj);
    let lib_dir = data_dir.join("lib");
    let core_name = format!("{}MaaCore{}", DLL_PREFIX, DLL_SUFFIX);
    if !lib_dir.join(core_name).exists() {
        panic!("cannot find maa core, make sure you have installed maa core at correct path");
    }
    if cfg!(target_os = "windows") {
        println!("cargo:rustc-link-arg=/LIB:{}", lib_dir.display());
        // println!("cargo:rustc-link-arg=/LIBPATH:{}", lib_dir.display());
    } else {
        println!("cargo:rustc-link-arg=-Wl,-rpath,{}", lib_dir.display());
    }
}
