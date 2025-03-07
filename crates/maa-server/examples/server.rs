use tokio_util::sync::CancellationToken;
use tonic::transport::Server;
use tracing_subscriber::{filter, fmt, layer::SubscriberExt, util::SubscriberInitExt, Layer};

const USING_UDS: bool = cfg!(unix);

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

    let cancel_token = CancellationToken::new();
    let child_cancel_token = cancel_token.child_token();

    let timeout = std::time::Duration::from_micros(100);

    let server = Server::builder()
        .add_service(maa_server::server_impl::task::gen_service())
        // need to be the parent node
        .add_service(maa_server::server_impl::core::gen_service(cancel_token));

    if USING_UDS {
        println!("Using Unix Socket");
        let path = "/tmp/tonic/testing.sock";
        let path = std::path::Path::new(path);
        if path.exists() {
            std::fs::remove_file(path).unwrap()
        } else {
            std::fs::create_dir_all(path.parent().unwrap()).unwrap()
        }
        let stream = tokio_stream::wrappers::UnixListenerStream::new(
            tokio::net::UnixListener::bind(path).unwrap(),
        );
        tokio::select!(
            _ = server.serve_with_incoming_shutdown(stream, async{
                // used to cancel running connections
                let token = child_cancel_token.child_token();
                token.cancelled().await
            }) => {}
            // make sure connection is closed
            _ = child_cancel_token.cancelled() => {tokio::time::sleep(timeout).await}
        );

        if maa_sys::binding::loaded() {
            println!("Clean Up");
            maa_sys::binding::unload();
        }
    } else {
        println!("Using Http Port");
        let stream = tokio_stream::wrappers::TcpListenerStream::new(
            tokio::net::TcpListener::bind("127.0.0.1:50051")
                .await
                .unwrap(),
        );
        tokio::select!(
            _ = server.serve_with_incoming_shutdown(stream, async{
                let token = child_cancel_token.child_token();
                token.cancelled().await
            }) => {}
            _ = child_cancel_token.cancelled() => {tokio::time::sleep(timeout).await}
        );

        if maa_sys::binding::loaded() {
            println!("Clean Up");
            maa_sys::binding::unload();
        }
    }
}
