//! A proxy agent for the user.

use super::Sender;
use crate::Agent;
use uuid::Uuid;

/// Errors that can occur when sending a message to a user agent.
#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("unable to send message to terminated user agent: {0:?}")]
    SendError(#[from] crate::agent::SendError<Message>),

    #[error("failed to read user input: {0:?}")]
    ReadUserInputError(#[from] std::io::Error),

    #[error("unable to reply to message: {0:?}")]
    ReplySendError(#[from] crate::agent::SendError<String>),
}

const USER_INPUT_PREFIX: &str = ">>>";

/// Messages that can be sent to a user agent.
#[derive(Debug)]
pub struct Message {
    /// The sender to reply to.
    pub sender: Sender<String>,

    /// The content of the to prompt the user.
    pub content: String,
}

/// A user agent.
#[derive(Debug)]
pub struct UserAgent {
    agent: Agent<Message, Error>,
}

impl UserAgent {
    /// Create a new user agent.
    pub fn spawn(id: Uuid, name: Option<String>) -> Self {
        let prompt_id = name.clone().unwrap_or_else(|| id.to_string());
        let agent = Agent::<Message, _>::spawn(id, name, move |message| {
            let prompt_id = prompt_id.clone();
            async move {
                println!("{prompt_id} {USER_INPUT_PREFIX} {}", message.content);
                let mut input = String::new();
                std::io::stdin().read_line(&mut input)?;

                // reply to message sender with the user input
                message.sender.send(input.trim().to_string())?;
                Ok(())
            }
        });

        Self { agent }
    }

    /// Send a message to the user agent.
    pub fn send(&self, message: Message) -> Result<(), Error> {
        self.agent.send(message)?;
        Ok(())
    }

    /// Terminates the agent by closing its message channel and waiting for it
    /// to finish processing remaining messages. Consumes the agent since it
    /// can no longer process messages.
    pub async fn terminate(self) {
        self.agent.terminate().await;
    }

    /// Aborts the agent's event loop immediately without waiting for it to finish.
    pub fn abort(self) {
        self.agent.abort()
    }

    /// Returns a sender that can be used to send messages to the user agent.
    pub fn sender(&self) -> Sender<Message> {
        Sender(self.agent.sender.clone())
    }
}

#[derive(Debug, Default)]
pub struct UserAgentBuilder {
    /// Unique identifier for the user agent.
    pub id: Option<Uuid>,

    /// A user-friendly name for the user agent.
    pub name: Option<String>,
}

impl UserAgentBuilder {
    /// Create a new agent builder.
    pub fn new() -> Self {
        Default::default()
    }

    /// Set the id of the agent.
    pub fn with_id(mut self, id: Uuid) -> Self {
        self.id = Some(id);
        self
    }

    /// Set the name of the agent.
    pub fn with_name(mut self, name: impl ToString) -> Self {
        self.name = Some(name.to_string());
        self
    }

    /// Builds the user agent.
    pub fn build(self) -> UserAgent {
        UserAgent::spawn(self.id.unwrap_or_else(Uuid::new_v4), self.name)
    }
}
