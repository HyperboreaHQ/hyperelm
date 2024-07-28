use hyperborealib::http::*;
use hyperborealib::rest_api::prelude::*;
use hyperborealib::drivers::prelude::*;

use super::ServerAppParams;

#[async_trait::async_trait]
pub trait ServerApp {
    type Router: Router + Send + Sync + 'static;
    type Traversal: Traversal + Send + Sync + 'static;
    type MessagesInbox: MessagesInbox + Send + Sync + 'static;

    type HttpClient: HttpClient + Send + Sync + 'static;
    type HttpServer: HttpServer + Send + Sync + 'static;

    type Error: Send;

    async fn get_router(&self) -> Result<Self::Router, Self::Error>;
    async fn get_traversal(&self) -> Result<Self::Traversal, Self::Error>;
    async fn get_messages_inbox(&self) -> Result<Self::MessagesInbox, Self::Error>;

    async fn get_http_client(&self) -> Result<Self::HttpClient, Self::Error>;
    async fn get_http_server(&self) -> Result<Self::HttpServer, Self::Error>;

    fn get_params(&self) -> ServerAppParams;

    #[allow(clippy::type_complexity)]
    async fn get_driver(&self) -> Result<ServerDriver<
        Self::Router,
        Self::Traversal,
        Self::MessagesInbox
    >, Self::Error> {
        let params = self.get_params();

        Ok(ServerDriver::new(
            self.get_router().await?,
            self.get_traversal().await?,
            self.get_messages_inbox().await?,
            ServerParams {
                secret_key: params.secret_key.clone(),
                address: params.remote_address.clone()
            }
        ))
    }

    #[allow(clippy::type_complexity)]
    async fn get_middleware(&self) -> Result<ServerMiddleware<
        Self::HttpClient,
        Self::HttpServer,
        Self::Router,
        Self::Traversal,
        Self::MessagesInbox
    >, Self::Error> {
        Ok(ServerMiddleware::new(
            self.get_http_client().await?,
            self.get_http_server().await?,
            self.get_driver().await?
        ).await)
    }
}
