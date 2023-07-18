use crate::core::{env::AAEnvironment, helpers::Aggregator, user_operation::UserOperation};
use crate::traits::aggregator::IAggregator;
use crate::traits::stake_manager::{IStakeManager, StakeInfo};

use crate::core::error::Result;
use ink::codegen::TraitCallForwarder;
use ink::env::Environment;
use ink::prelude::vec::Vec;
use scale::{Decode, Encode};

use super::{account::IAccount, nonce_manager::INonceManager, paymaster::IPaymaster};

pub type AggregatorRef<E> = <<ink::reflect::TraitDefinitionRegistry<E> as IAggregator> ::__ink_TraitInfo as TraitCallForwarder>::Forwarder;
pub type PaymasterRef<E> = <<ink::reflect::TraitDefinitionRegistry<E> as IPaymaster> ::__ink_TraitInfo as TraitCallForwarder>::Forwarder;
pub type EntryPointRef<E> = <<ink::reflect::TraitDefinitionRegistry<E> as IEntryPoint> ::__ink_TraitInfo as TraitCallForwarder>::Forwarder;
pub type StakeManagerRef<E> = <<ink::reflect::TraitDefinitionRegistry<E> as IStakeManager> ::__ink_TraitInfo as TraitCallForwarder>::Forwarder;
pub type AccountRef<E> = <<ink::reflect::TraitDefinitionRegistry<E> as IAccount> ::__ink_TraitInfo as TraitCallForwarder>::Forwarder;
pub type NonceManagerRef<E> = <<ink::reflect::TraitDefinitionRegistry<E> as INonceManager> ::__ink_TraitInfo as TraitCallForwarder>::Forwarder;

/// 为每个聚合器处理的 UserOps     
#[derive(Encode, Decode)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
pub struct UserOpsPerAggregator<E: Environment = AAEnvironment> {
    /// 用户操作            
    pub user_ops: Vec<UserOperation<E>>,

    /// 聚合器地址      
    pub aggregator: Aggregator<E>,

    /// 聚合签名   
    pub signature: Vec<u8>,
}

/// 模拟过程中返回的 gas 和值
///
/// - `pre_op_gas` 验证时消耗的 gas(包括 preValidationGas)    
/// - `prefund` 所需预存款
/// - `sig_failed`  validateUserOp(或 paymaster) 的签名检查失败
/// - `valid_after` 第一个该 UserOp 有效的时间戳(合并 account 和 paymaster 的时间范围)
/// - `valid_until` 最后一个该 UserOp 有效的时间戳(合并 account 和 paymaster 的时间范围)
/// - `paymaster_context`  validatePaymasterUserOp 返回(用于传递给 postOp)
#[derive(Debug, PartialEq, Eq, Encode, Decode)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
pub struct ReturnInfo<E: Environment = AAEnvironment> {
    pub pre_op_gas: u64,

    pub prefund: u64,

    pub sig_failed: bool,

    pub valid_after: E::Timestamp,

    pub valid_until: E::Timestamp,

    pub paymaster_context: Vec<u8>,
}

#[derive(Encode, Decode, Clone, PartialEq, Eq, Debug)]
pub struct SenderAddressResult<E: Environment = AAEnvironment> {
    pub sender: E::AccountId,
}

/// 返回的聚合签名信息
///
/// - `aggregator` 账户返回的聚合器  
/// - `stake_info` 其当前质押  
#[derive(PartialEq, Eq, Encode, Decode)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
pub struct AggregatorStakeInfo<E: Environment = AAEnvironment> {
    pub aggregator: E::AccountId,

    pub stake_info: StakeInfo<E>,
}

impl<E: core::fmt::Debug> core::fmt::Debug for AggregatorStakeInfo<E>
where
    E: Environment,
    E::AccountId: core::fmt::Debug,
    StakeInfo<E>: core::fmt::Debug,
{
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        let AggregatorStakeInfo {
            aggregator,
            stake_info,
        } = self;
        f.debug_struct("AggregatorStakeInfo")
            .field("aggregator", &aggregator)
            .field("stake_info", &stake_info)
            .finish()
    }
}

#[ink::trait_definition]
pub trait IEntryPoint {
    /// 执行一批 UserOperation。
    /// 不使用签名聚合器。    
    /// 如果任何账户需要聚合器(即,在执行 simulateValidation 时返回了聚合器),则必须使用 handleAggregatedOps()。
    ///  
    /// - `ops` 要执行的操作    
    /// - `beneficiary` 用于接收费用的地址
    #[ink(message, payable)]
    fn handle_ops(
        &mut self,
        ops: Vec<UserOperation<AAEnvironment>>,
        beneficiary: <AAEnvironment as Environment>::AccountId,
    ) -> Result<()>;
    /// 使用聚合器执行一批 UserOperation
    ///
    /// - `ops_per_aggregator` 按聚合器分组的操作(或地址(0) 用于没有聚合器的账户)
    /// - `beneficiary` 用于接收费用的地址
    #[ink(message)]
    fn handle_aggregated_ops(
        &mut self,
        ops_per_aggregator: Vec<UserOpsPerAggregator<AAEnvironment>>,
        beneficiary: <AAEnvironment as Environment>::AccountId,
    ) -> Result<()>;
    /// 生成请求 ID  - 该请求的唯一标识符。   
    ///  请求 ID 是 userOp 的内容(除签名外)、入口点以及链 ID 的哈希。
    #[ink(message)]
    fn get_user_op_hash(&self, user_op: UserOperation<AAEnvironment>) -> [u8; 32];
}
