#![cfg_attr(not(feature = "std"), no_std, no_main)]

pub use base_paymaster::{BasePaymasterRef, BasePaymasterTrait};

#[ink::contract(env = ink_aa::core::env::AAEnvironment)]
mod base_paymaster {
    use ink::prelude::vec::Vec;
    use ink_aa::{
        core::{
            env::AAEnvironment,
            error::{Error, Result},
            helpers::ValidationData,
            user_operation::UserOperation,
        },
        traits::{
            entry_point::{EntryPointRef, StakeManagerRef},
            paymaster::{IPaymaster, PostOpMode},
            stake_manager::IStakeManager,
        },
    };

    #[ink(storage)]
    pub struct BasePaymaster {
        entry_point: AccountId,
        owner: AccountId,
        advance: AccountId,
    }

    impl BasePaymaster {
        #[ink(constructor)]
        pub fn new(entry_point: AccountId, owner: AccountId, advance: AccountId) -> Self {
            Self {
                entry_point,
                owner,
                advance,
            }
        }

        #[ink(message)]
        pub fn get_owner(&self) -> AccountId {
            self.owner
        }

        #[ink(message)]
        pub fn entry_point(&self) -> AccountId {
            self.entry_point
        }

        #[ink(message)]
        pub fn require_from_entry_point(&self) -> Result<()> {
            self.inner_require_from_entry_point()
        }
        #[ink(message)]
        pub fn only_owner(&self) -> Result<()> {
            self.inner_only_owner()
        }
    }

    #[ink(impl)]
    impl BasePaymaster {
        #[inline]
        pub fn inner_require_from_entry_point(&self) -> Result<()> {
            if self.env().caller() != self.entry_point {
                return Err(Error::NotFromEntryPoint);
            }

            Ok(())
        }

        pub fn entry_point_ref(&self) -> EntryPointRef<AAEnvironment> {
            self.entry_point.into()
        }

        pub fn stake_manager_ref(&self) -> StakeManagerRef<AAEnvironment> {
            self.entry_point.into()
        }

        pub fn paymaster_ref(&self) -> ink::contract_ref!(BasePaymasterTrait) {
            self.advance.into()
        }

        pub fn inner_only_owner(&self) -> Result<()> {
            if self.env().caller() != self.owner {
                return Err(Error::NotOwner);
            }
            Ok(())
        }
    }

    impl IPaymaster for BasePaymaster {
        #[ink(message)]
        fn validate_paymaster_user_op(
            &self,
            user_op: UserOperation<AAEnvironment>,
            user_op_hash: Hash,
            max_cost: Balance,
        ) -> Result<(Vec<u8>, ValidationData<AAEnvironment>)> {
            self.inner_require_from_entry_point()?;
            self.paymaster_ref()
                .validate_paymaster_user_op(user_op, user_op_hash, max_cost)
        }

        #[ink(message)]
        fn post_op(
            &self,
            mode: PostOpMode,
            context: Vec<u8>,
            actual_gas_cost: Balance,
        ) -> Result<()> {
            self.inner_require_from_entry_point()?;
            self.paymaster_ref().post_op(mode, context, actual_gas_cost)
        }
    }

    #[ink::trait_definition]
    pub trait BasePaymasterTrait {
        #[ink(message)]
        fn validate_paymaster_user_op(
            &self,
            user_op: UserOperation<AAEnvironment>,
            user_op_hash: Hash,
            max_cost: Balance,
        ) -> Result<(Vec<u8>, ValidationData<AAEnvironment>)>;

        #[ink(message)]
        fn post_op(
            &self,
            mode: PostOpMode,
            context: Vec<u8>,
            actual_gas_cost: Balance,
        ) -> Result<()>;
    }

    impl IStakeManager for BasePaymaster {
        #[ink(message)]
        fn get_deposit_info(
            &self,
            account: AccountId,
        ) -> ink_aa::traits::stake_manager::DepositInfo<AAEnvironment> {
            self.stake_manager_ref().get_deposit_info(account)
        }
        #[ink(message)]
        fn balance_of(&self, account: AccountId) -> Balance {
            self.stake_manager_ref().balance_of(account)
        }
        #[ink(message, payable)]
        fn deposit_to(&mut self, account: AccountId) -> Result<()> {
            self.stake_manager_ref().deposit_to(account)
        }
        #[ink(message, payable)]
        fn add_stake(&mut self, unstake_delay_sec: Timestamp) -> Result<()> {
            self.inner_only_owner()?;
            self.stake_manager_ref().add_stake(unstake_delay_sec)
        }
        #[ink(message)]
        fn unlock_stake(&mut self) -> Result<()> {
            self.inner_only_owner()?;
            self.stake_manager_ref().unlock_stake()
        }
        #[ink(message, payable)]
        fn withdraw_stake(&mut self, withdraw_address: AccountId) -> Result<()> {
            self.inner_only_owner()?;
            self.stake_manager_ref().withdraw_stake(withdraw_address)
        }
        #[ink(message, payable)]
        fn withdraw_to(
            &mut self,
            withdraw_address: AccountId,
            withdraw_amount: Balance,
        ) -> Result<()> {
            self.inner_only_owner()?;
            self.stake_manager_ref()
                .withdraw_to(withdraw_address, withdraw_amount)
        }
    }
}
