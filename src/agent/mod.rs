//! Tools for creating tokio-based agents.

use std::future::Future;

mod actor;

pub use actor::Actor;
pub mod assistant;
pub mod user;

use {
    std::{fmt::Debug, time::Duration},
    tokio::{sync::mpsc::UnboundedSender, task::JoinHandle},
    uuid::Uuid,
};

/// Error returned when trying to send a message to an agent that has been
/// terminated. Returns the message that couldn't be sent.
#[derive(thiserror::Error, Debug, PartialEq, Eq, Clone, Copy)]
#[error("unable to send message to terminated agent: {0:?}")]
pub struct SendError<M>(pub M);

/// The AGENT_GRACE_PERIOD_SECONDS environment variable can be used to override
/// the default grace period.
const GRACE_PERIOD_ENV_VAR: &str = "AGENT_GRACE_PERIOD_SECONDS";

/// The amount of time to wait for an agent to terminate.
const DEFAULT_GRACE_PERIOD: Duration = Duration::from_secs(3);

/// Messages that can be sent to an actor;.
#[derive(Debug, Clone)]
pub struct Message {
    /// The sender to reply to.
    pub sender: Sender<Box<Message>>,

    /// The content of the to prompt the user.
    pub content: String,
}

/// A channel to send messages to an agent.
#[derive(Debug, Clone)]
pub struct Sender<M>(UnboundedSender<M>);

impl<M> Sender<M> {
    /// Send a message to the agent.
    pub fn send(&self, message: M) -> Result<(), SendError<M>> {
        // map the tokio SendError to our own SendError
        self.0.send(message).map_err(|m| SendError(m.0))
    }
}

/// A handle to an agent.
#[derive(Debug)]
pub struct Agent<M, E> {
    /// Unique identifier for the agent.
    pub id: Uuid,

    /// A user-friendly name for the agent.
    pub name: Option<String>,

    /// A channel to send messages to the agent.
    sender: UnboundedSender<M>,

    /// A handle to the agent's event loop.
    handle: JoinHandle<Result<(), E>>,
}

impl<M, E> Agent<M, E>
where
    M: Debug + Send + 'static,
    E: std::error::Error + Send + Sync + 'static,
{
    /// Create a new agent.
    pub fn spawn<H, R>(id: Uuid, name: Option<String>, handler: H) -> Self
    where
        H: Fn(Sender<M>, M) -> R + Send + Sync + 'static,
        R: Future<Output = Result<(), E>> + Send + 'static,
    {
        let (sender, mut receiver) = tokio::sync::mpsc::unbounded_channel();

        let handle = {
            let name = name.clone();
            let sender = sender.clone();
            tokio::spawn(async move {
                tracing::trace!(name, %id, "starting",);

                while let Some(message) = receiver.recv().await {
                    tracing::trace!(name, %id, ?message, "received message");
                    handler(Sender(sender.clone()), message).await?;
                }

                tracing::trace!(name, %id, "stopping");
                Ok(())
            })
        };

        Self {
            id,
            name,
            sender,
            handle,
        }
    }

    /// Terminates the agent by closing its message channel and waiting for it
    /// to finish processing remaining messages. Consumes the agent since it
    /// can no longer process messages.
    pub async fn terminate(self) {
        drop(self.sender); // drop the sender to signal the agent to stop.
        tokio::time::sleep(
            std::env::var(GRACE_PERIOD_ENV_VAR)
                .ok()
                .and_then(|s| s.parse().ok())
                .map(Duration::from_secs)
                .unwrap_or(DEFAULT_GRACE_PERIOD),
        )
        .await;
        self.handle.abort();
        tracing::trace!(name = self.name, id = %self.id, "stopped (gracefully terminated)");
    }

    /// Aborts the agent's event loop immediately without waiting for it to
    /// finish.
    pub fn abort(self) {
        self.handle.abort();
        tracing::trace!(name = self.name, id = %self.id, "stopped (aborted)");
    }

    /// Send a message to the agent.
    pub fn send(&self, message: M) -> Result<(), SendError<M>> {
        self.sender.send(message).map_err(|e| SendError(e.0))
    }

    /// Returns a sender that can be used to send messages to the agent.
    pub fn sender(&self) -> Sender<M> {
        Sender(self.sender.clone())
    }
}

#[cfg(test)]
mod tests {
    use {super::*, anyhow::Result};

    type TokioSendError<T> = tokio::sync::mpsc::error::SendError<T>;
    type Error<T> = SendError<T>;

    #[tokio::test]
    async fn test_actor_processes_message() -> Result<()> {
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();

        let agent = Agent::spawn(
            Uuid::new_v4(),
            Some("1".to_string()),
            move |_sender, message| {
                let tx = tx.clone();
                async move {
                    tx.send(message)?;
                    Result::<_, TokioSendError<_>>::Ok(())
                }
            },
        );

        let message = "hello world";
        agent.send(message)?;
        assert_eq!(rx.recv().await, Some(message));
        Ok(())
    }

    #[tokio::test]
    async fn test_multiple_agents() -> Result<()> {
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();

        let agent_1 = Agent::spawn(
            Uuid::new_v4(),
            Some("1".to_string()),
            move |_sender, message| {
                let tx = tx.clone();
                async move {
                    tx.send(message)?;
                    Result::<_, TokioSendError<_>>::Ok(())
                }
            },
        )
        .sender();

        let agent_2 = Agent::spawn(
            Uuid::new_v4(),
            Some("2".to_string()),
            move |_sender, message| {
                let agent_1 = agent_1.clone();
                async move {
                    agent_1.send(message)?;
                    Result::<_, Error<&'static str>>::Ok(())
                }
            },
        );

        let message = "hello world";
        agent_2.send(message)?;
        assert_eq!(rx.recv().await, Some(message));
        Ok(())
    }

    #[tokio::test]
    async fn test_terminate() -> Result<()> {
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();

        let agent = Agent::spawn(
            Uuid::new_v4(),
            Some("1".to_string()),
            move |_sender, message| {
                let tx = tx.clone();
                async move {
                    tx.send(message)?;
                    Result::<_, TokioSendError<_>>::Ok(())
                }
            },
        );

        let message = "hello world";
        agent.send(message)?;
        agent.terminate().await;

        assert_eq!(
            rx.recv().await,
            Some(message),
            "testing that the message gets processed before the agent terminates"
        );

        Ok(())
    }

    #[tokio::test]
    async fn test_terminate_timeout() -> Result<()> {
        std::env::set_var(GRACE_PERIOD_ENV_VAR, "1");
        let grace_period = Duration::from_millis(1200);

        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();

        let agent = Agent::spawn(
            Uuid::new_v4(),
            Some("1".to_string()),
            move |_sender, message| {
                let tx = tx.clone();
                async move {
                    tokio::time::sleep(grace_period).await;
                    tx.send(message)?;
                    Result::<_, TokioSendError<_>>::Ok(())
                }
            },
        );

        let message = "hello world";
        agent.send(message)?;
        agent.terminate().await;

        assert_eq!(
            rx.recv().await,
            None,
            "testing that the message doesn't get placed on the channel because the agent took too long"
        );

        Ok(())
    }

    #[tokio::test]
    async fn test_abort() -> Result<()> {
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();

        let agent = Agent::spawn(
            Uuid::new_v4(),
            Some("1".to_string()),
            move |_sender, message| {
                let tx = tx.clone();
                async move {
                    tx.send(message)?;
                    Result::<_, TokioSendError<_>>::Ok(())
                }
            },
        );

        let message = "hello world";
        agent.send(message)?;
        tokio::time::sleep(Duration::from_millis(100)).await;
        agent.abort();

        assert_eq!(
            rx.recv().await,
            Some(message),
            "testing that the agent places the message on the channel before it aborts"
        );

        Ok(())
    }
}
