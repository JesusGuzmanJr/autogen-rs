pub mod agent;

pub use agent::Agent;

#[cfg(test)]
mod tests {
    use super::*;

    type TokioSendError<T> = tokio::sync::mpsc::error::SendError<T>;
    type Error<T> = agent::SendError<T>;

    fn agent_id(id: usize) -> String {
        format!("agent {}", id)
    }

    #[tokio::test]
    async fn test_actor_processes_message() -> Result<(), Error<&'static str>> {
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();

        let agent = Agent::new(agent_id(1), move |message| {
            let tx = tx.clone();
            async move {
                tx.send(message)?;
                Result::<_, TokioSendError<_>>::Ok(())
            }
        });

        let message = "hello world";
        agent.send(message)?;
        assert_eq!(rx.recv().await, Some(message));
        Ok(())
    }

    #[tokio::test]
    async fn test_multiple_agents() -> Result<(), Error<&'static str>> {
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();

        let agent_1 = Agent::new(agent_id(1), move |message| {
            let tx = tx.clone();
            async move {
                tx.send(message)?;
                Result::<_, TokioSendError<_>>::Ok(())
            }
        })
        .sender();

        let agent_2 = Agent::new(agent_id(2), move |message| {
            let agent_1 = agent_1.clone();
            async move {
                agent_1.send(message)?;
                Result::<_, Error<&'static str>>::Ok(())
            }
        });

        let message = "hello world";
        agent_2.send(message)?;
        assert_eq!(rx.recv().await, Some(message));
        Ok(())
    }
}
