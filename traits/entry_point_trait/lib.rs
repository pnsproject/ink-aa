#![cfg_attr(not(feature = "std"), no_std, no_main)]

#[ink::contract]
mod entry_point_trait {
    use scale::{Decode, Encode};
    use stake_manager_trait::StakeInfo;
    use user_operation::{EnvUserOperation, UserOperation};

    #[ink(storage)]
    pub struct EntryPointTrait {}

    impl EntryPointTrait {
        #[ink(constructor)]
        pub fn new() -> Self {
            Self {}
        }
        #[ink(message)]
        pub fn hello(&self) {}
    }

    /// 每个成功请求之后发出的事件
    #[ink(event)]
    pub struct UserOperationEvent {
        /// 请求的唯一标识符（哈希其整个内容，除了签名）。
        #[ink(topic)]
        pub user_op_hash: [u8; 32],
        /// 生成此请求的帐户。
        #[ink(topic)]
        pub sender: AccountId,
        /// 如果非空，则为支付此请求的支付账户。
        #[ink(topic)]
        pub paymaster: AccountId,
        /// 请求中使用的nonce。
        pub nonce: Hash,
        /// 如果发送方的事务成功，则为true，反之为false。
        pub success: bool,
        /// 此UserOperation的实际付款金额（由帐户或支付账户支付）。
        pub actual_gas_cost: Balance,
        /// 此UserOperation使用的总气体量（包括preVerification、creation、validation和execution）。
        pub actual_gas_used: Balance,
    }

    /// 账户 "sender" 被部署。
    #[ink(event)]
    pub struct AccountDeployed {
        /// 部署此账户的userOp。将跟随UserOperationEvent。
        #[ink(topic)]
        pub user_op_hash: [u8; 32],
        /// 被部署的账户
        #[ink(topic)]
        pub sender: AccountId,
        /// 用于部署此账户的工厂（在 initCode 中）
        pub factory: AccountId,
        /// 此 UserOp 所使用的支付账户
        pub paymaster: AccountId,
    }

    /// 如果 UserOperation "callData" 返回非零长度，则发出的事件
    #[ink(event)]
    pub struct UserOperationRevertReason {
        /// 请求的唯一标识符。
        #[ink(topic)]
        pub user_op_hash: [u8; 32],
        /// 此请求的发送方
        #[ink(topic)]
        pub sender: AccountId,
        /// 请求中使用的nonce
        pub nonce: Hash,
        /// "callData" 的（已还原的）调用返回字节。
        pub revert_reason: Vec<u8>,
    }

    /// 在执行循环之前由 handleOps() 发出的事件。
    /// 在此事件之前发出的任何事件都属于验证。
    #[ink(event)]
    pub struct BeforeExecution {}

    /// 在此包中使用的签名聚合器。
    #[ink(event)]
    pub struct SignatureAggregatorChanged {
        /// 签名聚合器
        #[ink(topic)]
        pub aggregator: AccountId,
    }

    #[derive(Encode, Decode, Debug, PartialEq, Eq, Clone)]
    pub enum Error {
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
        FailedOp { op_index: u64, reason: String },
        /// 签名聚合器无法验证它生成的聚合签名时的错误情况。     
        SignatureValidationFailed { aggregator: AccountId },

        /// simulateValidation 的成功结果。
        ///   
        /// - `return_info` 返回值(gas 和时间范围)
        /// - `sender_info` 发送者的质押信息
        /// - `factory_info` 工厂的质押信息(如果有)
        /// - `paymaster_info` 交付方的质押信息(如果有)
        ValidationResult {
            return_info: ReturnInfo,
            sender_info: StakeInfo,
            factory_info: StakeInfo,
            paymaster_info: StakeInfo,
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
            return_info: ReturnInfo,
            sender_info: StakeInfo,
            factory_info: StakeInfo,
            paymaster_info: StakeInfo,
            aggregator_info: AggregatorStakeInfo,
        },

        /// getSenderAddress 的返回值    
        SenderAddressResult { sender: AccountId },

        /// simulateHandleOp 的返回值        
        ExecutionResult {
            pre_op_gas: Balance,
            paid: Balance,
            valid_after: Timestamp,
            valid_until: Timestamp,
            target_success: bool,
            target_result: Vec<u8>,
        },
    }

    /// 为每个聚合器处理的 UserOps     
    #[derive(Debug, Clone, Encode, Decode)]
    pub struct UserOpsPerAggregator {
        /// 用户操作            
        pub user_ops: UserOperation<AccountId, Hash, Balance>,

        /// 聚合器地址      
        pub aggregator: ink::contract_ref!(aggregator_trait::IAggregator),

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
    #[derive(Debug, Clone, Encode, Decode, PartialEq, Eq)]
    pub struct ReturnInfo {
        pub pre_op_gas: Balance,

        pub prefund: Balance,

        pub sig_failed: bool,

        pub valid_after: Timestamp,

        pub valid_until: Timestamp,

        pub paymaster_context: Vec<u8>,
    }

    /// 返回的聚合签名信息
    ///
    /// - `aggregator` 账户返回的聚合器  
    /// - `stake_info` 其当前质押  
    #[derive(Debug, Clone, Encode, Decode, PartialEq, Eq)]
    pub struct AggregatorStakeInfo {
        pub aggregator: AccountId,

        pub stake_info: StakeInfo,
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
        fn handle_ops(&self, ops: Vec<EnvUserOperation<Self::Env>>, beneficiary: AccountId);
        /// 使用聚合器执行一批 UserOperation     
        ///  
        /// - `ops_per_aggregator` 按聚合器分组的操作(或地址(0) 用于没有聚合器的账户)
        /// - `beneficiary` 用于接收费用的地址
        #[ink(message)]
        fn handle_aggregated_ops(
            &self,
            ops_per_aggregator: Vec<UserOpsPerAggregator>,
            beneficiary: AccountId,
        );
        /// 生成请求 ID  - 该请求的唯一标识符。   
        ///  请求 ID 是 userOp 的内容(除签名外)、入口点以及链 ID 的哈希。
        #[ink(message)]
        fn get_user_op_hash(&self, user_op: EnvUserOperation<Self::Env>) -> [u8; 32];
        /// 模拟 account.validateUserOp 和 paymaster.validatePaymasterUserOp 的调用。    
        /// @dev 此方法总是回滚,成功结果为 ValidationResult 错误。其他错误为失败。
        /// @dev 节点还必须验证它是否使用了禁用的操作码,并且它没有引用账户数据外部的存储。    
        ///  
        /// - `userOp` 要验证的用户操作
        #[ink(message)]
        fn simulate_validation(&self, user_op: EnvUserOperation<Self::Env>);
    }
}
