use ink::{
    env::Environment,
    primitives::{AccountId, Hash},
};
use scale::Encode;

use crate::traits::paymaster;

use super::{env::AAEnvironment, helpers::keccak256};
use ink::prelude::vec::Vec;

/// `UserOperation` 结构体定义了一个用户操作。
#[derive(scale::Encode, scale::Decode, Clone, Hash, Debug)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
pub struct UserOperation<E: Environment = AAEnvironment> {
    /// 发送人的账户 ID。
    /// 必须是钱包，即实现了IAccount接口的合约地址
    pub sender: E::AccountId,
    /// 用户操作的随机数。
    /// 目前不影响最终结果
    pub nonce: [u8; 32],
    /// 要在合约上创建的代码的字节数组。
    /// 目前不影响最终结果
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
    /// 付款人的地址和数据。
    pub paymaster_and_data: PaymasterAndData<E>,
    /// 用户操作的签名。
    /// 目前不影响最终结果
    pub signature: Vec<u8>,
}

#[derive(scale::Encode, scale::Decode, Clone, Hash, Debug)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
pub enum PaymasterAndData<E: Environment> {
    OnlyPaymaster(E::AccountId),
    PaymasterAndData {
        paymaster: E::AccountId,
        data: Vec<u8>,
    },
}

impl<E: Environment> PaymasterAndData<E> {
    pub fn is_eq_zero(&self) -> bool {
        self.paymaster_ref().as_ref().iter().all(|n| 0.eq(n))
    }
    pub fn paymaster_ref(&self) -> &E::AccountId {
        match self {
            PaymasterAndData::OnlyPaymaster(paymaster) => paymaster,
            PaymasterAndData::PaymasterAndData { paymaster, .. } => paymaster,
        }
    }

    pub fn paymaster(&self) -> E::AccountId {
        self.paymaster_ref().clone()
    }

    pub fn paymaster_take(self) -> E::AccountId {
        match self {
            PaymasterAndData::OnlyPaymaster(paymaster) => paymaster,
            PaymasterAndData::PaymasterAndData { paymaster, .. } => paymaster,
        }
    }

    pub fn hash(&self) -> [u8; 32] {
        let data = match self {
            PaymasterAndData::OnlyPaymaster(paymaster) => paymaster.encode(),
            PaymasterAndData::PaymasterAndData { paymaster, data } => {
                let mut res = paymaster.encode();
                res.extend(data);
                res
            }
        };
        keccak256(&data)
    }
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
            paymaster_and_data: PaymasterAndData::OnlyPaymaster(AccountId::from([0; 32])),
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
            init_code: keccak256(&self.init_code).into(),
            call_data: keccak256(&self.call_data).into(),
            call_gas_limit: self.call_gas_limit,
            verification_gas_limit: self.verification_gas_limit,
            pre_verification_gas: self.pre_verification_gas,
            max_fee_per_gas: self.max_fee_per_gas,
            max_priority_fee_per_gas: self.max_priority_fee_per_gas,
            paymaster_and_data: self.paymaster_and_data.hash().into(),
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
