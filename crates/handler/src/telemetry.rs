use anyhow::Error;
use opentelemetry::trace::TracerProvider;
use opentelemetry_otlp::WithExportConfig;
use opentelemetry_sdk::runtime;
use opentelemetry_sdk::trace::BatchConfigBuilder;
use opentelemetry_sdk::trace::BatchSpanProcessor;
use opentelemetry_sdk::trace::Config;
use opentelemetry_sdk::Resource;
use tracing_subscriber::prelude::*;
use tracing_subscriber::EnvFilter;

pub async fn init_azure_telemetry() -> Result<(), Error> {
    let app_insights_connection_string = std::env::var("APPLICATIONINSIGHTS_CONNECTION_STRING")?;

    // Set up Azure Monitor exporter
    let azure_monitor_exporter =
        opentelemetry_application_insights::Exporter::new_from_connection_string(
            app_insights_connection_string,
            reqwest::Client::new(),
        )
        .expect("valid connection string");

    // Set up OTLP exporter for Grafana
    let otlp_exporter = opentelemetry_otlp::new_exporter()
        .http()
        .with_endpoint("YOUR_GRAFANA_ENDPOINT")
        .build_span_exporter()?;

    // Create a BatchSpanProcessor for each exporter
    let azure_monitor_processor =
        BatchSpanProcessor::builder(azure_monitor_exporter, runtime::Tokio)
            .with_batch_config(
                BatchConfigBuilder::default()
                    .with_max_queue_size(4096)
                    .build(),
            )
            .build();
    let otlp_processor = BatchSpanProcessor::builder(otlp_exporter, runtime::Tokio)
        .with_batch_config(
            BatchConfigBuilder::default()
                .with_max_queue_size(4096)
                .build(),
        )
        .build();

    // Build the tracer provider
    let provider = opentelemetry_sdk::trace::TracerProvider::builder()
        .with_config(Config::default().with_resource(Resource::new(vec![
            opentelemetry::KeyValue::new("service.name", "glitch-guardian"),
        ])))
        .with_span_processor(azure_monitor_processor)
        .with_span_processor(otlp_processor)
        .build();

    let tracer = provider.tracer("thing");

    // Create a tracing layer with the configured tracer
    let telemetry = tracing_opentelemetry::layer().with_tracer(tracer);

    // Use the tracing subscriber `Registry`, or any other subscriber
    // that impls `LookupSpan`
    tracing_subscriber::registry()
        .with(telemetry)
        .with(EnvFilter::from_default_env())
        .init();

    Ok(())
}

pub fn init_local_telemetry() -> Result<(), Error> {
    // Set up console subscriber for local development
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    Ok(())
}
