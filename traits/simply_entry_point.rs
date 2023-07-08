use ink::env::Environment;

use crate::core::{env::AAEnvironment, exec::OpaqueTypes, user_operation::SimplyUserOperation};

#[ink::trait_definition]
pub trait ISimplyEntryPoint {
    /// 执行一批 UserOperation。
    /// 不使用签名聚合器。    
    /// 如果任何账户需要聚合器(即,在执行 simulateValidation 时返回了聚合器),则必须使用 handleAggregatedOps()。
    ///  
    /// - `ops` 要执行的操作    
    /// - `beneficiary` 用于接收费用的地址
    #[ink(message, payable)]
    fn handle_op(
        &self,
        op: SimplyUserOperation<AAEnvironment>,
        beneficiary: <AAEnvironment as Environment>::AccountId,
    ) -> Option<OpaqueTypes>;
}
