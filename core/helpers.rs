use ink::env::{
    hash::{CryptoHash, Keccak256},
    Environment,
};
use ink::primitives::Hash;
use scale::{Decode, Encode};

use super::env::AAEnvironment;

/// 返回从 validateUserOp 获得的数据。
///
/// validateUserOp 返回一个 uint256，它由 `_packedValidationData` 创建并由 `_parseValidationData` 解析。
///
/// # Arguments
///
/// * `aggregator` - 聚合器地址，用于验证签名。
/// * `valid_after` - 此 UserOp 的有效开始时间戳。
/// * `valid_until` - 此 UserOp 的有效截止时间戳。
#[derive(Clone, Encode, Decode)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
pub struct ValidationData<E: Environment = AAEnvironment> {
    pub aggregator: Aggregator<E>,
    pub valid_after: E::Timestamp,
    pub valid_until: E::Timestamp,
}

impl<E: Environment> Default for ValidationData<E> {
    fn default() -> Self {
        use num_traits::identities::Zero;
        Self {
            aggregator: Default::default(),
            valid_after: E::Timestamp::zero(),
            valid_until: E::Timestamp::zero(),
        }
    }
}

#[cfg(feature = "std")]
impl<E: core::fmt::Debug> core::fmt::Debug for ValidationData<E>
where
    E: Environment,
    E::Timestamp: core::fmt::Debug,
    E::AccountId: core::fmt::Debug,
{
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        let ValidationData {
            aggregator,
            valid_after,
            valid_until,
        } = self;
        f.debug_struct("ValidationData")
            .field("aggregator", &aggregator)
            .field("valid_after", &valid_after)
            .field("valid_until", &valid_until)
            .finish()
    }
}

/// 如果为 `address(0)`，则表示账户自己验证了签名；如果为 `address(1)`，则表示账户未能验证签名。
#[derive(Clone, Encode, Decode, Eq, PartialEq)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo, core::fmt::Debug))]
pub enum Aggregator<E: Environment = AAEnvironment> {
    VerifiedBySelf,
    VerifiedBy(E::AccountId),
    FailedVerification,
}

impl<E: Environment> Default for Aggregator<E> {
    fn default() -> Self {
        Self::VerifiedBySelf
    }
}

// 相交账户和支付主管的时间范围。
pub fn intersect_time_range<E: Environment>(
    account_validation_data: ValidationData<E>,
    paymaster_validation_data: ValidationData<E>,
) -> ValidationData<E> {
    let aggregator = if let Aggregator::<E>::VerifiedBySelf = account_validation_data.aggregator {
        paymaster_validation_data.aggregator
    } else {
        account_validation_data.aggregator
    };
    let valid_after = account_validation_data
        .valid_after
        .max(paymaster_validation_data.valid_after);
    let valid_until = account_validation_data
        .valid_until
        .min(paymaster_validation_data.valid_until);
    ValidationData {
        aggregator,
        valid_after,
        valid_until,
    }
}

/// 计算一个字节数组的 Keccak256 哈希值。
#[inline]
pub fn keccak256(input: &[u8]) -> [u8; 32] {
    let mut hash = [0u8; 32];
    Keccak256::hash(input, &mut hash);
    hash
}

/// 计算一个字节数组的 Keccak256 哈希值。
pub fn keccak256_hash(input: &[u8]) -> Hash {
    Hash::from(keccak256(input))
}
