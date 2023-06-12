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

pub type EnvUserOperation<E> = UserOperation<
    <E as Environment>::AccountId,
    <E as Environment>::Hash,
    <E as Environment>::Balance,
>;

#[derive(scale::Encode, scale::Decode)]
pub struct UserOperation<AccountId, Hash, Balance> {
    pub sender: AccountId,
    pub nonce: Hash,
    pub init_code: Vec<u8>,
    pub call_data: Vec<u8>,
    pub call_gas_limit: Balance,
    pub verification_gas_limit: Balance,
    pub pre_verification_gas: Balance,
    pub max_fee_per_gas: Balance,
    pub max_priority_fee_per_gas: Balance,
    pub paymaster_and_data: Vec<u8>,
    pub signature: Vec<u8>,
}

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

#[derive(Debug, Clone, Copy)]
pub struct UserOperationLib<E: Environment>(PhantomData<E>);

impl<E: Environment> UserOperationLib<E> {
    pub fn gas_price(user_op: &EnvUserOperation<E>) -> E::Balance {
        let max_fee_per_gas = user_op.max_fee_per_gas;

        let max_priority_fee_per_gas = user_op.max_priority_fee_per_gas;

        if max_fee_per_gas == max_priority_fee_per_gas {
            max_fee_per_gas
        } else {
            max_fee_per_gas.min(
                max_priority_fee_per_gas, // TODO: + basefee
            )
        }
    }

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
    pub fn hash(user_op: &EnvUserOperation<E>) -> Hash {
        keccak256(&Self::pack(user_op))
    }
}

fn keccak256(input: &[u8]) -> Hash {
    let mut hash = [0u8; 32];
    Keccak256::hash(input, &mut hash);
    Hash::from(hash)
}
