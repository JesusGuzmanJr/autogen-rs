//! Example of a user agent that sends a message to an assistant
#![feature(lazy_cell)]

use {
    anyhow::Result,
    autogen_rs::agent::{
        assistant::AssistantBuilder, user::UserAgentBuilder, Actor, Message, Sender,
    },
    dashmap::DashMap,
    std::sync::LazyLock,
    tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt},
    uuid::Uuid,
};

static AGENTS: LazyLock<DashMap<Uuid, Sender<Box<Message>>>> = LazyLock::new(DashMap::new);

/// Invoking the example:
/// ```zsh
/// RUST_LOG=debug cargo run --example user_agent
/// ```     
#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::fmt::layer()
                .without_time()
                .with_line_number(true),
        )
        .with(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let assistant = AssistantBuilder::new().with_name("assistant").build();
    AGENTS.insert(assistant.id(), assistant.sender());

    let user_agent = UserAgentBuilder::new().with_name("user-agent").build();
    AGENTS.insert(user_agent.id(), user_agent.sender());

    // start the conversation by sending a message to the user agent
    user_agent.send(Message {
        sender: assistant.sender(),
        content: "What can I do for you?".to_string(),
    })?;

    tokio::time::sleep(std::time::Duration::from_secs(5)).await;

    tracing::debug!("<conversation ended>");
    Ok(())
}
