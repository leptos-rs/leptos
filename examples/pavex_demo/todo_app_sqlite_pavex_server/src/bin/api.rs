use anyhow::Context;
use todo_app_sqlite_pavex_server::{
    configuration::load_configuration,
    telemetry::{get_subscriber, init_telemetry},
};
use todo_app_sqlite_pavex_server_sdk::{build_application_state, run};
use pavex::server::Server;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let subscriber = get_subscriber("todo_app_sqlite_pavex".into(), "info".into(), std::io::stdout);
    init_telemetry(subscriber)?;

    // We isolate all the server setup and launch logic in a separate function
    // in order to have a single choke point where we make sure to log fatal errors
    // that will cause the application to exit.
    if let Err(e) = _main().await {
        tracing::error!(
            error.msg = %e,
            error.error_chain = ?e,
            "The application is exiting due to an error"
        )
    }

    Ok(())
}

async fn _main() -> anyhow::Result<()> {
    // Load environment variables from a .env file, if it exists.
    let _ = dotenvy::dotenv();

    let config = load_configuration(None)?;
    let application_state = build_application_state()
        .await;

    let tcp_listener = config
        .server
        .listener()
        .await
        .context("Failed to bind the server TCP listener")?;
    let address = tcp_listener
        .local_addr()
        .context("The server TCP listener doesn't have a local socket address")?;
    let server_builder = Server::new().listen(tcp_listener);

    tracing::info!("Starting to listen for incoming requests at {}", address);
    run(server_builder, application_state).await;
    Ok(())
}
