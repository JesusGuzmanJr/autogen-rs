use anyhow::Result;
use autogen_rs::agent::{
    user::{Message, UserAgentBuilder},
    AgentBuilder,
};

type TokioSendError<T> = tokio::sync::mpsc::error::SendError<T>;

#[tokio::main]
async fn main() -> Result<()> {
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();

    let assistant = AgentBuilder::new()
        .with_name("1")
        .handler(move |message| {
            let tx = tx.clone();
            async move {
                // TODO: Open AI api call
                tx.send(message)?;
                Result::<_, TokioSendError<_>>::Ok(())
            }
        })
        .sender();

    let user_agent = UserAgentBuilder::new().with_name("user-agent").build();

    // start the conversation by sending a message to the user agent
    user_agent.send(Message {
        sender: assistant,
        content: "hello world".to_string(),
    })?;

    assert_eq!(rx.recv().await, Some("hello world".to_string()));
    Ok(())
}
