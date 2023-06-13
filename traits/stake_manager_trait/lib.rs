#![cfg_attr(not(feature = "std"), no_std, no_main)]

pub use stake_manager_trait::{
    DepositInfo, Deposited, IStakeManager, StakeInfo, StakeLocked, StakeUnlocked, StakeWithdrawn,
    Withdrawn,
};

/// 用于管理存款和抵押的 Ink! 合约特性的模块。
#[ink::contract(env = env::AccountAbstractionEnvironment)]
mod stake_manager_trait {
    use ink::storage::traits::StorageLayout;
    use scale::{Decode, Encode};
    use scale_info::TypeInfo;

    /// StakeManagerTrait 合约的存储结构体。
    #[ink(storage)]
    pub struct StakeManagerTrait {}

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

    /// 存款信息。
    #[derive(
        Debug, Clone, PartialEq, Eq, Hash, Encode, Decode, StorageLayout, TypeInfo, Default,
    )]
    pub struct DepositInfo {
        /// 实际存款金额。
        pub deposit: Balance,
        /// 是否已进行抵押。
        pub staked: bool,
        /// 为此实体抵押的实际以太币金额。
        pub stake: Balance,
        /// 抵押在可取回前需要的最短时间（秒）。
        pub unstake_delay_sec: Timestamp,
        /// 如果已锁定，则调用 `withdraw_stake` 的第一个块时间戳。如果未锁定，则为零。
        pub withdraw_time: Timestamp,
    }

    /// 用于 `get_stake_info` 和 `simulate_validation` 的 API 结构体。
    #[derive(Debug, Clone, PartialEq, Eq, Hash, Encode, Decode)]
    pub struct StakeInfo {
        /// 抵押的以太币金额。
        pub stake: Balance,
        /// 抵押在可取回前需要的延迟时间（秒）。
        pub unstake_delay_sec: Timestamp,
    }

    /// 用于管理存款和抵押的特性定义。
    #[ink::trait_definition]
    pub trait IStakeManager {
        /// 返回给定账户的存款信息。
        ///
        /// # Arguments
        ///
        /// * `account`：要获取存款信息的账户 ID。
        ///
        /// # Returns
        ///
        /// 给定账户的完整存款信息。
        #[ink(message)]
        fn get_deposit_info(&self, account: AccountId) -> DepositInfo;

        /// 返回给定账户的余额，用于支付燃气费。
        ///
        /// # Arguments
        ///
        /// * `account`：要获取余额的账户 ID。
        ///
        /// # Returns
        ///
        /// 给定账户的余额。
        #[ink(message)]
        fn balance_of(&self, account: AccountId) -> Balance;

        /// 向给定账户添加存款。
        ///
        /// # Arguments
        ///
        /// * `account`：要添加存款的账户 ID。
        #[ink(message, payable)]
        fn deposit_to(&mut self, account: AccountId);

        /// 向具有给定延迟的账户添加抵押。
        ///
        /// 任何待处理的取回操作将被取消。
        ///
        /// # Arguments
        ///
        /// * `unstake_delay_sec`：抵押在可取回前需要的新延迟时间（秒）。
        #[ink(message, payable)]
        fn add_stake(&mut self, unstake_delay_sec: Timestamp);

        /// 尝试取消抵押。
        ///
        /// 可以在取回延迟期结束后取回抵押金额。
        #[ink(message)]
        fn unlock_stake(&mut self);

        /// 从已取消抵押的抵押中取回金额。
        ///
        /// 必须先调用 `unlock_stake` 并等待取回延迟期结束。
        ///
        /// # Arguments
        ///
        /// * `withdraw_address`：要发送取回金额的地址。
        #[ink(message, payable)]
        fn withdraw_stake(&mut self, withdraw_address: AccountId);

        /// 从存款中取回金额。
        ///
        /// # Arguments
        ///
        /// * `withdraw_address`：要发送取回金额的地址。
        /// * `withdraw_amount`：要取回的金额。
        #[ink(message, payable)]
        fn withdraw_to(&mut self, withdraw_address: AccountId, withdraw_amount: Balance);
    }

    impl StakeManagerTrait {
        #[ink(constructor)]
        pub fn new() -> Self {
            Self {}
        }

        #[ink(message)]
        pub fn hello(&self) {}
    }
}
