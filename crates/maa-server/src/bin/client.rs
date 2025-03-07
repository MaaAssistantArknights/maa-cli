use maa_types::TaskType;

use maa_server::task::{task_state::State, NewTaskRequest};
use tokio_stream::StreamExt;
use tonic::transport::{Channel, Endpoint};

fn make_request<T>(payload: T, session_id: &str) -> tonic::Request<T> {
    let mut req = tonic::Request::new(payload);
    req.metadata_mut()
        .insert("x-session-id", session_id.parse().unwrap());
    req
}

fn get_resource_dirs() -> Vec<std::path::PathBuf> {
    use maa_dirs::{self as dirs, join};
    use std::path::PathBuf;

    #[cfg_attr(test, derive(Debug, PartialEq))]
    #[derive(Clone)]
    pub struct ResourceConfig {
        /// Resources used by global arknights client, e.g. `YostarEN`
        global_resource: Option<PathBuf>,
        /// Resources used by platform diff, subdirectories of `resource_base_dirs`, e.g.
        /// `platform_diff/iOS`
        platform_diff_resource: Option<PathBuf>,
        /// Resource base directories, a list of directories containing resource directories
        /// Not deserialized from config file
        pub(crate) resource_base_dirs: Vec<PathBuf>,
    }

    impl Default for ResourceConfig {
        fn default() -> Self {
            Self {
                resource_base_dirs: default_resource_base_dirs(),
                global_resource: None,
                platform_diff_resource: None,
            }
        }
    }

    fn default_resource_base_dirs() -> Vec<PathBuf> {
        let mut resource_dirs = Vec::new();

        if let Some(resource_dir) = dirs::find_resource() {
            tracing::debug!("Found resource directory: {}", resource_dir.display());
            resource_dirs.push(resource_dir.into_owned());
        } else {
            tracing::warn!("Resource directory not found!")
        }

        let hot_update_dir = dirs::hot_update();
        if hot_update_dir.exists() {
            tracing::debug!(
                "Found hot update resource directory: {}",
                hot_update_dir.display()
            );
            resource_dirs.push(join!(hot_update_dir, "resource"));
            resource_dirs.push(join!(hot_update_dir, "cache", "resource"));
        } else {
            tracing::warn!("Hot update resource directory not found!");
        }

        resource_dirs
    }

    impl ResourceConfig {
        /// Get all resource directories, including global and platform diff resources
        pub fn resource_dirs(&self) -> Vec<PathBuf> {
            let base_dirs = &self.resource_base_dirs;
            let mut resource_dirs = base_dirs.clone();
            if let Some(global_resource) = self.global_resource.as_ref() {
                let global_resource_dir = join!("global", global_resource, "resource");
                let full_paths = dirs::global_path(base_dirs, global_resource_dir);
                if full_paths.is_empty() {
                    tracing::warn!("Global resource {} not found", global_resource.display(),);
                } else {
                    resource_dirs.extend(full_paths);
                }
            }
            if let Some(platform_diff_resource) = self.platform_diff_resource.as_ref() {
                let platform_diff_resource_dir =
                    join!("platform_diff", platform_diff_resource, "resource");
                let full_paths = dirs::global_path(base_dirs, platform_diff_resource_dir);
                if full_paths.is_empty() {
                    tracing::warn!(
                        "Platform diff resource {} not found",
                        platform_diff_resource.display(),
                    );
                } else {
                    resource_dirs.extend(full_paths);
                }
            }

            resource_dirs
        }
    }
    ResourceConfig::default().resource_dirs()
}

#[cfg(feature = "unix-socket")]
async fn connect() -> Channel {
    use tokio::net::UnixStream;

    Endpoint::from_static("http://127.0.0.1:50051")
        .connect_with_connector(tower::service_fn(|_: tonic::transport::Uri| async {
            let path = "/tmp/tonic/testing.sock";
            Ok::<_, std::io::Error>(hyper_util::rt::TokioIo::new(
                UnixStream::connect(path).await?,
            ))
        }))
        .await
        .unwrap()
}

#[cfg(not(feature = "unix-socket"))]
async fn connect() -> Channel {
    Endpoint::try_from("http://127.0.0.1:50051")
        .unwrap()
        .connect()
        .await
        .unwrap()
}

#[tokio::main]
async fn main() {
    let channel = connect().await;
    println!("Connected to server");

    let mut coreclient = maa_server::core::core_client::CoreClient::new(channel.clone());
    let success = coreclient
        .load_core(maa_server::core::CoreConfig {
            static_ops: Some(maa_server::core::core_config::StaticOptions {
                cpu_ocr: true,
                gpu_ocr: None,
            }),
            log_ops: Some(maa_server::core::core_config::LogOptions {
                path: "/home/maa/".to_owned(),
                level: maa_server::core::core_config::LogLevel::Debug.into(),
            }),
            lib_path: maa_dirs::find_library()
                .map(|path| {
                    path.join(maa_dirs::MAA_CORE_LIB)
                        .to_str()
                        .unwrap()
                        .to_owned()
                })
                .unwrap_or(maa_dirs::MAA_CORE_LIB.to_owned()),
            resource_dirs: get_resource_dirs()
                .into_iter()
                .map(|p| p.to_str().unwrap().to_owned())
                .collect(),
        })
        .await
        .unwrap()
        .into_inner();

    if !success {
        println!("Core has been configured");
    }

    let mut taskclient = maa_server::task::task_client::TaskClient::new(channel);

    let session_id = taskclient
        .new_connection(maa_server::task::NewConnectionRequest {
            conncfg: Some(maa_server::task::new_connection_request::ConnectionConfig {
                adb_path: "adb".to_owned(),
                address: "192.168.240.112:5555".to_owned(),
                config: "Waydroid".to_owned(),
            }),
            instcfg: Some(maa_server::task::new_connection_request::InstanceOptions {
                touch_mode:
                    maa_server::task::new_connection_request::instance_options::TouchMode::MaaTouch
                        .into(),
                deployment_with_pause: false,
                adb_lite_enabled: false,
                kill_adb_on_exit: true,
            }),
        })
        .await
        .unwrap()
        .into_inner();

    println!("session_id: {}", session_id);

    let mut channel = taskclient
        .task_state_update(make_request((), &session_id))
        .await
        .unwrap()
        .into_inner();

    let mut payload = NewTaskRequest::default();
    payload.set_task_type(TaskType::StartUp.into());
    payload.task_params =
        r#"{ "enable": true, "client_type": "Official", "start_game_enabled": true, "account_name": "" }"#.to_owned();
    taskclient
        .append_task(make_request(payload, &session_id))
        .await
        .unwrap();
    println!("Add task StartUp");
    let mut payload = NewTaskRequest::default();
    payload.set_task_type(TaskType::Fight.into());
    payload.task_params = r#" { "stage": "1-7" } "#.to_owned();
    let id = taskclient
        .append_task(make_request(payload, &session_id))
        .await
        .unwrap()
        .into_inner();
    println!("Add task Fight");
    taskclient
        .deactivate_task(make_request(id, &session_id))
        .await
        .unwrap();
    println!("Deactivate task Fight");
    let mut payload = NewTaskRequest::default();
    payload.set_task_type(TaskType::Fight.into());
    // payload.task_params = r#" { "stage": "EA-6" } "#.to_owned();
    payload.task_params = r#" { } "#.to_owned();
    taskclient
        .append_task(make_request(payload, &session_id))
        .await
        .unwrap()
        .into_inner();
    println!("Add task Fight EA-6");
    taskclient
        .start_tasks(make_request((), &session_id))
        .await
        .unwrap();

    println!("Starting show callback");
    loop {
        if let Some(msg) = channel.next().await {
            let msg = msg.unwrap();
            println!("{}: {}", msg.state, msg.content);
            if msg.state == State::AllTasksCompleted as i32 {
                break;
            }
        }
    }

    println!("Grab remote log");
    // skip first 10 log
    let logs = taskclient
        .fetch_logs(make_request(10, &session_id))
        .await
        .unwrap();
    println!("{:?}", logs.into_inner());

    println!("Clean up");
    taskclient
        .close_connection(make_request((), &session_id))
        .await
        .unwrap();
}
