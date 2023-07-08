use ink::env::Environment;

use crate::traits::{
    entry_point::{AggregatorStakeInfo, ReturnInfo},
    stake_manager::StakeInfo,
};

use super::env::AAEnvironment;

#[derive(Debug, PartialEq, Eq, scale::Encode, scale::Decode)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
pub enum Error<E: Environment = AAEnvironment> {
    NotOwner,
    NotFromEntryPoint,
    PaymasterDepositTooLow,
    PaymasterNotFound,
    TooLittleVerificationGas,
    InvalidAccountNonce,
    InvalidPaymasterAddress,
    OverVerificationGasLimit,
    GasValuesOverflow,
    FailedSendToBeneficiary,
    InvalidBeneficiary,
    NoStakeSpecified,
    CannotDecreaseUnstakeTime,
    MustSpecifyUnstakeDelay,
    AlreadyUnstaking,
    NotStaked,
    FailedToWithdrawStake,
    StakeWithdrawalIsNotDue,
    MustCallUnlockStakeFirst,
    NoStakeToWithdraw,
    FailedToWithdraw,
    WithdrawAmountTooLarge,
    DepositOverflow,

    /// Returned if not enough balance to fulfill a request is available.
    InsufficientBalance,
    /// Returned if not enough allowance to fulfill a request is available.
    InsufficientAllowance,
    OnlyInternalCall,
    OutOfGas,
    /// handleOps 调用失败产生的错误,用于识别失败的操作。
    ///  若 simulateValidation 成功通过,则 handleOps 不存在失败的可能。
    ///   
    /// - `op_index` - 失败操作在数组中的索引(在 simulateValidation 中总是为 0)
    /// - `reason` - 失败原因
    ///   字符串以 "AAmn" 开头,其中 "m" 代表分类:  
    ///      1 - fabric 失败
    ///      2 - account 失败
    ///      3 - paymaster 失败
    ///    这样则可以归类到正确的实体。
    ///    
    /// 应该在宕机下的 handleOps 模拟中捕获,不应该在链上产生。
    /// 有助于防范批处理器或 paymaster/factory/account回滚攻击,或用于故障排除。
    FailedOp {
        op_index: u64,
        reason: String,
    },
    /// 签名聚合器无法验证它生成的聚合签名时的错误情况。     
    SignatureValidationFailed {
        aggregator: E::AccountId,
    },

    /// simulateValidation 的成功结果。
    ///   
    /// - `return_info` 返回值(gas 和时间范围)
    /// - `sender_info` 发送者的质押信息
    /// - `factory_info` 工厂的质押信息(如果有)
    /// - `paymaster_info` 交付方的质押信息(如果有)
    ValidationResult {
        return_info: ReturnInfo<E>,
        sender_info: StakeInfo<E>,
        factory_info: StakeInfo<E>,
        paymaster_info: StakeInfo<E>,
    },

    /// simulateValidation 的成功结果,如果用户返回了一个签名聚合器
    ///     
    /// - `return_info` 返回值(gas 和时间范围)
    /// - `sender_info`  发送者的质押信息
    /// - `factory_info` 工厂的质押信息(如果有)   
    /// - `paymaster_info`  交付方的质押信息(如果有)
    /// - `aggregator_info`  签名聚合信息(如果用户需要签名聚合器)    
    ///      捆绑器必须使用它验证签名,否则拒绝 UserOperation。
    ValidationResultWithAggregation {
        return_info: ReturnInfo<E>,
        sender_info: StakeInfo<E>,
        factory_info: StakeInfo<E>,
        paymaster_info: StakeInfo<E>,
        aggregator_info: AggregatorStakeInfo<E>,
    },

    /// getSenderAddress 的返回值    
    SenderAddressResult {
        sender: E::AccountId,
    },

    /// simulateHandleOp 的返回值        
    ExecutionResult {
        pre_op_gas: E::Balance,
        paid: E::Balance,
        valid_after: E::Timestamp,
        valid_until: E::Timestamp,
        target_success: bool,
        target_result: Vec<u8>,
    },
}

pub type Result<T> = core::result::Result<T, Error<AAEnvironment>>;
