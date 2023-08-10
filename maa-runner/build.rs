use maa_utils::dirs::{Dirs, ProjectDirs};

fn main() {
    let project = ProjectDirs::from("maa");
    let dir = project.data_dir().unwrap();
    let dir_path = dir.to_str().unwrap();
    println!("cargo:rustc-link-search=native={}/lib", dir_path);
    println!("cargo:rustc-link-arg=-Wl,-rpath,{}/lib", dir_path);
}
