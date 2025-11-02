use std::sync::Arc;

use http_body_util::BodyExt;
use http_body_util::Full;
use hyper::body::Bytes;
use hyper::{Request, Response};

use crate::grpcweb::args::Args;
use crate::grpcweb::grpc_client::GrpcClient;
use crate::grpcweb::grpc_client::GrpcClientHandler;
use crate::grpcweb::grpc_web_server::GrpcWebServer;
use crate::grpcweb::grpc_web_server::GrpcWebServerHandler;

/// Main proxy action
/// hold all parameters and handlers
pub(crate) struct GrpcWebProxy<'a, T: GrpcClientHandler, S: GrpcWebServerHandler> {
    pub proxy_address: String,
    pub forward_address: String,
    pub forward_host: &'a str,
    pub forward_port: u16,
    grpc_client: T,
    grpc_web_server: S,
}
impl<T: GrpcClientHandler, S: GrpcWebServerHandler> GrpcWebProxy<'_, T, S> {
    pub async fn handle_grpc_web_request<F>(
        self: Arc<Self>,
        req: Request<F>,
    ) -> Result<Response<Full<Bytes>>, Box<dyn std::error::Error + Send + Sync>>
    where
        F: hyper::body::Body + Send + 'static,
        F::Data: Send,
        F::Error: std::error::Error + Send + Sync + 'static,
    {
        // we need to build full forward address
        // by combining forward host, forward port with
        // uri path of original address
        let full_forward_address = self
            .grpc_client
            .get_full_forward_address(req.uri(), &self.forward_address);
        let headers = req.headers().clone();

        // convert incoming request to bytes
        let body_as_bytes = req.collect().await?.to_bytes();
        let body = Full::from(body_as_bytes.clone());

        // send request and
        // get response from grpc service
        let original_res = self
            .grpc_client
            .forward_req(full_forward_address, headers, body)
            .await?;

        // forward response to grpc web client
        let forward_response = self.grpc_web_server.return_response(original_res).await?;

        Ok(forward_response)
    }
}
impl<'a> GrpcWebProxy<'a, GrpcClient, GrpcWebServer> {
    ///
    /// create GrpcWebProxy instance from Args
    /// # Examples
    /// ```
    /// let args = Args {
    ///     proxy_host: "localhost".to_string(),
    ///     proxy_port: 8080,
    /// };
    /// let grpc_web_proxy = GrpcWebProxy::new(&args);
    /// assert_eq!(grpc_web_proxy.proxy_address, "http://localhost:8080");
    /// ```
    pub fn from_args(args: &'a Args) -> Self {
        let proxy_address = format!("{}:{}", args.proxy_host, args.proxy_port);
        let forward_address = format!("http://{}:{}", args.forward_host, args.forward_port);
        let grpc_client = GrpcClient;
        let grpc_web_server = GrpcWebServer;
        GrpcWebProxy::<GrpcClient, GrpcWebServer> {
            proxy_address,
            forward_address,
            forward_host: &args.forward_host,
            forward_port: args.forward_port,
            grpc_client,
            grpc_web_server,
        }
    }
    #[cfg(test)]
    fn default() -> Self {
        let grpc_client = GrpcClient;
        let grpc_web_server = GrpcWebServer;
        GrpcWebProxy::<GrpcClient, GrpcWebServer> {
            proxy_address: "".to_string(),
            forward_address: "".to_string(),
            forward_host: "",
            forward_port: 0,
            grpc_client,
            grpc_web_server,
        }
    }
}

#[cfg(test)]
mod tests {

    use crate::grpcweb::{
        grpc_client::MockGrpcClientHandler, grpc_web_server::MockGrpcWebServerHandler,
    };

    use super::*;

    impl GrpcWebProxy<'_, MockGrpcClientHandler, MockGrpcWebServerHandler> {
        fn with_mock(
            grpc_client: MockGrpcClientHandler,
            grpc_web_server: MockGrpcWebServerHandler,
        ) -> Self {
            GrpcWebProxy {
                proxy_address: "".to_string(),
                forward_address: "".to_string(),
                forward_host: "",
                forward_port: 0,
                grpc_client,
                grpc_web_server,
            }
        }
    }
    #[test]
    fn test_get_full_forward_address() {
        // let mut mock_grpc_client_handler = MockGrpcClientHandler::new();
        // mock_grpc_client_handler
        //     .expect_get_full_forward_address()
        //     .returning(move |original_uri, forward_url| {
        //         let path = original_uri.path();
        //         format!("http://forward_address{}", path)
        //     });

        // let client = Client;
        // let grpc_web_proxy = super::GrpcWebProxy {
        //     proxy_address: "".to_string(),
        //     forward_address: "".to_string(),
        //     forward_host: "",
        //     forward_port: 0,
        //     client,
        // };
        let grpc_web_proxy = GrpcWebProxy::default();
        assert_eq!(
            grpc_web_proxy.grpc_client.get_full_forward_address(
                &"/test/path".parse().unwrap(),
                "http://forward_address:3000",
            ),
            "http://forward_address:3000/test/path"
        );
    }
    #[tokio::test]
    async fn test_forward_request() {
        let mut mock_grpc_client_handler = MockGrpcClientHandler::new();

        mock_grpc_client_handler
            .expect_get_full_forward_address()
            .returning(|_, _| "".to_string());
        mock_grpc_client_handler
            .expect_forward_req()
            .returning(|_, _, _| {
                Box::pin(async move {
                    let response = Response::builder()
                        .status(200)
                        .body(Full::from(Bytes::from("response body")))?;
                    Ok(response)
                })
            });
        let mut mock_grpc_web_server_handler = MockGrpcWebServerHandler::new();
        mock_grpc_web_server_handler
            .expect_return_response()
            .returning(|original_res| {
                let response = Response::builder()
                    .status(original_res.status())
                    .body(Full::from(Bytes::from("modified response body")))?;
                Ok(response)
            });
        let grpc_web_proxy = Arc::new(GrpcWebProxy::with_mock(
            mock_grpc_client_handler,
            mock_grpc_web_server_handler,
        ));
        let request = Request::builder()
            .uri("/test/path")
            .body(Full::<Bytes>::from("Hello"))
            .unwrap();
        let response = grpc_web_proxy.handle_grpc_web_request(request).await;
        assert!(response.is_ok());
    }
}
