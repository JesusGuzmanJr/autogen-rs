# autogen-rs 😃

## Introduction

The [autogen-framework](https://github.com/microsoft/autogen) is a python library that enables developers to orchestrate LLM agents to solve tasks. In this proposal I will outline how we can port autogen's agents and their communication logic to Rust.

# Agents
Agents are actors that can converse with each other to collectively perform tasks through chat. They can be powered by LLMs, tools or humans. As actors they are able send and receive messages while encapsulating state and behavior.
## Agents in Autogen

Autogen defines two types of agents: `UserProxyAgent` and `AssistantAgent`. The `UserProxyAgent` is a proxy for the user that will prompt for human input every time a message is received. The `AssistantAgent` is an agent that uses OpenAI chat completion functionality to generate a response every time a message is received. 

There are many configuration parameters that can be adjusted for both agents (which subclass a parent `ConversableAgent(Agent)` class) such as:
- the starting prompt
- whether to stop after a certain number of turns or when a terminal message is received
- whether to run OpenAI generated code, etc.

There is a lot of functionality provided by the `ConversableAgent` class that is common to both agents. Because Rust does not have classes or inheritance, we need to find a different way to implement this functionality.

## Modeling Agents in Rust

Agents can be modeled as actors that receive messages from a queue and process them by invoking their handler function. Each running actor will have an event looping in a green thread (async task) awaiting a message and invoking the handler function. We can terminate the actor (i.e. break out of the event loop) when there are no more messages in the queue.

Leveraging Rust's fearless concurrency, we can use asynchronous tasks from one of popular async runtimes such as [tokio](https://tokio.rs/) to run the event loop. 
> Bandwidth permitted, we could also provide feature flagged support for other async runtimes e.g. [async-std](https://async.rs/) or [smol](https://github.com/smol-rs/smol).

Tokio provides a multiple-produce single-consumer (mpsc) channel that we can use as the actor's mailbox which will serve as the queue of messages. (Or we can use alternative crates for channels such as [crossbeam-channel](https://github.com/crossbeam-rs/crossbeam).) By using mpsc channels, we can have multiple senders and a single receiver. This will allow us to have multiple agents sending messages to a single agent. Tokio also provides broadcast channels which allow us to have multiple receivers. This will allow us to have multiple agents receiving messages from a single agent (such as the case of a manager agent).

We want our framework to facilitate the development of all kinds of AI agents. We may want to have an agent that uses a different chat completion model such as [EleutherAI's](https://www.eleuther.ai/about) [gpt-neox-20b](https://huggingface.co/EleutherAI/gpt-neox-20b). As such, we need a generic way to define the handler function for an agent. The handler function will take a message as input and return a result (either a message or an error). 

There are two approaches we can use here. We can use Rust's traits to define a `Responder` trait that will be implemented by all agents. We use associated types to group together types that semantically belong together. Here's an example of how we can define the `Responder` trait barring the fact that stable Rust does not yet support async functions in traits, ignoring `Send`, `Sync`, or `'static` type constraints, and omitting a context type for simplicity:

```rust
trait Responder {
    type Input;
    type Output;
    type Error;

    // note: stable rust doesn't allow async fn in traits yet
    async fn respond(&self, message: Self::Input) -> Result<Self::Output, Self::Error>;
```

Alternatively, we can leverage generics to define a base struct that is generic over the types of the handler. This is the approach I took in the [`Agent`](src/agent/mod.rs) struct. I chose this approach mostly to practice using generics in Rust. There is a third approach of using traits with generic parameters (as opposed to associated types) but I don't think that's ergonomic because it would require the developer to provide type annotations to indicate which implementation of the trait to use since now we can have multiple implementations of the trait for the same type.

> While implementing agents as actors with tokio can be done without using actor libraries, we should consider using one such as [actix](https://actix.rs/docs/actix/actor) or [coerce](https://docs.rs/coerce/latest/coerce/index.html) to simplify the implementation. Additionally, Alice Rhyl, a well-known Rustacean has a post on how to implement [Actors in Rust with Tokio](https://ryhl.io/blog/actors-with-tokio/) sans actor libraries.

### State Management
Agents may need to retain some form of state for conversational context or to recall past interactions. This could be stored internally within the agent, but a more scalable solution might involve a separate data store or database.

## Modeling Group Chat

To port autogen's `GroupChat` , we can define and implement a struct of the same name. A group chat is not an agent but rather an orchestrator of agents. It is responsible for:

- choosing the next speaker
- filtering function calls, i.e. when a chat message suggests a function call, the next agent could be chosen to be one that has the corresponding function handler
- storing chat messages
- storing references to agents
- letting the admin agent take over when a keyboard interrupt is received or when a SIGTERM is caught
  
### Speaker Selection

The `GroupChat` impl can have a `next_speaker()` method that will determine the next speaker. When creating the `GroupChat` struct, the developer configures what algorithm to use for speaker selection:
```rust
enum SpeakerSelection {
    /// Speaker is selected automatically by LLM.
    Auto,
    /// Speaker is selected manually by user.
    Manual,
    /// Speaker is selected randomly.
    Random,
    /// Speaker is selected in a round robin fashion, i.e. iterating in the same order in a loop.
    RoundRobin,
}
```

In the case of collaboration, we could develop more intricate protocol mechanisms. Maybe votes are taken by broadcasting a proposal and tallying replies. The specifics will depend heavily on what kind of collaboration the agents are undertaking. Additionally, since we are implementing a library, we should provide examples of how to use the library to implement more complex collaboration. Dogfooding the library in this way will help us identify any shortcomings and improve the developer experience.

Below are a few communication methods that can be used between agents in a group chat:

1. **Direct Communication**: The senders direct their messages to specific agents. Typically, this is done using an addressing mechanism where each agent has a unique handle.

2. **Broadcast Communication**: An agent sends a message to all other agents in the chat room. This can happen in an open system where the total number of agents is unknown or in a system where a message is relevant to all agents.

In terms of specific protocols or strategies for managing communication, below are a few general approaches:

1. **Request-Reply Protocol**: One agent (client) sends a request message to another agent (server) and the server replies to the request. This synchronous interaction is simple and common.

2. **Publish-Subscribe Protocol**: Agents publish messages to specific channels or topics while other agents subscribe to these topics to receive the messages. This establishes a pattern of asynchronous, de-coupled communication.


# Further Capabilities

In addition to the communication and decision-making capabilities, agents can have other capabilities such as learning, self-management, and interaction capabilities.

-  **Learning Capabilities** Some agents can learn from past interactions and make improvements to their behavior. This typically involves machine learning techniques, and could be particularly powerful in complex systems where optimal behavior isn't known a priori.

- **Self-Management Capabilities** Agents can be implemented in a self-healing manner. This includes the ability to handle errors, recover from failures, or manage local resources to achieve their goals.

- **Interaction Capabilities** Agents can interact with the environment or system they're in. This could involve reading data from a database, making requests to a web API, accessing files, or manipulating other system resources. 

# Running the Code

To run the code, you will need to have Rust installed. You'll need a nightly version which cargo will install upon first invocation.

- To run unit tests: `cargo test`
- To run the `user_agent` example: `RUST_LOG=debug cargo run --example user_agent`
- To view docs: `cargo doc --open`



