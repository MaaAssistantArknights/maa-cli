use tonic::transport::Server;
use tracing_subscriber::{filter, fmt, layer::SubscriberExt, util::SubscriberInitExt, Layer};

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

    #[cfg(not(feature = "unix-socket"))]
    let stream = {
        tokio_stream::wrappers::TcpListenerStream::new(
            tokio::net::TcpListener::bind("127.0.0.1:50051")
                .await
                .unwrap(),
        )
    };

    #[cfg(feature = "unix-socket")]
    let stream = {
        let path = "/tmp/tonic/testing.sock";
        std::fs::create_dir_all(std::path::Path::new(path).parent().unwrap()).unwrap();
        tokio_stream::wrappers::UnixListenerStream::new(
            tokio::net::UnixListener::bind(path).unwrap(),
        )
    };

    Server::builder()
        .add_service(maa_server::server_impl::task::gen_service())
        .add_service(maa_server::server_impl::core::gen_service())
        .serve_with_incoming(stream)
        .await
        .unwrap();
}
