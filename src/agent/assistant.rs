//! The OpenAI backed agent. It is a wrapper around the OpenAI API.
//!
//! *Under development*

use {
    super::{Actor, Message, Sender},
    crate::Agent,
    uuid::Uuid,
};

/// Errors that can occur when sending a message to a assistant.
#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("unable to send message: {0:?}")]
    SendError(#[from] crate::agent::SendError<Box<Message>>),

    #[error("Io error: {0:?}")]
    IoError(#[from] std::io::Error),
}

/// An LLM assistant.
///
/// Usage:
/// ```
/// # use autogen_rs::agent::assistant::AssistantBuilder;
/// # tokio_test::block_on(async {
/// let assistant = AssistantBuilder::new().with_name("assistant").build();
/// # anyhow::Ok(())
/// # });
/// ````
#[derive(Debug)]
pub struct Assistant {
    pub agent: Agent<Box<Message>, Error>,
}

impl Assistant {
    /// Create a new assistant.
    pub fn spawn(id: Uuid, name: Option<String>) -> Self {
        let agent = Agent::<Box<Message>, _>::spawn(id, name, move |sender, message| {
            async move {
                tracing::trace!(%id,  message = &message.content, "received message; pretending to call OpenAI API");
                // TODO: call OpenAI API
                // for now just echo the message back

                message.sender.clone().send(Box::new(Message {
                    sender,
                    content: message.content,
                }))?;
                Ok(())
            }
        });

        Self { agent }
    }

    /// Send a message to the assistant.
    pub fn send(&self, message: Message) -> Result<(), Error> {
        self.agent.send(Box::new(message))?;
        Ok(())
    }

    /// Terminates the agent by closing its message channel and waiting for it
    /// to finish processing remaining messages. Consumes the agent since it
    /// can no longer process messages.
    pub async fn terminate(self) {
        self.agent.terminate().await;
    }

    /// Aborts the agent's event loop immediately without waiting for it to
    /// finish.
    pub fn abort(self) {
        self.agent.abort()
    }

    /// Returns a sender that can be used to send messages to the assistant.
    pub fn sender(&self) -> Sender<Box<Message>> {
        Sender(self.agent.sender.clone())
    }
}

#[derive(Debug, Default)]
pub struct AssistantBuilder {
    /// Unique identifier for the assistant.
    pub id: Option<Uuid>,

    /// A user-friendly name for the assistant.
    pub name: Option<String>,
}

impl AssistantBuilder {
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

    /// Builds the assistant.
    pub fn build(self) -> Assistant {
        Assistant::spawn(self.id.unwrap_or_else(Uuid::new_v4), self.name)
    }
}

impl Actor for Assistant {
    type Error = super::SendError<Box<Message>>;
    type Message = Message;

    fn id(&self) -> Uuid {
        self.agent.id
    }

    /// Returns the assistant's name
    fn name(&self) -> Option<&str> {
        self.agent.name.as_deref()
    }

    fn send(&self, message: Self::Message) -> Result<(), Self::Error> {
        self.agent.send(Box::new(message))?;
        Ok(())
    }
}
