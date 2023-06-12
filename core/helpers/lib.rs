#![cfg_attr(not(feature = "std"), no_std)]

use ink::env::Environment;
use scale::{Decode, Encode};

/// 返回从 validateUserOp 获得的数据。
///
/// validateUserOp 返回一个 uint256，它由 `_packedValidationData` 创建并由 `_parseValidationData` 解析。
///
/// # Arguments
///
/// * `aggregator` - 聚合器地址，用于验证签名。如果为 `address(0)`，则表示账户自己验证了签名；如果为 `address(1)`，则表示账户未能验证签名。
/// * `valid_after` - 此 UserOp 的有效开始时间戳。
/// * `valid_until` - 此 UserOp 的有效截止时间戳。
#[derive(Clone, Copy, Encode, Decode)]
pub struct ValidationData<Account> {
    pub aggregator: Account,
    pub valid_after: u64,
    pub valid_until: u64,
}

// 相交账户和支付主管的时间范围。
pub fn intersect_time_range<E: Environment>(
    account_validation_data: ValidationData<E::AccountId>,
    paymaster_validation_data: ValidationData<E::AccountId>,
) -> ValidationData<E::AccountId> {
    let aggregator = if account_validation_data
        .aggregator
        .as_ref()
        .iter()
        .all(|&s| s == 0)
    {
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
