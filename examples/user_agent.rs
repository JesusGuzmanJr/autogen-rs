//! Example of a user agent that sends a message to an assistant

use {
    anyhow::Result,
    autogen_rs::agent::{
        user::{Message, UserAgentBuilder},
        AgentBuilder,
    },
    tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt},
};

type TokioSendError<T> = tokio::sync::mpsc::error::SendError<T>;

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

    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();

    let assistant_name = "assistant";
    let assistant = AgentBuilder::new()
        .with_name(assistant_name)
        .handler(move |message| {
            let tx = tx.clone();
            async move {
                // just echo the message back
                tracing::debug!(
                    name = assistant_name,
                    message,
                    "received message; normally this message would be sent to OpenAI but for this example we just echo it back"
                );
                let response = message;
                tx.send(response)?;

                Result::<_, TokioSendError<_>>::Ok(())
            }
        })
        .sender();

    let user_agent = UserAgentBuilder::new().with_name("user-agent").build();

    // start the conversation by sending a message to the user agent
    user_agent.send(Message {
        // the LLM assistant is the originator of the message
        sender: assistant,
        content: "What can I do for you?".to_string(),
    })?;

    if let Some(response) = rx.recv().await {
        tracing::debug!(response, "received response");
    }

    tracing::debug!("<conversation ended>");
    Ok(())
}
