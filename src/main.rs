pub mod grpcweb;

use clap::Parser;

use std::sync::Arc;

use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper_util::rt::TokioIo;
use tokio::net::TcpListener;

use crate::grpcweb::grpc_web_proxy::GrpcWebProxy;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let args = Box::leak(Box::new(grpcweb::args::Args::parse()));
    let grpc_web_proxy = Arc::new(GrpcWebProxy::from_args(args));
    let listener = TcpListener::bind(grpc_web_proxy.proxy_address.clone()).await?;

    loop {
        let (stream, _) = listener.accept().await?;
        let cloned_gprc_web_proxy = grpc_web_proxy.clone();
        let io = TokioIo::new(stream);
        tokio::task::spawn(async move {
            if let Err(err) = http1::Builder::new()
                .serve_connection(
                    io,
                    service_fn(|req| {
                        let proxy = cloned_gprc_web_proxy.clone();
                        proxy.handle_grpc_web_request(req)
                    }),
                )
                .await
            {
                eprintln!("Error serving connection: {:?}", err);
            }
        });
    }
}
