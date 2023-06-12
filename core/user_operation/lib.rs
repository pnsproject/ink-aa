#![cfg_attr(not(feature = "std"), no_std, no_main)]

use core::marker::PhantomData;

use ink::{
    env::{
        hash::{CryptoHash, Keccak256},
        DefaultEnvironment, Environment,
    },
    primitives::Hash,
};
use scale::Encode;

/// `EnvUserOperation` 是一个类型别名，表示一个环境中的用户操作。
pub type EnvUserOperation<E> = UserOperation<
    <E as Environment>::AccountId,
    <E as Environment>::Hash,
    <E as Environment>::Balance,
>;

/// `UserOperation` 结构体定义了一个用户操作。
#[derive(scale::Encode, scale::Decode, Debug, Clone, PartialEq, Eq, Hash)]
pub struct UserOperation<AccountId, Hash, Balance> {
    /// 发送人的账户 ID。
    pub sender: AccountId,
    /// 用户操作的随机数。
    pub nonce: Hash,
    /// 要在合约上创建的代码的字节数组。
    pub init_code: Vec<u8>,
    /// 要调用的方法的字节数组。
    pub call_data: Vec<u8>,
    /// 调用此用户操作时可用的燃料量。
    pub call_gas_limit: Balance,
    /// 用于验证此用户操作的燃料量。
    pub verification_gas_limit: Balance,
    /// 在验证之前执行的燃料量。
    pub pre_verification_gas: Balance,
    /// 最高可支付的燃料价格。
    pub max_fee_per_gas: Balance,
    /// 最高优先级燃料价格。
    pub max_priority_fee_per_gas: Balance,
    /// 付款人和数据的哈希值。
    pub paymaster_and_data: Vec<u8>,
    /// 用户操作的签名。
    pub signature: Vec<u8>,
}

/// `UserOperationPack` 结构体定义了一个打包了用户操作的结构体。
#[derive(scale::Encode, scale::Decode)]
struct UserOperationPack<E: Environment = DefaultEnvironment> {
    sender: E::AccountId,
    nonce: E::Hash,
    init_code: Hash,
    call_data: Hash,
    call_gas_limit: E::Balance,
    verification_gas_limit: E::Balance,
    pre_verification_gas: E::Balance,
    max_fee_per_gas: E::Balance,
    max_priority_fee_per_gas: E::Balance,
    paymaster_and_data: Hash,
}

/// `UserOperationLib` 结构体定义了一些有用的方法来操作 `UserOperation` 和 `EnvUserOperation` 结构体。
#[derive(Debug, Clone, Copy)]
pub struct UserOperationLib<E: Environment>(PhantomData<E>);

impl<E: Environment> UserOperationLib<E> {
    /// 计算用户操作的燃料价格。
    pub fn gas_price(user_op: &EnvUserOperation<E>) -> E::Balance {
        let max_fee_per_gas = user_op.max_fee_per_gas;
        let max_priority_fee_per_gas = user_op.max_priority_fee_per_gas;

        if max_fee_per_gas == max_priority_fee_per_gas {
            max_fee_per_gas
        } else {
            max_fee_per_gas.min(max_priority_fee_per_gas) // TODO: + basefee
        }
    }

    /// 打包一个 `EnvUserOperation` 为字节数组。
    pub fn pack(user_op: &EnvUserOperation<E>) -> Vec<u8> {
        UserOperationPack::<E> {
            sender: user_op.sender.clone(),
            nonce: user_op.nonce,
            init_code: keccak256(&user_op.init_code),
            call_data: keccak256(&user_op.call_data),
            call_gas_limit: user_op.call_gas_limit,
            verification_gas_limit: user_op.verification_gas_limit,
            pre_verification_gas: user_op.pre_verification_gas,
            max_fee_per_gas: user_op.max_fee_per_gas,
            max_priority_fee_per_gas: user_op.max_priority_fee_per_gas,
            paymaster_and_data: keccak256(&user_op.paymaster_and_data),
        }
        .encode()
    }

    /// 计算一个 `EnvUserOperation` 的哈希值。
    pub fn hash(user_op: &EnvUserOperation<E>) -> Hash {
        keccak256(&Self::pack(user_op))
    }
}

/// 计算一个字节数组的 Keccak256 哈希值。
fn keccak256(input: &[u8]) -> Hash {
    let mut hash = [0u8; 32];
    Keccak256::hash(input, &mut hash);
    Hash::from(hash)
}
