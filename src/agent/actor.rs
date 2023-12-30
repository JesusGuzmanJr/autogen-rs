//! Actor trait.
pub trait Actor {
    type Message;
    type Error;

    /// Returns the actor's id.
    fn id(&self) -> uuid::Uuid;

    /// Returns the actor's name.
    fn name(&self) -> Option<&str>;

    /// Send a message to the actor.
    fn send(&self, message: Self::Message) -> Result<(), Self::Error>;
}
