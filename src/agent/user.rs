//! A proxy agent for the user. Every time the agent receives a message, it asks
//! the user for input and sends the input back to the sender of the message.

use {
    super::{Actor, Message, Sender},
    crate::Agent,
    uuid::Uuid,
};

/// Errors that can occur when sending a message to a user agent.
#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("unable to send message: {0:?}")]
    SendError(#[from] crate::agent::SendError<Box<Message>>),

    #[error("failed to read user input: {0:?}")]
    ReadUserInputError(#[from] std::io::Error),
}

const USER_INPUT_PREFIX: &str = ">>>";

/// A user proxy agent.
///
/// Usage:
/// ```
/// # use autogen_rs::agent::user::UserAgentBuilder;
/// # tokio_test::block_on(async {
/// let user_agent = UserAgentBuilder::new().with_name("user-agent").build();
/// # anyhow::Ok(())
/// # });
/// ````
#[derive(Debug)]
pub struct UserAgent {
    pub agent: Agent<Box<Message>, Error>,
}

impl UserAgent {
    /// Create a new user agent.
    pub fn spawn(id: Uuid, name: Option<String>) -> Self {
        let prompt_id = name.clone().unwrap_or_else(|| id.to_string());
        let agent = Agent::<Box<Message>, _>::spawn(id, name, move |sender, message| {
            let prompt_id = prompt_id.clone();
            async move {
                println!("{prompt_id} {USER_INPUT_PREFIX} {}", message.content);
                let mut input = String::new();
                std::io::stdin().read_line(&mut input)?;

                // reply to message sender with the user input
                message.sender.send(Box::new(Message {
                    sender,
                    content: input.trim().to_string(),
                }))?;
                Ok(())
            }
        });

        Self { agent }
    }

    /// Returns a sender that can be used to send messages to the user agent.
    pub fn sender(&self) -> Sender<Box<Message>> {
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

impl Actor for UserAgent {
    type Error = super::SendError<Box<Message>>;
    type Message = Message;

    fn id(&self) -> Uuid {
        self.agent.id
    }

    /// Returns the user agent's name
    fn name(&self) -> Option<&str> {
        self.agent.name.as_deref()
    }

    async fn terminate(self) {
        self.agent.terminate().await;
    }

    fn abort(self) {
        self.agent.abort()
    }

    fn send(&self, message: Self::Message) -> Result<(), Self::Error> {
        self.agent.send(Box::new(message))?;
        Ok(())
    }
}
