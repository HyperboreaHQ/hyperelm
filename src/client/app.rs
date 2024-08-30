use serde_json::{json, Value as Json};

use hyperborealib::exports::tokio;

use hyperborealib::crypto::prelude::*;
use hyperborealib::rest_api::prelude::*;

use hyperborealib::http::HttpClient;

use super::*;

#[derive(Debug, thiserror::Error)]
pub enum ClientAppError<E: Send + Sync> {
    #[error(transparent)]
    SerdeJsonError(#[from] serde_json::Error),

    #[error(transparent)]
    AsJsonError(#[from] AsJsonError),

    #[error(transparent)]
    MiddlewareError(#[from] MiddlewareError),

    #[error(transparent)]
    MessagesError(#[from] MessagesError),

    #[error(transparent)]
    Custom(E)
}

#[async_trait::async_trait]
pub trait ClientApp {
    /// Request which can be received from other clients.
    type InputRequest: AsJson + Send;

    /// Response which will be sent after processing the `InputRequest`.
    type InputResponse: AsJson + Send;

    /// Message which can be received from other clients.
    type InputMessage: AsJson + Send;

    /// Request which can be sent to other clients.
    type OutputRequest: AsJson + Send;

    /// Response which can be received from other clients.
    type OutputResponse: AsJson + Send;

    /// Message which can be sent to other clients.
    type OutputMessage: AsJson + Send;

    type HttpClient: HttpClient;
    type State;
    type Error: Send + Sync;

    /// Get params of the client app.
    fn get_params(&self) -> &ClientAppParams;

    /// Get client app HTTP REST API client middleware.
    fn get_middleware(&self) -> &ClientMiddleware<Self::HttpClient>;

    /// Get client app's state.
    fn get_state(&self) -> Arc<Self::State>;

    /// Get connected client middleware.
    ///
    /// It is highly recommended to re-implement this method
    /// to reuse some local cache with some TTL.
    async fn get_connected_middleware(&self) -> Result<ConnectedClientMiddleware<Self::HttpClient>, ClientAppError<Self::Error>> {
        let params = self.get_params();

        Ok(self.get_middleware().connect_to(
            &params.server_address,
            params.server_public.clone()
        ).await?)
    }

    /// Perform client searching in the network.
    async fn lookup(&self, public_key: PublicKey, client_type: Option<ClientType>) -> Result<Option<ClientEndpoint>, ClientAppError<Self::Error>> {
        let result = self.get_connected_middleware().await?
            .lookup(public_key, client_type).await?
            .map(|(client, server, _)| {
                ClientEndpoint {
                    server_address: server.address,
                    client_public: client.public_key
                }
            });

        Ok(result)
    }

    /// Send request to given endpoint.
    async fn request(&self, endpoint: ClientEndpoint, request: Self::OutputRequest) -> Result<Self::OutputResponse, ClientAppError<Self::Error>> {
        let params = self.get_params();
        let middleware = self.get_connected_middleware().await?;

        // Prepare request
        let request_id = safe_random_u64();

        let request = json!({
            "id": request_id,
            "request": request.to_json()?
        });

        // Send request
        let request = Message::create(
            &params.client_secret,
            &endpoint.client_public,
            serde_json::to_vec(&request)?,
            params.encoding,
            params.compression_level
        )?;

        middleware.send(
            endpoint.server_address,
            endpoint.client_public,
            &params.channel,
            request
        ).await?;

        // Receive response
        loop {
            let (messages, _) = middleware.poll(
                format!("{}@{request_id}", params.channel),
                Some(1)
            ).await?;

            // If there's an incoming message
            if let Some(message) = messages.first() {
                // Decode the message and verify its validity
                let response = message.message.read(
                    &params.client_secret,
                    &message.sender.client.public_key
                )?;

                // Deserialize it and return
                let response = serde_json::from_slice::<Json>(&response)?;

                let response = Self::OutputResponse::from_json(&response)?;

                return Ok(response);
            }

            // Sleep otherwise and try again
            tokio::time::sleep(params.delay).await;
        }
    }

    /// Send message to given endpoint.
    async fn send(&self, endpoint: ClientEndpoint, message: Self::OutputMessage) -> Result<(), ClientAppError<Self::Error>> {
        let params = self.get_params();
        let middleware = self.get_connected_middleware().await?;

        // Prepare message
        let message = json!({
            "message": message.to_json()?
        });

        let message = Message::create(
            &params.client_secret,
            &endpoint.client_public,
            serde_json::to_vec(&message)?,
            params.encoding,
            params.compression_level
        )?;

        // Send message
        middleware.send(
            endpoint.server_address,
            endpoint.client_public,
            &params.channel,
            message
        ).await?;

        Ok(())
    }

    /// Try to poll a message from the connected hyperborea server.
    async fn poll_message(&self) -> Result<Option<MessageInfo>, ClientAppError<Self::Error>> {
        let params = self.get_params();

        // Implementers should poll all available messages and store them
        // in a queue, polling from it and fulfilling it when it becomes empty.
        let (mut messages, _) = self.get_connected_middleware().await?
            .poll(&params.channel, Some(1)).await?;

        Ok(messages.pop())
    }

    /// Receive and process incoming messages.
    async fn update(&self) -> Result<(), ClientAppError<Self::Error>> {
        if let Some(message) = self.poll_message().await? {
            let params = self.get_params();

            // Decode the message and verify its validity
            let content = message.message.read(
                &params.client_secret,
                &message.sender.client.public_key
            )?;

            // Deserialize it and process
            let content = serde_json::from_slice::<Json>(&content)?;

            // Handle request
            if let Some(request) = content.get("request") {
                if let Some(request_id) = content.get("id").and_then(Json::as_u64) {
                    // Deserialize request
                    let request = Self::InputRequest::from_json(request)?;

                    // Process request
                    let response = self.handle_request(request, message.clone()).await?;

                    // Send response
                    let response = Message::create(
                        &params.client_secret,
                        &message.sender.client.public_key,
                        serde_json::to_vec(&response.to_json()?)?,
                        params.encoding,
                        params.compression_level
                    )?;

                    self.get_connected_middleware().await?.send(
                        &message.sender.server.address,
                        message.sender.client.public_key,
                        format!("{}@{request_id}", params.channel),
                        response
                    ).await?;
                }
            }

            // Handle message
            else if let Some(request) = content.get("message") {
                let request = Self::InputMessage::from_json(request)?;

                // Process message
                self.handle_message(request, message).await?;
            }
        }

        Ok(())
    }

    /// Handle incoming request.
    async fn handle_request(&self, request: Self::InputRequest, info: MessageInfo) -> Result<Self::InputResponse, ClientAppError<Self::Error>>;

    /// Handle incoming message.
    async fn handle_message(&self, message: Self::InputMessage, info: MessageInfo) -> Result<(), ClientAppError<Self::Error>>;
}
