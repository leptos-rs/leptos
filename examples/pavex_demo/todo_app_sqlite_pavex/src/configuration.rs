use pavex::server::IncomingStream;
use serde_aux::field_attributes::deserialize_number_from_string;
use std::net::SocketAddr;

#[derive(serde::Deserialize)]
/// The top-level configuration, holding all the values required
/// to configure the entire application.
pub struct Config {
    pub server: ServerConfig,
}

#[derive(serde::Deserialize, Clone)]
/// Configuration for the HTTP server used to expose our API
/// to users.
pub struct ServerConfig {
    /// The port that the server must listen on.
    #[serde(deserialize_with = "deserialize_number_from_string")]
    pub port: u16,
    /// The network interface that the server must be bound to.
    ///
    /// E.g. `0.0.0.0` for listening to incoming requests from
    /// all sources.
    pub ip: std::net::IpAddr,
}

impl ServerConfig {
    /// Bind a TCP listener according to the specified parameters.
    pub async fn listener(&self) -> Result<IncomingStream, std::io::Error> {
        let addr = SocketAddr::new(self.ip, self.port);
        IncomingStream::bind(addr).await
    }
}