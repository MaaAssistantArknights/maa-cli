use maa_server::{
    prelude::HEADER_SESSION_ID,
    task::{ModifyTaskRequest, NewTaskRequest, TaskState},
};
use maa_types::{TaskStateType, TaskType};
use tokio_stream::StreamExt;
use tonic::transport::Endpoint;
use tracing_subscriber::{Layer, filter, fmt, layer::SubscriberExt, util::SubscriberInitExt};

fn make_request<T>(payload: T, session_id: &str) -> tonic::Request<T> {
    let mut req = tonic::Request::new(payload);
    req.metadata_mut()
        .insert(HEADER_SESSION_ID, session_id.parse().unwrap());
    req
}

fn get_resource_dirs() -> Vec<std::path::PathBuf> {
    use maa_dirs::{self as dirs, join};

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

#[tokio::main]
async fn main() {
    tracing_subscriber::Registry::default()
        .with(
            fmt::layer()
                .compact()
                .with_ansi(true)
                .with_filter(filter::LevelFilter::DEBUG),
        )
        .init();

    let channel = {
        #[cfg(unix)]
        {
            tracing::debug!("Using Unix Socket");
            Endpoint::from_static("http://127.0.0.1:50051")
                .connect_with_connector(tower::service_fn(|_: tonic::transport::Uri| async {
                    let path = "/tmp/maa_server/testing.sock";
                    Ok::<_, std::io::Error>(hyper_util::rt::TokioIo::new(
                        tokio::net::UnixStream::connect(path).await?,
                    ))
                }))
                .await
                .unwrap()
        }
        #[cfg(windows)]
        {
            tracing::debug!("Using Http Port");
            Endpoint::try_from("http://127.0.0.1:50051")
                .unwrap()
                .connect()
                .await
                .unwrap()
        }
    };
    tracing::info!("Connected to server");

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
        tracing::warn!("Core has been configured");
    }

    let mut taskclient = maa_server::task::task_client::TaskClient::new(channel);

    let Ok(session_id) = taskclient
        .new_connection(maa_server::task::NewConnectionRequest {
            conncfg: Some(maa_server::task::new_connection_request::ConnectionConfig {
                adb_path: "adb".to_owned(),
                address: "192.168.240.112:5555".to_owned(),
                config: "Waydroid".to_owned(),
            }),
            instcfg: Some(maa_server::task::new_connection_request::InstanceOptions {
                touch_mode: maa_types::TouchMode::MaaTouch.into(),
                deployment_with_pause: false,
                adb_lite_enabled: false,
                kill_adb_on_exit: false,
            }),
        })
        .await
        .map(|resp| resp.into_inner())
    else {
        tracing::error!("Failed to create new connection");
        tracing::info!("Close Server");
        coreclient.unload_core(()).await.unwrap();
        return;
    };

    tracing::debug!("session_id: {}", session_id);

    let mut channel = taskclient
        .task_state_update(make_request((), &session_id))
        .await
        .unwrap()
        .into_inner();

    taskclient
        .append_task(make_request(
            NewTaskRequest {
                task_type: TaskType::StartUp.into(),
                task_params: r#"{ "enable": true, "client_type": "Official", "start_game_enabled": true, "account_name": "" }"#.to_owned(),
            },
            &session_id,
        ))
        .await
        .unwrap();
    tracing::info!("Add task StartUp");

    let id = taskclient
        .append_task(make_request(
            NewTaskRequest {
                task_type: TaskType::Fight.into(),
                task_params: r#" { "stage": "1-7" } "#.to_owned(),
            },
            &session_id,
        ))
        .await
        .unwrap()
        .into_inner();
    tracing::info!("Add task Fight 1-7");

    taskclient
        .deactivate_task(make_request(id, &session_id))
        .await
        .unwrap();
    tracing::info!("Deactivate task Fight 1-7");

    let _id = taskclient
        .append_task(make_request(
            NewTaskRequest {
                task_type: TaskType::Fight.into(),
                task_params: r#" { "stage" : "Annihilation", "times" : 1 } "#.to_owned(),
            },
            &session_id,
        ))
        .await
        .unwrap()
        .into_inner();
    tracing::info!("Add task Annihilation");

    let _id = taskclient
        .append_task(make_request(
            NewTaskRequest {
                task_type: TaskType::Fight.into(),
                task_params: r#" { "stage" : "EA-6" } "#.to_owned(),
            },
            &session_id,
        ))
        .await
        .unwrap()
        .into_inner();
    tracing::info!("Add task Fight");

    let id = taskclient
        .append_task(make_request(
            NewTaskRequest {
                task_type: TaskType::Mall.into(),
                task_params: r#" { "enable": false,
                "shopping" : true,
                "credit_fight": true,
                "buy_first": ["招聘许可", "加急许可", "龙门币"],
                "blacklist": ["碳", "家具"] } "#
                    .to_owned(),
            },
            &session_id,
        ))
        .await
        .unwrap()
        .into_inner();
    tracing::info!("Add inactive task Mall");

    taskclient
        .modify_task(make_request(
            ModifyTaskRequest {
                task_id: Some(id),
                task_params: r#" { "enable": true,
                "shopping" : true,
                "credit_fight": true,
                "buy_first": ["招聘许可", "加急许可", "龙门币"],
                "blacklist": ["碳", "家具"] } "#
                    .to_owned(),
            },
            &session_id,
        ))
        .await
        .unwrap();
    tracing::info!("Activate task Mall");

    let _id = taskclient
        .append_task(make_request(
            NewTaskRequest {
                task_type: TaskType::Award.into(),
                task_params: r#" { } "#.to_owned(),
            },
            &session_id,
        ))
        .await
        .unwrap()
        .into_inner();
    tracing::info!("Add task Award");

    let _id = taskclient
        .append_task(make_request(
            NewTaskRequest {
                task_type: TaskType::Roguelike.into(),
                task_params: r#" { "theme": "Mizuki", "starts_count": 1 } "#.to_owned(),
            },
            &session_id,
        ))
        .await
        .unwrap()
        .into_inner();
    tracing::info!("Add task Roguelike");

    taskclient
        .start_tasks(make_request((), &session_id))
        .await
        .unwrap();
    tracing::info!("Start Tasks");

    tracing::info!("Starting show callback");
    while let Some(msg) = channel.next().await {
        let TaskState { content, state } = msg.unwrap();
        let state: TaskStateType = state.try_into().unwrap();
        tracing::debug!("{:?}: {}", state, content);
        if state == TaskStateType::AllTasksCompleted {
            break;
        }
    }
    drop(channel);

    tracing::info!("Grab remote log");
    // skip first 100 log
    let logs = taskclient
        .fetch_logs(make_request(100, &session_id))
        .await
        .unwrap();
    tracing::debug!("{:?}", logs.into_inner());

    tracing::info!("Clean up");
    taskclient
        .close_connection(make_request((), &session_id))
        .await
        .unwrap();
    drop(taskclient);
    tracing::info!("Close Server");
    coreclient.unload_core(()).await.unwrap();
}
