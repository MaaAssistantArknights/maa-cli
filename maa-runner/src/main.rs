mod config;
use asst::{AsstConfig, Connection};
use config::{asst, task};
use task::{TaskList, TaskType};

mod maacore;
use maacore::Assistant;

use maa_utils::config::{Error as ConfigError, FindFile};
use maa_utils::dirs::{Dirs, ProjectDirs};

use serde_json::Value;

use clap::Parser;

#[derive(Parser)]
#[clap(author, about, version)]
enum CLI {
    #[clap(about = "Run a task defined by a config file")]
    Run {
        #[clap(help = "Task name")]
        task: String,
        #[clap(short, long, help = "ADB serial number of the device")]
        adb: Option<String>,
        #[clap(short, long, help = "Config for maacore")]
        config: Option<String>,
        #[clap(short, long, help = "Verbose output")]
        verbose: bool,
    },
    #[clap(about = "Show version information")]
    Version,
}

fn main() {
    let project = ProjectDirs::from("maa");

    let cli = CLI::parse();

    match cli {
        CLI::Run {
            task,
            adb,
            config,
            verbose,
        } => {
            if let Some(dir) = project.state_dir() {
                if !dir.exists() {
                    std::fs::create_dir_all(&dir).unwrap();
                }
                Assistant::set_user_dir(dir).unwrap();
            } else {
                panic!("Failed to get state directory!");
            }

            if let Some(dir) = project.data_dir() {
                if !dir.exists() {
                    panic!("Resource directory not exists!");
                }
                Assistant::load_resource(dir).unwrap();
            } else {
                panic!("Failed to get data directory!");
            }

            let assistant = Assistant::new(None, None);

            if let Some(dir) = project.config_dir() {
                let asst_config = AsstConfig::find_file(&dir.join("asst"));
                match asst_config {
                    Ok(asst_config) => {
                        if let Some(options) = asst_config.instance_options {
                            if verbose {
                                println!("Touch mode: {:?}", options.touch_mode);
                            }
                            assistant
                                .set_instance_option(2, options.touch_mode)
                                .unwrap();
                            if let Some(v) = options.deployment_with_pause {
                                if verbose {
                                    println!("Deployment with pause: {}", v);
                                }
                                assistant.set_instance_option(3, v).unwrap();
                            }
                            if let Some(v) = options.adb_lite_enabled {
                                if verbose {
                                    println!("ADB lite enabled: {}", v);
                                }
                                assistant.set_instance_option(4, v).unwrap();
                            }
                            if let Some(v) = options.kill_adb_on_exit {
                                if verbose {
                                    println!("Kill ADB on exit: {}", v);
                                }
                                assistant.set_instance_option(5, v).unwrap();
                            }
                        }
                        if let Some(connection) = asst_config.connection {
                            match connection {
                                Connection::ADB {
                                    adb_path,
                                    device,
                                    config: f_config,
                                } => {
                                    if verbose {
                                        println!("ADB path: {}", adb_path);
                                        println!("ADB device: {}", device);
                                        println!("ADB config: {}", f_config);
                                    }
                                    let adb_device = adb.unwrap_or(device);
                                    let config = config.unwrap_or(f_config);
                                    assistant.connect(adb_path, adb_device, config).unwrap();
                                }
                                Connection::Playcover {} => {
                                    println!("Warning: this is not implemented yet!");
                                    println!("Playcover is not supported yet!");
                                }
                            }
                        }
                    }
                    Err(ConfigError::FileNotFound(_)) => {
                        if verbose {
                            println!("No asst config found, using default settings");
                            println!("ADB path: {}", asst::default_adb_path());
                            println!("ADB device: {}", asst::default_device());
                            println!("ADB config: {}", asst::default_config());
                        }
                        assistant
                            .set_instance_option(2, asst::TouchMode::default())
                            .unwrap();

                        let adb_path = asst::default_adb_path();
                        let adb_device = adb.unwrap_or(asst::default_device());
                        let config = config.unwrap_or(asst::default_config());
                        assistant.connect(adb_path, adb_device, config).unwrap();
                    }
                    Err(err) => {
                        panic!("Failed to load connection config: {}", err);
                    }
                }

                match TaskList::find_file(&dir.join("tasks").join(&task)) {
                    Ok(task_list) => {
                        for task in task_list.tasks {
                            if task.is_active() {
                                let task_type = task.get_type();
                                let params = &mut task.get_params();
                                if *task_type == TaskType::Infrast {
                                    if let Some(v) = params.get_mut("filename") {
                                        assert!(v.is_string());
                                        *v = Value::String(
                                            dir.join("infrast")
                                                .join(v.as_str().unwrap())
                                                .to_str()
                                                .unwrap()
                                                .to_string(),
                                        );
                                    }
                                }
                                if verbose {
                                    println!("Task: {:?}", task_type);
                                    println!("Params: {}", serde_json::to_string(&params).unwrap());
                                }
                                assistant
                                    .append_task(
                                        task.get_type(),
                                        serde_json::to_string(&params).unwrap(),
                                    )
                                    .unwrap();
                            }
                        }
                    }
                    Err(err) => {
                        panic!("Failed to load task config: {}", err);
                    }
                }
            } else {
                panic!("Failed to get config directory!");
            }

            assistant.start().unwrap();

            while assistant.running() {
                std::thread::sleep(std::time::Duration::from_millis(100));
            }

            assistant.stop().unwrap();
        }
        CLI::Version => {
            let cli_version = env!("CARGO_PKG_VERSION");
            let core_version = Assistant::get_version().unwrap();
            println!("maa-cli v{}\nMaaCore {}", cli_version, core_version);
        }
    }
}
