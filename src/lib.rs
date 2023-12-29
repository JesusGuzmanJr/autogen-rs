pub mod agent;

pub use agent::Agent;

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(thiserror::Error, Debug)]
    enum Error {}

    #[tokio::test]
    async fn test() {
        let agent = Agent::new("test".to_string(), |message| async move {
            println!("message: {:?}", message);
            Result::<_, Error>::Ok(())
        });

        agent.send("hello".to_string());
    }
}
