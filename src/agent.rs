use std::{future::Future, pin::Pin};

use {
    std::{fmt::Debug, time::Duration},
    tokio::{sync::mpsc::UnboundedSender, task::JoinHandle},
    uuid::Uuid,
};

/// The amount of time to wait for an agent to terminate.
const DEFAULT_GRACE_PERIOD: Duration = Duration::from_secs(3);

/// A handle to an agent.
pub struct Agent<M, E> {
    /// Unique identifier for the agent.
    pub id: Uuid,

    /// A user-friendly name for the agent.
    pub name: String,

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
    pub fn new<H, R>(name: String, on_message: H) -> Self
    where
        // make H be a closure that takes an M and returns a future that resolves to a Result<R, E>
        H: Fn(M) -> R + Send + Sync + 'static,
        R: Future<Output = Result<(), E>> + Send + 'static,
    {
        let (sender, mut receiver) = tokio::sync::mpsc::unbounded_channel();

        let id = Uuid::new_v4();
        let handle = {
            let name = name.clone();
            tokio::spawn(async move {
                tracing::trace!(name, %id, "starting",);

                while let Some(message) = receiver.recv().await {
                    tracing::trace!(name, %id, ?message, "received message");
                    on_message(message).await?;
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
            std::env::var("GRACE_PERIOD_SECONDS")
                .ok()
                .and_then(|s| s.parse().ok())
                .map(Duration::from_secs)
                .unwrap_or(DEFAULT_GRACE_PERIOD),
        )
        .await;
        self.handle.abort();
        tracing::trace!(name = self.name, id = %self.id, "terminated");
    }

    /// Send a message to the agent.
    pub fn send(&self, message: M) {
        self.sender.send(message).unwrap();
    }
}
