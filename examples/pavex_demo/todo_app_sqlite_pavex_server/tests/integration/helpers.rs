use todo_app_sqlite_pavex_server::configuration::{load_configuration, ApplicationProfile};
use todo_app_sqlite_pavex_server_sdk::{build_application_state, run};
use todo_app_sqlite_pavex::configuration::Config;
use pavex::server::Server;

pub struct TestApi {
    pub api_address: String,
    pub api_client: reqwest::Client,
}

impl TestApi {
    pub async fn spawn() -> Self {
        let config = Self::get_config();

        let application_state = build_application_state().await;

        let tcp_listener = config
            .server
            .listener()
            .await
            .expect("Failed to bind the server TCP listener");
        let address = tcp_listener
            .local_addr()
            .expect("The server TCP listener doesn't have a local socket address");
        let server_builder = Server::new().listen(tcp_listener);

        tokio::spawn(async move {
            run(server_builder, application_state).await
        });

        TestApi {
            api_address: format!("http://{}:{}", config.server.ip, address.port()),
            api_client: reqwest::Client::new(),
        }
    }

    fn get_config() -> Config {
        load_configuration(Some(ApplicationProfile::Test)).expect("Failed to load test configuration")
    }
}

/// Convenient methods for calling the API under test.
impl TestApi {
    pub async fn get_ping(&self) -> reqwest::Response
    {
        self.api_client
            .get(&format!("{}/api/ping", &self.api_address))
            .send()
            .await
            .expect("Failed to execute request.")
    }
}
