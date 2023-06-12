#![cfg_attr(not(feature = "std"), no_std, no_main)]

use ink::{
    env::{
        hash::{CryptoHash, Keccak256},
        DefaultEnvironment, Environment,
    },
    primitives::Hash,
};
use scale::Encode;

#[derive(scale::Encode, scale::Decode)]
pub struct UserOperation<E: Environment = DefaultEnvironment> {
    pub sender: E::AccountId,
    pub nonce: E::Hash,
    pub init_code: Vec<u8>,
    pub call_data: Vec<u8>,
    pub call_gas_limit: E::Balance,
    pub verification_gas_limit: E::Balance,
    pub pre_verification_gas: E::Balance,
    pub max_fee_per_gas: E::Balance,
    pub max_priority_fee_per_gas: E::Balance,
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

impl<E> UserOperation<E>
where
    E: Environment,
{
    pub fn get_sender(&self) -> &E::AccountId {
        &self.sender
    }
    pub fn gas_price(&self) -> E::Balance {
        let max_fee_per_gas = self.max_fee_per_gas;

        let max_priority_fee_per_gas = self.max_priority_fee_per_gas;

        if max_fee_per_gas == max_priority_fee_per_gas {
            max_fee_per_gas
        } else {
            max_fee_per_gas.min(
                max_priority_fee_per_gas, // TODO: + basefee
            )
        }
    }

    pub fn pack(&self) -> Vec<u8> {
        UserOperationPack::<E> {
            sender: self.sender.clone(),
            nonce: self.nonce,
            init_code: keccak256(&self.init_code),
            call_data: keccak256(&self.call_data),
            call_gas_limit: self.call_gas_limit,
            verification_gas_limit: self.verification_gas_limit,
            pre_verification_gas: self.pre_verification_gas,
            max_fee_per_gas: self.max_fee_per_gas,
            max_priority_fee_per_gas: self.max_priority_fee_per_gas,
            paymaster_and_data: keccak256(&self.paymaster_and_data),
        }
        .encode()
    }

    pub fn hash(&self) -> Hash {
        keccak256(&self.pack())
    }
}

fn keccak256(input: &[u8]) -> Hash {
    let mut hash = [0u8; 32];
    Keccak256::hash(input, &mut hash);
    Hash::from(hash)
}
