use hyperborealib::drivers::prelude::*;
use hyperborealib::http::*;

use super::*;

/// Implement most of the `ServerApp` types and methods
/// using default values.
/// 
/// ```rust
/// use hyperelm::prelude::*;
/// 
/// use hyperborealib::crypto::SecretKey;
/// 
/// struct MyServerApp;
/// 
/// impl BasicServerApp for MyServerApp {
///     fn get_params(&self) -> ServerAppParams {
///         ServerAppParams {
///             secret_key: SecretKey::random(),
///             local_address: String::from("127.0.0.1:8001"),
///             remote_address: String::from("127.0.0.1:8001"),
///             bootstrap: vec![],
///             announce: false,
///             traverse_delay: std::time::Duration::from_secs(60 * 10)
///         }
///     }
/// }
/// ```
pub trait BasicServerApp {
    fn get_params(&self) -> ServerAppParams;
}

#[async_trait::async_trait]
impl<T> ServerApp for T where T: BasicServerApp + Send + Sync {
    type Router = GlobalTableRouter;
    type Traversal = BfsRecursionTraversal;
    type MessagesInbox = StoredQueueMessagesInbox;

    type HttpClient = ReqwestHttpClient;
    type HttpServer = AxumHttpServer;

    type Error = std::io::Error;

    async fn get_router(&self) -> Result<Self::Router, Self::Error> {
        let params = self.get_params();

        let router = GlobalTableRouter::new(params.backend_folder.join("router")).await?;

        Ok(router)
    }

    #[inline]
    async fn get_traversal(&self) -> Result<Self::Traversal, Self::Error> {
        Ok(BfsRecursionTraversal)
    }

    #[inline]
    async fn get_messages_inbox(&self) -> Result<Self::MessagesInbox, Self::Error> {
        let params = self.get_params();

        let inbox = StoredQueueMessagesInbox::new(params.backend_folder.join("inbox")).await?;

        Ok(inbox)
    }

    #[inline]
    async fn get_http_client(&self) -> Result<Self::HttpClient, Self::Error> {
        Ok(ReqwestHttpClient::default())
    }

    #[inline]
    async fn get_http_server(&self) -> Result<Self::HttpServer, Self::Error> {
        Ok(AxumHttpServer::default())
    }

    #[inline]
    fn get_params(&self) -> ServerAppParams {
        T::get_params(self)
    }
}
