#![cfg_attr(not(feature = "std"), no_std, no_main)]

pub use stake_manager::StakeManagerRef;

#[ink::contract(env = ink_aa::core::env::AAEnvironment)]
mod stake_manager {
    use ink::storage::Mapping;
    use ink_aa::{
        core::{
            env::AAEnvironment,
            error::{Error, Result},
        },
        traits::stake_manager::{DepositInfo, IStakeManager},
    };

    #[ink(storage)]
    pub struct StakeManager {
        deposits: Mapping<AccountId, DepositInfo<AAEnvironment>>,
    }

    // TODO: 等`event2.0`合并发布之后，转移到`traits`中
    /// 存款成功时触发的事件。
    #[ink(event)]
    pub struct Deposited {
        #[ink(topic)]
        /// 进行存款的账户 ID。
        pub account: AccountId,
        /// 存款总额。
        pub total_deposit: Balance,
    }

    /// 取款成功时触发的事件。
    #[ink(event)]
    pub struct Withdrawn {
        #[ink(topic)]
        /// 进行取款的账户 ID。
        pub account: AccountId,
        /// 取款金额将转入的账户 ID。
        pub withdraw_address: AccountId,
        /// 取款金额。
        pub amount: Balance,
    }

    /// 抵押成功时触发的事件。
    #[ink(event)]
    pub struct StakeLocked {
        #[ink(topic)]
        /// 进行抵押的账户 ID。
        pub account: AccountId,
        /// 抵押总额。
        pub total_staked: Balance,
        /// 抵押在可取回前需要的延迟时间（秒）。
        pub unstake_delay_sec: Timestamp,
    }

    /// 取消抵押成功时触发的事件。
    #[ink(event)]
    pub struct StakeUnlocked {
        #[ink(topic)]
        /// 进行取消抵押的账户 ID。
        pub account: AccountId,
        /// 抵押可以取回的时间。
        pub withdraw_time: Timestamp,
    }

    /// 取回抵押成功时触发的事件。
    #[ink(event)]
    pub struct StakeWithdrawn {
        #[ink(topic)]
        /// 进行取回抵押的账户 ID。
        pub account: AccountId,
        /// 取回抵押金额将转入的账户 ID。
        pub withdraw_address: AccountId,
        /// 取回抵押的金额。
        pub amount: Balance,
    }

    impl StakeManager {
        /// Constructor that initializes the `bool` value to the given `init_value`.
        #[ink(constructor)]
        pub fn new() -> Self {
            Self {
                deposits: Mapping::default(),
            }
        }

        #[ink(message, payable)]
        pub fn required_prefund(
            &mut self,
            required_address: AccountId,
            required_amount: Balance,
        ) -> Result<()> {
            let info = self.deposits.get(required_address).unwrap_or_default();
            if required_amount > info.deposit {
                return Err(Error::WithdrawAmountTooLarge);
            }
            let deposit = info
                .deposit
                .checked_sub(required_amount)
                .ok_or(Error::DepositOverflow)?;

            self.deposits
                .insert(Self::env().caller(), &DepositInfo { deposit, ..info });
            // self.env().emit_event(Withdrawn {
            //     account: self.env().caller(),
            //     withdraw_address,
            //     amount: withdraw_amount,
            // });
            // self.env()
            //     .transfer(withdraw_address, withdraw_amount)
            //     .map_err(|_| Error::FailedToWithdraw)?;

            Ok(())
        }

        #[ink(message)]
        pub fn increment_deposit(&mut self, account: AccountId, amount: Balance) -> Result<()> {
            let mut info = self.get_deposit_info(account);
            let new_amount = info
                .deposit
                .checked_add(amount)
                .ok_or(Error::DepositOverflow)?;

            info.deposit = new_amount;
            self.deposits.insert(account, &info);
            Ok(())
        }
    }

    impl IStakeManager for StakeManager {
        #[ink(message)]
        fn get_deposit_info(&self, account: AccountId) -> DepositInfo<AAEnvironment> {
            self.deposits.get(account).unwrap_or_default()
        }

        #[ink(message)]
        fn balance_of(&self, account: AccountId) -> Balance {
            self.get_deposit_info(account).deposit
        }

        #[ink(message, payable)]
        fn deposit_to(&mut self, account: AccountId) -> Result<()> {
            self.increment_deposit(account, Self::env().transferred_value())?;
            let info = self.get_deposit_info(account);
            self.env().emit_event(Deposited {
                account,
                total_deposit: info.deposit,
            });
            Ok(())
        }

        #[ink(message, payable)]
        fn add_stake(&mut self, unstake_delay_sec: Timestamp) -> Result<()> {
            let info = self.deposits.get(Self::env().caller()).unwrap_or_default();

            if unstake_delay_sec <= 0 {
                return Err(Error::MustSpecifyUnstakeDelay);
            }

            if unstake_delay_sec < info.unstake_delay_sec {
                return Err(Error::CannotDecreaseUnstakeTime);
            }
            let stake = info
                .stake
                .checked_add(Self::env().transferred_value())
                .ok_or(Error::DepositOverflow)?;

            if stake <= 0 {
                return Err(Error::NoStakeSpecified);
            }
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
            Ok(())
        }

        #[ink(message)]
        fn unlock_stake(&mut self) -> Result<()> {
            let info = self.get_deposit_info(self.env().caller());

            if info.unstake_delay_sec <= 0 {
                return Err(Error::NotStaked);
            }
            if !info.staked {
                return Err(Error::AlreadyUnstaking);
            }
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
            Ok(())
        }

        #[ink(message, payable)]
        fn withdraw_stake(&mut self, withdraw_address: AccountId) -> Result<()> {
            let info = self.deposits.get(Self::env().caller()).unwrap_or_default();
            let stake = info.stake;

            if stake <= 0 {
                return Err(Error::NoStakeToWithdraw);
            }

            if info.withdraw_time <= 0 {
                return Err(Error::MustCallUnlockStakeFirst);
            }

            if info.withdraw_time > Self::env().block_timestamp() {
                return Err(Error::StakeWithdrawalIsNotDue);
            }

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
            self.env()
                .transfer(withdraw_address, stake)
                .map_err(|_| Error::FailedToWithdrawStake)?;

            Ok(())
        }

        #[ink(message, payable)]
        fn withdraw_to(
            &mut self,
            withdraw_address: AccountId,
            withdraw_amount: Balance,
        ) -> Result<()> {
            let info = self.deposits.get(Self::env().caller()).unwrap_or_default();
            if withdraw_amount > info.deposit {
                return Err(Error::WithdrawAmountTooLarge);
            }
            let deposit = info
                .deposit
                .checked_sub(withdraw_amount)
                .ok_or(Error::DepositOverflow)?;

            self.deposits
                .insert(Self::env().caller(), &DepositInfo { deposit, ..info });
            self.env().emit_event(Withdrawn {
                account: self.env().caller(),
                withdraw_address,
                amount: withdraw_amount,
            });
            self.env()
                .transfer(withdraw_address, withdraw_amount)
                .map_err(|_| Error::FailedToWithdraw)?;

            Ok(())
        }
    }
}
