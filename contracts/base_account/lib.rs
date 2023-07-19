#![cfg_attr(not(feature = "std"), no_std, no_main)]

pub use base_account::{BaseAccountRef, BaseAccountTrait};

#[ink::contract(env = ink_aa::core::env::AAEnvironment)]
mod base_account {
    use ink_aa::{
        core::{
            env::AAEnvironment,
            error::{Error, Result},
            helpers::ValidationData,
            user_operation::UserOperation,
        },
        traits::{
            account::IAccount,
            entry_point::{EntryPointRef, NonceManagerRef, StakeManagerRef},
            nonce_manager::INonceManager,
        },
    };

    #[ink(storage)]
    pub struct BaseAccount {
        entry_point: AccountId,
        advanced: AccountId,
    }

    impl BaseAccount {
        #[ink(constructor)]
        pub fn new(entry_point: AccountId, advanced: AccountId) -> Self {
            Self {
                entry_point,
                advanced,
            }
        }

        #[ink(message)]
        pub fn get_nonce(&self) -> [u8; 32] {
            self.nonce_manager_ref()
                .get_nonce(self.env().account_id(), [0; 24])
        }

        /**
         * sends to the entrypoint (msg.sender) the missing funds for this transaction.
         * subclass MAY override this method for better funds management
         * (e.g. send to the entryPoint more than the minimum required, so that in future transactions
         * it will not be required to send again)
         * @param missingAccountFunds the minimum value this method should send the entrypoint.
         *  this value MAY be zero, in case there is enough deposit, or the userOp has a paymaster.
         */
        fn pay_prefund(&self, missing_account_funds: Balance) {
            if missing_account_funds != 0 {
                //ignore failure (its EntryPoint's job to verify, not account.)
                let _ = self
                    .env()
                    .transfer(self.env().caller(), missing_account_funds);
            }
        }
    }

    #[ink(impl)]
    impl BaseAccount {
        pub fn entry_point_ref(&self) -> EntryPointRef<AAEnvironment> {
            self.entry_point.into()
        }

        pub fn base_ref(&self) -> ink::contract_ref!(BaseAccountTrait) {
            self.advanced.into()
        }

        pub fn stake_manager_ref(&self) -> StakeManagerRef<AAEnvironment> {
            self.entry_point.into()
        }

        pub fn nonce_manager_ref(&self) -> NonceManagerRef<AAEnvironment> {
            self.entry_point.into()
        }

        #[inline]
        pub fn inner_require_from_entry_point(&self) -> Result<()> {
            if self.env().caller() != self.entry_point {
                return Err(Error::NotFromEntryPoint);
            }

            Ok(())
        }
    }

    impl IAccount for BaseAccount {
        #[ink(message)]
        fn validate_user_op(
            &self,
            user_op: UserOperation<AAEnvironment>,
            user_op_hash: Hash,
            missing_account_funds: Balance,
        ) -> Result<ValidationData<AAEnvironment>> {
            self.inner_require_from_entry_point()?;
            let advanced = self.base_ref();
            let nonce = user_op.nonce;
            let validation_data = advanced.validate_signature(user_op, user_op_hash)?;
            advanced.validate_nonce(nonce)?;
            self.pay_prefund(missing_account_funds);
            return Ok(validation_data);
        }
    }

    #[ink::trait_definition]
    pub trait BaseAccountTrait {
        /// 执行一批 UserOperation。
        /// 不使用签名聚合器。    
        /// 如果任何账户需要聚合器(即,在执行 simulateValidation 时返回了聚合器),则必须使用 handleAggregatedOps()。
        ///  
        /// - `ops` 要执行的操作    
        /// - `beneficiary` 用于接收费用的地址
        #[ink(message, payable)]
        fn validate_signature(
            &self,
            op: UserOperation<AAEnvironment>,
            user_op_hash: Hash,
        ) -> Result<ValidationData<AAEnvironment>>;

        #[ink(message)]
        fn validate_nonce(&self, nonce: [u8; 32]) -> Result<()>;
    }
}
