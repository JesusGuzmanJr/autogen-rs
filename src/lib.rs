pub mod agent;

pub use agent::user::UserAgent;
pub use agent::Agent;

#[cfg(test)]
#[ctor::ctor]
fn init() {
    use tracing_subscriber::layer::SubscriberExt;
    use tracing_subscriber::util::SubscriberInitExt;

    tracing_subscriber::registry()
        .with(
            tracing_subscriber::fmt::layer()
                .without_time()
                .with_line_number(true),
        )
        .with(tracing_subscriber::EnvFilter::from_default_env())
        .init();
}
