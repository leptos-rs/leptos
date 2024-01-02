use anyhow::Context;
use tracing::subscriber::set_global_default;
use tracing::Subscriber;
use tracing_bunyan_formatter::{BunyanFormattingLayer, JsonStorageLayer};
use tracing_subscriber::fmt::MakeWriter;
use tracing_subscriber::{layer::SubscriberExt, EnvFilter, Registry};

/// Perform all the required setup steps for our telemetry:
///
/// - Register a subscriber as global default to process span data
/// - Register a panic hook to capture any panic and record its details
///
/// It should only be called once!
pub fn init_telemetry(subscriber: impl Subscriber + Sync + Send) -> Result<(), anyhow::Error> {
    std::panic::set_hook(Box::new(tracing_panic::panic_hook));
    set_global_default(subscriber).context("Failed to set a `tracing` global subscriber")
}

/// Compose multiple layers into a `tracing`'s subscriber.
///
/// # Implementation Notes
///
/// We are using `impl Subscriber` as return type to avoid having to spell out the actual
/// type of the returned subscriber, which is indeed quite complex.
pub fn get_subscriber<Sink>(
    application_name: String,
    default_env_filter: String,
    sink: Sink,
) -> impl Subscriber + Sync + Send
    where
        Sink: for<'a> MakeWriter<'a> + Send + Sync + 'static,
{
    let env_filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(default_env_filter));
    let formatting_layer = BunyanFormattingLayer::new(application_name, sink);
    Registry::default()
        .with(env_filter)
        .with(JsonStorageLayer)
        .with(formatting_layer)
}