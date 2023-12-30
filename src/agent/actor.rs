//! Actor trait.
#[allow(async_fn_in_trait)]

pub trait Actor {
    type Message;
    type Error;

    /// Returns the actor's id.
    fn id(&self) -> uuid::Uuid;

    /// Returns the actor's name.
    fn name(&self) -> Option<&str>;

    /// Send a message to the actor.
    fn send(&self, message: Self::Message) -> Result<(), Self::Error>;

    // Terminates the actor by closing its message channel and waiting for it
    /// to finish processing remaining messages. Consumes the actor since it
    /// can no longer process messages.
    async fn terminate(self);

    /// Aborts the actor's event loop immediately without waiting for it to
    /// finish.
    fn abort(self);
}
