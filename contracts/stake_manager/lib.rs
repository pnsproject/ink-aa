#![cfg_attr(not(feature = "std"), no_std, no_main)]

#[ink::contract(env = env::AccountAbstractionEnvironment)]
mod stake_manager {
    use ink::codegen::EmitEvent;
    use ink::storage::Mapping;
    use stake_manager_trait::{
        DepositInfo, Deposited, IStakeManager, StakeLocked, StakeUnlocked, StakeWithdrawn,
        Withdrawn,
    };

    #[ink(storage)]
    pub struct StakeManager {
        deposits: Mapping<AccountId, DepositInfo>,
    }

    impl StakeManager {
        /// Constructor that initializes the `bool` value to the given `init_value`.
        #[ink(constructor)]
        pub fn new() -> Self {
            Self {
                deposits: Mapping::default(),
            }
        }

        fn increment_deposit(&mut self, account: AccountId, amount: Balance) {
            let mut info = self.get_deposit_info(account);
            let new_amount = info.deposit.checked_add(amount);
            assert!(new_amount.is_some(), "deposit overflow");
            info.deposit = new_amount.unwrap();
            self.deposits.insert(account, &info);
        }
    }

    impl IStakeManager for StakeManager {
        #[ink(message)]
        fn get_deposit_info(&self, account: AccountId) -> DepositInfo {
            self.deposits.get(account).unwrap_or_default()
        }

        #[ink(message)]
        fn balance_of(&self, account: AccountId) -> Balance {
            self.get_deposit_info(account).deposit
        }

        #[ink(message, payable)]
        fn deposit_to(&mut self, account: AccountId) {
            self.increment_deposit(account, Self::env().transferred_value());
            let info = self.get_deposit_info(account);
            self.env().emit_event(Deposited {
                account,
                total_deposit: info.deposit,
            });
        }

        #[ink(message, payable)]
        fn add_stake(&mut self, unstake_delay_sec: Timestamp) {
            let info = self.deposits.get(Self::env().caller()).unwrap_or_default();
            assert!(unstake_delay_sec > 0, "must specify unstake delay");
            assert!(
                unstake_delay_sec >= info.unstake_delay_sec,
                "cannot decrease unstake time"
            );
            let stake = info.stake.checked_add(Self::env().transferred_value());
            assert!(stake.is_some(), "deposit overflow");
            let stake = stake.unwrap();
            assert!(stake > 0, "no stake specified");
            self.deposits.insert(
                Self::env().caller(),
                &DepositInfo {
                    deposit: info.deposit,
                    staked: true,
                    stake,
                    unstake_delay_sec,
                    withdraw_time: 0,
                },
            );
            Self::env().emit_event(StakeLocked {
                account: Self::env().caller(),
                total_staked: stake,
                unstake_delay_sec,
            });
        }

        #[ink(message)]
        fn unlock_stake(&mut self) {
            let info = self.get_deposit_info(self.env().caller());
            assert!(info.unstake_delay_sec > 0, "not staked");
            assert!(info.staked, "already unstaking");
            let withdraw_time = Self::env()
                .block_timestamp()
                .checked_add(info.unstake_delay_sec)
                .unwrap_or(Timestamp::MAX);

            self.deposits.insert(
                Self::env().caller(),
                &DepositInfo {
                    staked: false,
                    withdraw_time,
                    ..info
                },
            );
            Self::env().emit_event(StakeUnlocked {
                account: Self::env().caller(),
                withdraw_time,
            });
        }

        #[ink(message, payable)]
        fn withdraw_stake(&mut self, withdraw_address: AccountId) {
            let info = self.deposits.get(Self::env().caller()).unwrap_or_default();
            let stake = info.stake;
            assert!(stake > 0, "No stake to withdraw");
            assert!(info.withdraw_time > 0, "must call unlockStake() first");
            assert!(
                info.withdraw_time <= Self::env().block_timestamp(),
                "Stake withdrawal is not due"
            );
            self.deposits.insert(
                Self::env().caller(),
                &DepositInfo {
                    unstake_delay_sec: 0,
                    withdraw_time: 0,
                    stake: 0,
                    ..info
                },
            );
            Self::env().emit_event(StakeWithdrawn {
                account: Self::env().caller(),
                withdraw_address,
                amount: stake,
            });
            let transfer_result = self.env().transfer(withdraw_address, stake);

            assert!(transfer_result.is_ok(), "failed to withdraw stake");
        }

        #[ink(message, payable)]
        fn withdraw_to(&mut self, withdraw_address: AccountId, withdraw_amount: Balance) {
            let info = self.deposits.get(Self::env().caller()).unwrap_or_default();
            assert!(withdraw_amount <= info.deposit, "Withdraw amount too large");
            let deposit = info.deposit.checked_sub(withdraw_amount);
            assert!(deposit.is_some(), "deposit overflow");
            let deposit = deposit.unwrap();
            self.deposits
                .insert(Self::env().caller(), &DepositInfo { deposit, ..info });
            self.env().emit_event(Withdrawn {
                account: self.env().caller(),
                withdraw_address,
                amount: withdraw_amount,
            });
            let transfer_result = self.env().transfer(withdraw_address, withdraw_amount);

            assert!(transfer_result.is_ok(), "failed to withdraw");
        }
    }

    // TODO:
    /// This is how you'd write end-to-end (E2E) or integration tests for ink! contracts.
    ///
    /// When running these you need to make sure that you:
    /// - Compile the tests with the `e2e-tests` feature flag enabled (`--features e2e-tests`)
    /// - Are running a Substrate node which contains `pallet-contracts` in the background
    #[cfg(all(test, feature = "e2e-tests"))]
    mod e2e_tests {
        /// Imports all the definitions from the outer scope so we can use them here.
        use super::*;

        /// A helper function used for calling contract messages.
        use ink_e2e::build_message;

        /// The End-to-End test `Result` type.
        type E2EResult<T> = std::result::Result<T, Box<dyn std::error::Error>>;

        /// We test that we can upload and instantiate the contract using its default constructor.
        #[ink_e2e::test]
        async fn default_works(mut client: ink_e2e::Client<C, E>) -> E2EResult<()> {
            // Given
            let constructor = StakeManagerRef::default();

            // When
            let contract_account_id = client
                .instantiate("stake_manager", &ink_e2e::alice(), constructor, 0, None)
                .await
                .expect("instantiate failed")
                .account_id;

            // Then
            let get = build_message::<StakeManagerRef>(contract_account_id.clone())
                .call(|stake_manager| stake_manager.get());
            let get_result = client.call_dry_run(&ink_e2e::alice(), &get, 0, None).await;
            assert!(matches!(get_result.return_value(), false));

            Ok(())
        }

        /// We test that we can read and write a value from the on-chain contract contract.
        #[ink_e2e::test]
        async fn it_works(mut client: ink_e2e::Client<C, E>) -> E2EResult<()> {
            // Given
            let constructor = StakeManagerRef::new(false);
            let contract_account_id = client
                .instantiate("stake_manager", &ink_e2e::bob(), constructor, 0, None)
                .await
                .expect("instantiate failed")
                .account_id;

            let get = build_message::<StakeManagerRef>(contract_account_id.clone())
                .call(|stake_manager| stake_manager.get());
            let get_result = client.call_dry_run(&ink_e2e::bob(), &get, 0, None).await;
            assert!(matches!(get_result.return_value(), false));

            // When
            let flip = build_message::<StakeManagerRef>(contract_account_id.clone())
                .call(|stake_manager| stake_manager.flip());
            let _flip_result = client
                .call(&ink_e2e::bob(), flip, 0, None)
                .await
                .expect("flip failed");

            // Then
            let get = build_message::<StakeManagerRef>(contract_account_id.clone())
                .call(|stake_manager| stake_manager.get());
            let get_result = client.call_dry_run(&ink_e2e::bob(), &get, 0, None).await;
            assert!(matches!(get_result.return_value(), true));

            Ok(())
        }
    }
}
