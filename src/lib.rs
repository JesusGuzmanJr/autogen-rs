//! Autogen-rs is a Rust library for building AI agents.

pub mod agent;

pub use agent::{user::UserAgent, Agent};

/// Init logger for unit tests.
#[cfg(test)]
#[ctor::ctor]
fn init() {
    use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

    tracing_subscriber::registry()
        .with(
            tracing_subscriber::fmt::layer()
                .without_time()
                .with_line_number(true),
        )
        .with(tracing_subscriber::EnvFilter::from_default_env())
        .init();
}
