use directories::ProjectDirs;
use std::env::var_os;
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
    let core_name = if cfg!(target_os = "linux") {
        "libMaaCore.so"
    } else if cfg!(target_os = "macos") {
        "libMaaCore.dylib"
    } else if cfg!(target_os = "windows") {
        "MaaCore.dll"
    } else {
        panic!("Unsupported platform!");
    };
    if !lib_dir.join(core_name).exists() {
        panic!("maa core not exists, please install maa core with maa-installer firstly");
    }
    println!("cargo:rustc-link-search=native={}", lib_dir.display());
    println!("cargo:rustc-link-arg=-Wl,-rpath,{}", lib_dir.display());
}
