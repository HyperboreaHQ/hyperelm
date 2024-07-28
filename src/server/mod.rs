use hyperborealib::drivers::prelude::*;
use hyperborealib::rest_api::prelude::*;
use hyperborealib::port_forward::*;

mod params;
mod app;

pub use params::*;
pub use app::*;

#[cfg(feature = "server-basic-app")]
mod basic_app;

#[cfg(feature = "server-basic-app")]
pub use basic_app::*;

/// Start given server application in tokio async thread,
/// returning back an `Arc` containing original variant
/// of the server.
/// 
/// This method will freeze caller's thread while server app is running.
pub async fn run<T>(app: T) -> Result<(), T::Error>
where
    T: ServerApp + Send + Sync + 'static,
    T::Error: std::fmt::Debug
{
    let params = app.get_params();

    // Resolve server middleware and driver
    let middleware = app.get_middleware().await
        .expect("Failed to obtain application middleware");

    let driver = middleware.driver();

    // Create client middleware for traversal thread
    let traversal_client = ClientMiddleware::new(
        app.get_http_client().await?,
        driver.as_client()
    );

    // Open ports if given
    if !params.open_ports.is_empty() {
        tokio::spawn(async move {
            let duration = std::time::Duration::from_secs(3600);

            let upnp = UpnpPortForwarder::new();

            loop {
                for port in params.open_ports.iter().copied() {
                    if let Err(_err) = upnp.open(port, Protocol::TCP, duration).await {
                        #[cfg(feature = "tracing")]
                        tracing::error!("[server] Failed to open port {port} using UPnP forwarder: {_err}");
                    }
                }

                tokio::time::sleep(duration).await;
            }
        });
    }

    // Start the server
    tokio::spawn(async move {
        if let Err(_err) = middleware.serve(&params.local_address).await {
            #[cfg(feature = "tracing")]
            tracing::error!("[server] {_err}");
        }
    });

    loop {
        // Index bootstrap servers
        #[cfg(feature = "tracing")]
        tracing::debug!("[server] Indexing bootstrap addresses");

        for address in &params.bootstrap {
            if let Ok(server) = traversal_client.get_info(&address).await {
                let _result = driver.router().index_server(Server::new(
                    server.public_key,
                    address
                )).await;

                #[cfg(feature = "tracing")]
                if let Err(err) = _result {
                    tracing::error!("[server] Failed to index bootstrap server: {err}");
                }
            }
        }

        // Traverse network
        #[cfg(feature = "tracing")]
        tracing::debug!("[server] Traversing network");

        driver.traversal().traverse(
            traversal_client.http_client_ref().clone(),
            &driver
        ).await;

        // Announce servers about ourselves
        if params.announce {
            // TODO
        }

        // Wait before repeating
        tokio::time::sleep(params.traverse_delay).await;
    }
}
