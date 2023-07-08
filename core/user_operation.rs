use ink::{
    env::Environment,
    primitives::{AccountId, Hash},
};
use scale::Encode;

use super::{
    env::AAEnvironment,
    helpers::{keccak256, keccak256_hash},
};

/// `UserOperation` 结构体定义了一个用户操作。
#[derive(scale::Encode, scale::Decode, Clone, Hash, Debug)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
pub struct UserOperation<E: Environment = AAEnvironment> {
    /// 发送人的账户 ID。
    pub sender: E::AccountId,
    /// 用户操作的随机数。
    pub nonce: [u8; 32],
    /// 要在合约上创建的代码的字节数组。
    pub init_code: Vec<u8>,
    /// 要调用的合约地址。
    pub callee: E::AccountId,
    /// 要调用的合约方法。
    pub selector: [u8; 4],
    /// 要调用的合约参数。
    pub call_data: Vec<u8>,
    /// 调用此用户操作时可用的燃料量。
    pub call_gas_limit: u64,
    /// 用于验证此用户操作的燃料量。
    pub verification_gas_limit: u64,
    /// 在验证之前执行的燃料量。
    pub pre_verification_gas: u64,
    /// 最高可支付的燃料价格。
    pub max_fee_per_gas: u64,
    /// 最高优先级燃料价格。
    pub max_priority_fee_per_gas: u64,
    /// 付款人的地址。
    pub paymaster: E::AccountId,
    //  付款人的数据。
    pub paymaster_data: Vec<u8>,
    /// 用户操作的签名。
    pub signature: Vec<u8>,
}

impl Default for UserOperation<AAEnvironment> {
    fn default() -> Self {
        Self {
            sender: AccountId::from([0; 32]),
            nonce: Default::default(),
            init_code: Default::default(),
            callee: AccountId::from([0; 32]),
            selector: Default::default(),
            call_data: Default::default(),
            call_gas_limit: Default::default(),
            verification_gas_limit: Default::default(),
            pre_verification_gas: Default::default(),
            max_fee_per_gas: Default::default(),
            max_priority_fee_per_gas: Default::default(),
            paymaster: AccountId::from([0; 32]),
            paymaster_data: Default::default(),
            signature: Default::default(),
        }
    }
}

impl<E: Environment> UserOperation<E> {
    /// 计算用户操作的燃料价格。
    pub fn gas_price(&self) -> u64 {
        let max_fee_per_gas = self.max_fee_per_gas;
        let max_priority_fee_per_gas = self.max_priority_fee_per_gas;

        if max_fee_per_gas == max_priority_fee_per_gas {
            max_fee_per_gas
        } else {
            max_fee_per_gas.min(max_priority_fee_per_gas) // TODO: + basefee
        }
    }

    /// 打包一个 `EnvUserOperation` 为字节数组。
    pub fn pack(&self) -> Vec<u8> {
        UserOperationPack::<E> {
            sender: self.sender.clone(),
            nonce: self.nonce,
            init_code: keccak256_hash(&self.init_code),
            call_data: keccak256_hash(&self.call_data),
            call_gas_limit: self.call_gas_limit,
            verification_gas_limit: self.verification_gas_limit,
            pre_verification_gas: self.pre_verification_gas,
            max_fee_per_gas: self.max_fee_per_gas,
            max_priority_fee_per_gas: self.max_priority_fee_per_gas,
            paymaster_and_data: {
                let mut data = self.paymaster.encode();
                data.extend(&self.paymaster_data);
                keccak256_hash(&data)
            },
        }
        .encode()
    }

    /// 计算一个 `EnvUserOperation` 的哈希值。
    pub fn hash(&self) -> [u8; 32] {
        keccak256(&Self::pack(self))
    }
}

/// `UserOperationPack` 结构体定义了一个打包了用户操作的结构体。
#[derive(scale::Encode, scale::Decode)]
struct UserOperationPack<E: Environment> {
    sender: E::AccountId,
    nonce: [u8; 32],
    init_code: Hash,
    call_data: Hash,
    call_gas_limit: u64,
    verification_gas_limit: u64,
    pre_verification_gas: u64,
    max_fee_per_gas: u64,
    max_priority_fee_per_gas: u64,
    paymaster_and_data: Hash,
}
