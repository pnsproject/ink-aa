# Polkadot Smart Account 

## Project Overview

Polkadot Smart Account is a project that brings the Ethereum EIP-4337 account abstraction to the Substrate ecosystem. It implements account abstraction at the application level without the need for blockchain-level protocol changes.

The project is initiated by the PNS project team, which is a decentralized identity and account management solution. We believe that the Substrate and Polkadot ecosystems need their own account abstraction standards, which is why we started this project.

## Motivation

- Existing account management methods have usability issues and cannot provide a good user experience.

- Account abstraction can enhance security, support more account functions, and improve user experience.

- The current account abstraction solution requires blockchain-level protocol changes, which is difficult to promote.

- The Substrate ecosystem needs an account abstraction standard that can be implemented without protocol changes.

## Solution

Polkadot Smart Account implements account abstraction at the application level by introducing the concepts of User Operations and Bundlers, without the need for blockchain protocol adjustments.

It allows users to use smart contract wallets instead of EOAs as their main accounts, and the wallet contracts can contain any verification logic. It also supports payment management, signature aggregation, and other functions.

We will implement this solution in the `ink!` ecosystem and ultimately extract a PSP standard specification suitable for Substrate.

## User Operation Flow

The User Operation flow is as follows:

1. The user constructs a UserOperation object containing the operation to be executed.

2. The user broadcasts the UserOperation to the public User Operation Memory Pool.

3. The Bundler retrieves UserOperations from the memory pool and runs local simulation to verify their validity.

4. The Bundler bundles validated UserOperations into a transaction and calls Entrypoint's `handle_ops` method. 

5. Entrypoint verifies and executes each UserOperation, and interacts with the chain.

The Bundler plays a pivotal role in the User Operation flow by receiving, validating, batching and ultimately sending UserOperations on chain. Its workflow is shown in the following diagram:

```mermaid
sequenceDiagram
participant U as User
participant C as Bundler Client
participant S as Bundler Server  
participant E as Entrypoint

U->>C: Construct UserOperation
C->>U: Sign UserOperation  
U->>C: Send Signed UserOperation
C->>S: Send via RPC/API
S->>S: Queue UserOperation   

loop Process Queue
  S->>S: Dequeue UserOperation
  S->>S: Local Simulation
  alt Invalid 
    S-->>C: Return Result
  else Valid
    S->>S: Batch UserOperations
    opt Reach Batch Size Threshold
      S->>E: Send Batch Tx
    end 
  end
end

E-->>S: Emit Events
S->>S: Handle Events, Update Reputation etc.  
S-->>C: Return Result
C-->>U: Return Result
```

After the Bundler bundles validated UserOperations and sends the transaction to Entrypoint, further processing logic is executed:

The Entrypoint contract implements the core logic for processing UserOperations. It contains methods like handle_ops to process batched UserOperations, handle_aggregated_ops for aggregated signatures, etc.

Entrypoint also implements functionalities related to deposit, withdrawal, gas payment, and deposit protection. It interacts with the chain to update state based on the UserOperations.

The workflow is shown below:

```mermaid
sequenceDiagram
participant U as User
participant B as Bundler
participant E as EntryPoint

U->>B: Send UserOp
B->>E: Batch UserOps 
E->>E: Check Paymaster
alt Paymaster
  E->>Paymaster: Verify Payment
  alt Can Pay
    Paymaster-->>E: Pay Gas
  else Cannot Pay
    E-->>B: Fail
  end
else Sender
  E->>Sender: Check Balance 
  alt Can Pay  
    Sender-->>E: Pay Gas
  else Cannot Pay
    E-->>B: Fail
  end
end

opt All Paid 
  E->>E: Execute UserOps
end 
```

## Bundler

The Bundler is an independent component responsible for retrieving UserOperations, validating, batching and sending them to Entrypoint for execution. 

Its main functions include:

- Providing RPC/REST APIs to receive UserOperations

- Queuing and processing UserOperations 

- Validating UserOperations via local simulation

- Batching validated UserOperations into transactions

- Calling Entrypoint's `handle_ops` to execute transactions

- Implementing reputation system, event monitoring 

- Caching, debugging interfaces

Bundler enables the closed-loop User Operation flow, achieving account abstraction without blockchain changes.

Bundler plays a key role by receiving, validating, batching user operations, and sending them to Entrypoint. It is a pivotal component that connects the off-chain User Operation construction and on-chain execution.

## Technical Details

### EntryPoint Contract

The `EntryPoint` contract contains two main methods that implement the processing logic for `UserOperations`: `handle_ops` and `handle_aggregated_ops`. 

The `handle_ops` method takes a batch of `UserOperation` as input, validates and executes each operation. The method signature in Rust is as follows:

```rust
pub fn handle_ops(&mut self, ops: Vec<UserOperation>) -> Result<()>
```

The `handle_aggregated_ops` method is similar to `handle_ops`, but it is used for processing `UserOperation` with signature aggregation. It first validates the signature aggregation and then validates and executes each operation. The method signature in Rust is as follows:

```rust
pub fn handle_aggregated_ops(&mut self, ops: Vec<UserOperation>) -> Result<()>
```

In addition to these methods, the `EntryPoint` contract also implements several related functions such as deposit, withdraw, and deposit security. 

The `deposit` function is used to deposit funds for the user. The method signature in Rust is as follows:

```rust
pub fn deposit(&mut self) -> Result<()>
```

The `withdraw` function is used to withdraw funds for the user. It takes the amount to withdraw as input and returns a `Result` indicating success or failure. The method signature in Rust is as follows:

```rust
pub fn withdraw(&mut self, amount: Balance) -> Result<()>
```

Overall, the `EntryPoint` contract provides the main entry point for processing user operations and related functions for managing user funds.

### Bundler

The Bundler implements the key functionality of receiving, validating, batching user operations and sending them to the Entrypoint for execution. 

The Bundler provides RPC and REST APIs for receiving UserOperations from users:

```rust
#[rpc(server, client)]
pub trait BundlerRpc {

  /// Send user operation
  #[method(name = "sendUserOperation")]
  async fn send_user_op(&self, op: UserOperation) -> Result<OpHash, Error>;

  /// Get user operation status
  #[method(name = "getUserOpStatus")]
  async fn get_user_op_status(&self, op_hash: OpHash) -> Result<OpStatus, Error>;
}
```

After receiving a UserOperation, the Bundler adds it to a queue for processing: 

```rust 
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct UserOperation {
  //...
  pub signature: Vec<u8>,
}

// Add UserOperation to processing queue
fn add_to_queue(user_op: UserOperation) {
  let queue = ...;
  queue.push(user_op); 
}
```

The Bundler periodically takes UserOperations from the queue and performs local simulation to verify them:

```rust
// Periodically take and verify UserOperations from queue
fn process_queue() {
  let user_op = queue.pop();
  if verify_user_op(user_op) {
    // Verification succeeded, batch process 
  } else {
    // Verification failed, return error
  }
}
```

After verification succeeds, the Bundler batches and packages the UserOperations and sends them to the Entrypoint for execution.

The Bundler implements the key functionality of taking user operations constructed off-chain and sending them on-chain for execution. 

*Note: The interfaces shown are subject to change during further development.*

## Roadmap

1. Implement EntryPoint and Bundler based on `ink!`.

2. Conduct integration testing and security auditing.

3. Propose a PSP standard specification suitable for Substrate.

4. Promote in the Polkadot ecosystem and collaborate with other projects.

### Detailed Roadmap

- Q3 2023: Develop and test EntryPoint and Bundler.

- Q4 2023: Release EntryPoint and Bundler.

- Q1 2024: Conduct integration testing and security auditing.

- Q2 2024: Propose a PSP standard specification suitable for Substrate.

- Q3 2024: Promote in the Polkadot ecosystem and collaborate with other projects.

## Conclusion

The Polkadot Smart Account project aims to implement Substrate ecosystem's account abstraction standards at the application level, without the need for blockchain-level protocol changes. The project introduces User Operations and Bundlers to enable smart contract wallets and support payment management, signature aggregation, and other functions.

The User Operation flow is closed-looped, with the Bundler as a key component. The Bundler is responsible for retrieving UserOperations, running local simulations, bundling validated UserOperations into transactions, and calling EntryPoint's methods for execution.

The EntryPoint contract implements the processing logic for UserOperations and related functions such as deposit, withdrawal, and deposit protection.

The project's roadmap includes developing and testing EntryPoint and Bundler, conducting integration testing and security auditing, proposing a PSP standard specification suitable for Substrate, and promoting the project in the Polkadot ecosystem and collaborating with other projects.

The Polkadot Smart Account project addresses the usability issues of existing account management methods, enhances security, supports more account functions, and improves user experience.