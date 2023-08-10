mod config;
use asst::{AsstConfig, Connection};
use config::{asst, task};
use task::{TaskList, TaskType};

mod maacore;
use maacore::Assistant;

use maa_utils::config::{Error as ConfigError, FindFile};
use maa_utils::dirs::{Dirs, ProjectDirs};

use serde_json::Value;

use anyhow::{anyhow, Result};
use std::path::PathBuf;
use std::process::ExitCode;

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
        #[clap(short, long, help = "Verbose output")]
        verbose: bool,
    },
    #[clap(about = "Show version information")]
    Version,
}

trait DirExists: Sized {
    fn exist_or_create(self) -> Result<Self>;
    fn exist_or_err(self) -> Result<Self>;
}

impl DirExists for PathBuf {
    fn exist_or_create(self) -> Result<Self> {
        if !self.exists() {
            std::fs::create_dir_all(&self)?;
        }
        Ok(self)
    }

    fn exist_or_err(self) -> Result<Self> {
        if !self.exists() {
            return Err(anyhow!("{} does not exist!", self.display()));
        }
        Ok(self)
    }
}

fn main() -> Result<std::process::ExitCode> {
    let project = ProjectDirs::from("maa");

    let cli = CLI::parse();

    match cli {
        CLI::Run { task, adb, verbose } => {
            let state_dir = project
                .state_dir()
                .ok_or(anyhow!("Failed to get state directory!"))?
                .exist_or_create()?;
            Assistant::set_user_dir(state_dir)?;

            let data_dir = project
                .data_dir()
                .ok_or(anyhow!("Failed to get data directory!"))?
                .exist_or_err()?;
            Assistant::load_resource(data_dir)?;

            let assistant = if verbose {
                Assistant::new(Some(maacore::default_callback), None)
            } else {
                Assistant::new(None, None)
            };

            let config_dir = project
                .config_dir()
                .ok_or(anyhow!("Failed to get config directory!"))?
                .exist_or_err()?;

            let asst_config = AsstConfig::find_file(&config_dir.join("asst"));
            match asst_config {
                Ok(asst_config) => {
                    if let Some(options) = asst_config.instance_options {
                        if verbose {
                            println!("Touch mode: {:?}", options.touch_mode);
                        }
                        assistant.set_instance_option(2, options.touch_mode)?;
                        if let Some(v) = options.deployment_with_pause {
                            if verbose {
                                println!("Deployment with pause: {}", v);
                            }
                            assistant.set_instance_option(3, v)?;
                        }
                        if let Some(v) = options.adb_lite_enabled {
                            if verbose {
                                println!("ADB lite enabled: {}", v);
                            }
                            assistant.set_instance_option(4, v)?;
                        }
                        if let Some(v) = options.kill_adb_on_exit {
                            if verbose {
                                println!("Kill ADB on exit: {}", v);
                            }
                            assistant.set_instance_option(5, v)?;
                        }
                    }
                    if let Some(connection) = asst_config.connection {
                        match connection {
                            Connection::ADB {
                                adb_path,
                                device,
                                config,
                            } => {
                                if verbose {
                                    println!("ADB path: {}", adb_path);
                                    println!("ADB device: {}", device);
                                    println!("ADB config: {}", config);
                                }
                                let adb_device = adb.unwrap_or(device);
                                assistant.connect(adb_path, adb_device, config)?;
                            }
                            Connection::Playcover {} => {
                                eprintln!("Playcover is not supported yet!");
                                return Ok(ExitCode::FAILURE);
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
                    assistant.set_instance_option(2, asst::TouchMode::default())?;

                    let adb_path = asst::default_adb_path();
                    let adb_device = adb.unwrap_or(asst::default_device());
                    let config = asst::default_config();
                    assistant.connect(adb_path, adb_device, config)?;
                }
                Err(err) => {
                    panic!("Failed to load connection config: {}", err);
                }
            }

            match TaskList::find_file(&config_dir.join("tasks").join(&task)) {
                Ok(task_list) => {
                    for task in task_list.tasks {
                        if task.is_active() {
                            let task_type = task.get_type();
                            let params = &mut task.get_params();
                            if *task_type == TaskType::Infrast {
                                if let Some(v) = params.get_mut("filename") {
                                    assert!(v.is_string());
                                    *v = Value::String(
                                        config_dir
                                            .join("infrast")
                                            .join(v.as_str().ok_or(anyhow!("Invalid filename!"))?)
                                            .to_str()
                                            .ok_or(anyhow!("Invalid filename!"))?
                                            .to_string(),
                                    );
                                }
                            }
                            if verbose {
                                println!("Task: {:?}", task_type);
                                println!("Params: {}", serde_json::to_string(&params)?);
                            }
                            assistant
                                .append_task(task.get_type(), serde_json::to_string(&params)?)?;
                        }
                    }
                }
                Err(err) => {
                    panic!("Failed to load task config: {}", err);
                }
            }

            assistant.start()?;

            while assistant.running() {
                std::thread::sleep(std::time::Duration::from_millis(100));
            }

            assistant.stop()?;
        }
        CLI::Version => {
            let cli_version = env!("CARGO_PKG_VERSION");
            let core_version = Assistant::get_version()?;
            println!("maa-cli v{}\nMaaCore {}", cli_version, core_version);
        }
    }

    return Ok(ExitCode::SUCCESS);
}
