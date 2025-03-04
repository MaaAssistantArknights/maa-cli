use std::time::Duration;

use maa_types::TaskType;

use maa_server::task::NewTaskRequest;
use tokio::time::sleep;

fn make_request<T>(payload: T, session_id: &str) -> tonic::Request<T> {
    let mut req = tonic::Request::new(payload);
    req.metadata_mut()
        .insert("x-session-key", session_id.parse().unwrap());
    req
}

#[tokio::main(flavor = "current_thread")]
async fn main() {
    let mut coreclient =
        maa_server::core::core_client::CoreClient::connect("http://127.0.0.1:50051")
            .await
            .unwrap();
    coreclient
        .load_core(maa_server::core::CoreConfig {
            static_ops: Some(maa_server::core::core_config::StaticOptions {
                cpu_ocr: true,
                gpu_ocr: None,
            }),
        })
        .await
        .unwrap();

    let mut taskclient =
        maa_server::task::task_client::TaskClient::connect("http://127.0.0.1:50051")
            .await
            .unwrap();

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
    payload.task_params = r#""#.to_owned();
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
    taskclient
        .start_tasks(make_request((), &session_id))
        .await
        .unwrap();

    println!("Sleep now");
    // Todo: subscribe to task_state_update to exit
    sleep(Duration::from_secs(60)).await;

    taskclient
        .close_connection(make_request((), &session_id))
        .await
        .unwrap();
}
