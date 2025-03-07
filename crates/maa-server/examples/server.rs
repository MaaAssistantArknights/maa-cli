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

    let server = Server::builder()
        .add_service(maa_server::server_impl::task::gen_service())
        .add_service(maa_server::server_impl::core::gen_service());

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
        server.serve_with_incoming(stream).await.unwrap();
    } else {
        println!("Using Http Port");
        let stream = tokio_stream::wrappers::TcpListenerStream::new(
            tokio::net::TcpListener::bind("127.0.0.1:50051")
                .await
                .unwrap(),
        );
        server.serve_with_incoming(stream).await.unwrap();
    }
}
