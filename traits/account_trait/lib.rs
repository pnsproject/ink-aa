#![cfg_attr(not(feature = "std"), no_std, no_main)]

use helpers::ValidationData;
use ink::env::Environment;
use user_operation::EnvUserOperation;

#[ink::trait_definition]
pub trait Account {
    /// Resets the current value to zero.
    #[ink(message)]
    fn validate_user_op(
        &self,
        user_op: EnvUserOperation<Self::Env>,
        user_op_hash: <Self::Env as Environment>::Hash,
        missing_account_funds: <Self::Env as Environment>::Balance,
    ) -> ValidationData<<Self::Env as Environment>::AccountId>;
}
