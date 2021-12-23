use crate::{Error, Result};

///  Fetch an opentelemetry::trace::TraceId as hex through the full tracing stack
pub fn get_trace_id() -> String {
    use opentelemetry::trace::TraceContextExt as _; // opentelemetry::Context -> opentelemetry::trace::Span
    use tracing_opentelemetry::OpenTelemetrySpanExt as _; // tracing::Span to opentelemetry::Context

    tracing::Span::current()
        .context()
        .span()
        .span_context()
        .trace_id()
        .to_hex()
}

#[cfg(feature = "telemetry")]
pub async fn init_tracer() -> opentelemetry::sdk::trace::Tracer {
    let otlp_endpoint =
        std::env::var("OPENTELEMETRY_ENDPOINT_URL").expect("Need a otel tracing collector configured");

    let channel = tonic::transport::Channel::from_shared(otlp_endpoint)
        .unwrap()
        .connect()
        .await
        .unwrap();

    opentelemetry_otlp::new_pipeline()
        .tracing()
        .with_exporter(opentelemetry_otlp::new_exporter().tonic().with_channel(channel))
        .with_trace_config(opentelemetry::sdk::trace::config().with_resource(
            opentelemetry::sdk::Resource::new(vec![opentelemetry::KeyValue::new(
                "service.name",
                "engula-operator",
            )]),
        ))
        .install_batch(opentelemetry::runtime::Tokio)
        .unwrap()
}
