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

const USER_INPUT_PREFIX: &str = ">>> ";

/// Messages that can be sent to a user agent.
#[derive(Debug)]
pub struct Message {
    /// The sender to reply to.
    sender: Sender<String>,

    /// The content of the to prompt the user.
    content: String,
}

/// A user agent.
#[derive(Debug)]
pub struct UserAgent {
    agent: Agent<Message, Error>,
}

impl UserAgent {
    /// Create a new user agent.
    pub fn spawn(id: Uuid, name: Option<String>) -> Self {
        let agent = Agent::<Message, _>::spawn(id, name, |message| async move {
            println!("{USER_INPUT_PREFIX} {}", message.content);
            let mut input = String::new();
            std::io::stdin().read_line(&mut input)?;

            // reply to message sender with the user input
            message.sender.send(input)?;
            Ok(())
        });

        Self { agent }
    }

    /// Send a message to the user agent.
    pub fn send(&self, message: Message) -> Result<(), Error> {
        self.agent.send(message)?;
        Ok(())
    }
}
