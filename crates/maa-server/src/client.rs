use maa_types::TaskType;

use maa_server::task::NewTaskRequest;
use tokio_stream::StreamExt;
use tonic::transport::{Channel, Endpoint};

fn make_request<T>(payload: T, session_id: &str) -> tonic::Request<T> {
    let mut req = tonic::Request::new(payload);
    req.metadata_mut()
        .insert("x-session-id", session_id.parse().unwrap());
    req
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

    let mut coreclient = maa_server::core::core_client::CoreClient::new(channel.clone());
    coreclient
        .load_core(maa_server::core::CoreConfig {
            static_ops: Some(maa_server::core::core_config::StaticOptions {
                cpu_ocr: true,
                gpu_ocr: None,
            }),
            log_ops: Some(maa_server::core::core_config::LogOptions {
                path: "test".to_owned(),
                level: maa_server::core::core_config::LogLevel::Debug.into(),
            }),
        })
        .await
        .unwrap();

    let mut taskclient = maa_server::task::task_client::TaskClient::new(channel);

    println!("Connected to server");

    let session_id = taskclient
        .new_connection(maa_server::task::NewConnectionRequst {
            conncfg: Some(maa_server::task::new_connection_requst::ConnectionConfig {
                adb_path: "adb".to_owned(),
                address: "192.168.240.112:5555".to_owned(),
                config: "Waydroid".to_owned(),
            }),
            instcfg: Some(maa_server::task::new_connection_requst::InstanceOptions {
                touch_mode:
                    maa_server::task::new_connection_requst::instance_options::TouchMode::MaaTouch
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
        .deactive_task(make_request(id, &session_id))
        .await
        .unwrap();
    println!("Deactive task Fight");
    let mut payload = NewTaskRequest::default();
    payload.set_task_type(TaskType::Fight.into());
    payload.task_params = r#" { "stage": "1-7" } "#.to_owned();
    taskclient
        .append_task(make_request(payload, &session_id))
        .await
        .unwrap()
        .into_inner();
    println!("Add task Fight 1-7");
    taskclient
        .start_tasks(make_request((), &session_id))
        .await
        .unwrap();

    println!("Starting show callback");
    loop {
        if let Some(msg) = channel.next().await {
            let msg = msg.unwrap();
            println!("{}: {}", msg.state, msg.content);
            if msg.content.contains("finished_tasks") {
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
