#![cfg_attr(not(feature = "std"), no_std)]

use ink::env::Environment;

/**
 * 返回从validateUserOp获得的数据。  
 * validateUserOp 返回一个uint256,它由`_packedValidationData`创建并由`_parseValidationData`解析  
 * @param aggregator - address(0) - 账户自己验证了签名。  
 *              address(1) - 账户未能验证签名。  
 *              否则 - 这是一个签名聚合器的地址,必须用于验证签名。  
 * @param validAfter - 只有在此时间戳之后,此UserOp才有效。  
 * @param validaUntil - 只有在此时间戳之前,此UserOp才有效。   
 */
#[derive(Clone, Copy, Debug)]
pub struct ValidationData<E: Environment> {
    pub aggregator: E::AccountId,
    pub valid_after: u64,
    pub valid_until: u64,
}

// 相交账户和支付主管的时间范围。
pub fn intersect_time_range<E: Environment>(
    account_validation_data: ValidationData<E>,
    paymaster_validation_data: ValidationData<E>,
) -> ValidationData<E> {
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
