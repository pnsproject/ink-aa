#![cfg_attr(not(feature = "std"), no_std, no_main)]

pub use simple_paymaster::SimplePaymasterRef;

#[ink::contract]
mod simple_paymaster {
    use base_paymaster::BasePaymasterTrait;
    use ink::prelude::vec;
    use ink::prelude::vec::Vec;
    use ink_aa::core::{error::Result, helpers::ValidationData};
    use ink_aa::core::{helpers::Aggregator, user_operation::UserOperation};
    use ink_aa::{core::env::AAEnvironment, traits::paymaster::PostOpMode};
    #[ink(storage)]
    pub struct SimplePaymaster;

    impl SimplePaymaster {
        #[ink(constructor)]
        pub fn new() -> Self {
            Self {}
        }

        /// Simply returns the current value of our `bool`.
        #[ink(message)]
        pub fn gi(&self) {}
    }

    impl BasePaymasterTrait for SimplePaymaster {
        #[ink(message)]
        fn validate_paymaster_user_op(
            &self,
            _user_op: UserOperation<AAEnvironment>,
            _user_op_hash: Hash,
            _max_cost: Balance,
        ) -> Result<(Vec<u8>, ValidationData<AAEnvironment>)> {
            Ok((
                vec![],
                ValidationData {
                    aggregator: Aggregator::NoAggregator,
                    valid_after: self.env().block_timestamp(),
                    valid_until: self.env().block_timestamp() + 5000,
                },
            ))
        }

        #[ink(message)]
        fn post_op(
            &self,
            _mode: PostOpMode,
            _context: Vec<u8>,
            _actual_gas_cost: Balance,
        ) -> Result<()> {
            // TODO:
            Ok(())
        }
    }
}
