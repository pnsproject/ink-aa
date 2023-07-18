#![cfg_attr(not(feature = "std"), no_std, no_main)]

pub use entry_point::EntryPointRef;

#[ink::contract(env = ink_aa::core::env::AAEnvironment)]
mod entry_point {
    use ink::prelude::{format, vec::Vec};

    use ink_aa::{
        core::{
            env::AAEnvironment,
            error::{Error, Result},
            exec::{OpaqueTypes, Transaction},
            helpers::{Aggregator, ValidationData},
            user_operation::UserOperation,
        },
        traits::{
            entry_point::{AggregatorRef, IEntryPoint, PaymasterRef, UserOpsPerAggregator},
            nonce_manager::INonceManager,
            paymaster::{IPaymaster, PostOpMode},
            stake_manager::{DepositInfo, IStakeManager},
        },
    };

    #[ink(storage)]
    pub struct EntryPoint {
        stake_manager: stake_manager::StakeManagerRef,
        nonce_manager: nonce_manager::NonceManagerRef,
    }

    // TODO：等`event2.0`合并发布之后，转移到`traits`下
    #[ink(event)]
    pub struct UserOperationReturnValue {
        #[ink(topic)]
        pub user_op_hash: Hash,
        #[ink(topic)]
        pub success: bool,
        #[ink(topic)]
        pub result: OpaqueTypes,
    }

    /// 每个成功请求之后发出的事件
    #[ink(event)]
    pub struct UserOperationEvent {
        /// 请求的唯一标识符（哈希其整个内容，除了签名）。
        #[ink(topic)]
        pub user_op_hash: Hash,
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
        pub actual_gas_cost: u64,
        /// 此UserOperation使用的总气体量（包括preVerification、creation、validation和execution）。
        pub actual_gas_used: u64,
    }

    /// 账户 "sender" 被部署。
    #[ink(event)]
    pub struct AccountDeployed {
        /// 部署此账户的userOp。将跟随UserOperationEvent。
        #[ink(topic)]
        pub user_op_hash: Hash,
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
        pub user_op_hash: Hash,
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

    impl EntryPoint {
        #[ink(constructor)]
        pub fn new(
            version: u32,
            stake_manager_code_hash: Hash,
            nonce_manager_code_hash: Hash,
        ) -> Self {
            // let total_balance = Self::env().balance();
            let salt = version.to_le_bytes();
            let stake_manager = stake_manager::StakeManagerRef::new()
                .endowment(0)
                .code_hash(stake_manager_code_hash)
                .salt_bytes(salt)
                .instantiate();
            let nonce_manager = nonce_manager::NonceManagerRef::new()
                .endowment(0)
                .code_hash(nonce_manager_code_hash)
                .salt_bytes(salt)
                .instantiate();

            Self {
                stake_manager,
                nonce_manager,
            }
        }
    }

    #[derive(Clone, Default)]
    struct UserOpInfo {
        user_op: UserOperation<AAEnvironment>,
        user_op_hash: [u8; 32],
        prefund: u64,
        context: Vec<u8>,
        pre_op_gas: u64,
    }

    #[ink(impl)]
    impl EntryPoint {
        /**
         * compensate the caller's beneficiary address with the collected fees of all UserOperations.
         * @param beneficiary the address to receive the fees
         * @param amount amount to transfer.
         */
        fn compensate(&self, beneficiary: AccountId, amount: Balance) -> Result<()> {
            if beneficiary == AccountId::from([0x0; 32]) {
                return Err(Error::InvalidBeneficiary);
            }
            self.env()
                .transfer(beneficiary, amount)
                .map_err(|_| Error::FailedSendToBeneficiary)?;

            Ok(())
        }

        /**
         * the gas price this UserOp agrees to pay.
         * relayer/block builder might submit the TX with higher priorityFee, but the user should not
         */
        fn get_user_op_gas_price(&self, user_op: &UserOperation<AAEnvironment>) -> u64 {
            let max_fee_per_gas = user_op.max_fee_per_gas;
            let max_priority_fee_per_gas = user_op.max_priority_fee_per_gas;
            if max_fee_per_gas == max_priority_fee_per_gas {
                // legacy mode (for networks that don't support basefee opcode)
                return max_fee_per_gas;
            }
            return max_fee_per_gas.min(
                max_priority_fee_per_gas, // + self.env().base_fee()
            );
        }

        /**
         * process post-operation.
         * called just after the callData is executed.
         * if a paymaster is defined and its validation returned a non-empty context, its postOp is called.
         * the excess amount is refunded to the account (or paymaster - if it was used in the request)
         * @param opIndex index in the batch
         * @param mode - whether is called from innerHandleOp, or outside (postOpReverted)
         * @param opInfo userOp fields and info collected during validation
         * @param context the context returned in validatePaymasterUserOp
         * @param actualGas the gas used so far by this user operation
         */
        fn handle_post_op(
            &mut self,
            op_index: u64,
            mode: PostOpMode,
            op_info: &UserOpInfo,
            context: &Vec<u8>,
            mut actual_gas: u64,
        ) -> Result<u64> {
            let pre_gas = self.env().gas_left();
            let user_op = &op_info.user_op;
            let gas_price = self.get_user_op_gas_price(&user_op);

            let paymaster: PaymasterRef<AAEnvironment> =
                user_op.paymaster_and_data.paymaster().into();
            let refund_address;
            if user_op
                .paymaster_and_data
                .paymaster_ref()
                .eq(&AccountId::from([0x0; 32]))
            {
                refund_address = user_op.sender;
            } else {
                refund_address = user_op.paymaster_and_data.paymaster();
                if !context.is_empty() {
                    let actual_gas_cost = (actual_gas as Balance)
                        .checked_mul(gas_price as Balance)
                        .ok_or(Error::GasValuesOverflow)?;

                    if let Err(e) = paymaster.post_op(mode, context.clone(), actual_gas_cost) {
                        return Err(Error::FailedOp {
                            op_index,
                            reason: format!("AA50 postOp reverted: {e:?}"),
                        });
                    }
                }
            }
            actual_gas = pre_gas
                .checked_sub(self.env().gas_left())
                .and_then(|pre| pre.checked_add(actual_gas))
                .ok_or(Error::GasValuesOverflow)?;

            let actual_gas_cost = actual_gas * gas_price;
            if op_info.prefund < actual_gas_cost {
                return Err(Error::FailedOp {
                    op_index,
                    reason: format!("AA51 prefund below {actual_gas_cost}"),
                });
            }
            let refund = op_info
                .prefund
                .checked_sub(actual_gas_cost)
                .ok_or(Error::GasValuesOverflow)?;
            self.stake_manager
                .increment_deposit(refund_address, refund as Balance)?;
            let success = mode == PostOpMode::OpSucceeded;
            ink::codegen::EmitEvent::<Self>::emit_event(
                self.env(),
                UserOperationEvent {
                    user_op_hash: op_info.user_op_hash.into(),
                    sender: user_op.sender,
                    paymaster: user_op.paymaster_and_data.paymaster(),
                    nonce: user_op.nonce.into(),
                    success,
                    actual_gas_cost,
                    actual_gas_used: actual_gas,
                },
            );

            Ok(actual_gas_cost)
        }

        /**
         * inner function to handle a UserOperation.
         * Must be declared "external" to open a call context, but it can only be called by handleOps.
         */
        fn inner_handle_op(
            &mut self,
            call_data: &Vec<u8>,
            op_info: &UserOpInfo,
            context: &Vec<u8>,
        ) -> Result<u64> {
            let pre_gas = self.env().gas_left();
            // TODO:
            ink::env::debug_println!(
                "only internal call caller: {:?} address: {:?}",
                self.env().caller(),
                self.env().account_id()
            );
            // if self.env().caller() != self.env().account_id() {
            //     ink::env::debug_println!(
            //         "only internal call caller: {:?} address: {:?}",
            //         self.env().caller(),
            //         self.env().account_id()
            //     );
            //     return Err(Error::OnlyInternalCall);
            // }

            let user_op = op_info.user_op.clone();
            let call_gas_limit = user_op.call_gas_limit;
            if self.env().gas_left()
                < call_gas_limit
                    .checked_add(user_op.verification_gas_limit)
                    .and_then(|pre| pre.checked_add(5000))
                    .ok_or(Error::GasValuesOverflow)?
            {
                ink::env::debug_println!("out of gas");
                return Err(Error::OutOfGas);
            }
            let mut mode = PostOpMode::OpSucceeded;

            if !call_data.is_empty() {
                let user_op_hash = user_op.hash();
                let call = Transaction::<AAEnvironment>::new(
                    user_op.callee,
                    user_op.selector,
                    user_op.call_data,
                    user_op.call_gas_limit,
                )
                .call();
                match call.try_invoke() {
                    Ok(Ok(result)) => {
                        ink::env::debug_println!("call result {:?}", result);
                        ink::codegen::EmitEvent::<Self>::emit_event(
                            self.env(),
                            UserOperationReturnValue {
                                user_op_hash: user_op_hash.into(),
                                success: true,
                                result,
                            },
                        );
                    }
                    e => {
                        ink::env::debug_println!("call error: {:?}", e);
                        ink::codegen::EmitEvent::<Self>::emit_event(
                            self.env(),
                            UserOperationRevertReason {
                                user_op_hash: op_info.user_op_hash.into(),
                                sender: user_op.sender,
                                nonce: user_op.nonce.into(),
                                revert_reason: format!("{:?}", e).into_bytes(),
                            },
                        );
                        mode = PostOpMode::OpReverted;
                    }
                };
            }
            let actual_gas = pre_gas
                .checked_sub(self.env().gas_left())
                .and_then(|pre| pre.checked_add(op_info.pre_op_gas))
                .ok_or(Error::GasValuesOverflow)?;
            self.handle_post_op(0, mode, op_info, context, actual_gas)
        }

        /**
         * execute a user op
         * @param opIndex index into the opInfo array
         * @param userOp the userOp to execute
         * @param opInfo the opInfo filled by validatePrepayment for this userOp.
         * @return collected the total amount this userOp paid.
         */
        fn execute_user_op(
            &mut self,
            op_index: u64,
            user_op: &UserOperation<AAEnvironment>,
            op_info: &UserOpInfo,
        ) -> Result<u64> {
            let pre_gas = self.env().gas_left();
            let context = op_info.context.clone();

            let actual_gas_cost = match self.inner_handle_op(&user_op.call_data, op_info, &context)
            {
                Ok(actual_gas_cost) => actual_gas_cost,
                Err(Error::OutOfGas) => return Err(Error::OutOfGas),
                Err(_) => {
                    let actual_gas = pre_gas
                        .checked_sub(self.env().gas_left())
                        .and_then(|pre| pre.checked_add(op_info.pre_op_gas))
                        .ok_or(Error::GasValuesOverflow)?;
                    self.handle_post_op(
                        op_index,
                        PostOpMode::OpReverted,
                        op_info,
                        &context,
                        actual_gas,
                    )?
                }
            };
            Ok(actual_gas_cost)
        }

        /**
         * validate account and paymaster (if defined).
         * also make sure total validation doesn't exceed verificationGasLimit
         * this method is called off-chain (simulateValidation()) and on-chain (from handleOps)
         * @param opIndex the index of this userOp into the "opInfos" array
         * @param userOp the userOp to validate
         */
        fn validate_prepayment(
            &mut self,
            op_index: u64,
            user_op: &UserOperation<AAEnvironment>,
            out_op_info: &mut UserOpInfo,
        ) -> Result<(ValidationData<AAEnvironment>, ValidationData<AAEnvironment>)> {
            let pre_gas = self.env().gas_left();

            let m_user_op = &mut out_op_info.user_op;
            *m_user_op = user_op.clone();
            out_op_info.user_op_hash = self.inner_get_user_op_hash(user_op);

            let required_pre_fund = self.get_required_prefund(m_user_op)?;
            drop(m_user_op);
            let (gas_used_by_validate_account_prepayment, validation_data) = self
                .validate_account_prepayment(
                    op_index,
                    user_op,
                    out_op_info,
                    required_pre_fund as Balance,
                )?;
            let m_user_op = &mut out_op_info.user_op;

            if !self
                .nonce_manager
                .validate_and_update_nonce(m_user_op.sender, m_user_op.nonce)
            {
                return Err(Error::InvalidAccountNonce);
            }

            if m_user_op.paymaster_and_data.is_eq_zero() {
                return Err(Error::InvalidPaymasterAddress);
            }

            let (context, paymaster_validation_data) = self.validate_paymaster_prepayment(
                op_index,
                user_op,
                out_op_info,
                required_pre_fund as Balance,
                gas_used_by_validate_account_prepayment,
            )?;

            let gas_used = pre_gas
                .checked_sub(self.env().gas_left())
                .ok_or(Error::GasValuesOverflow)?;
            if user_op.verification_gas_limit < gas_used {
                return Err(Error::OverVerificationGasLimit);
            }
            out_op_info.prefund = required_pre_fund;
            out_op_info.context = context;
            out_op_info.pre_op_gas = pre_gas
                .checked_sub(self.env().gas_left())
                .and_then(|pre| pre.checked_add(user_op.pre_verification_gas))
                .ok_or(Error::GasValuesOverflow)?;
            Ok((validation_data, paymaster_validation_data))
        }

        fn get_required_prefund(&self, user_op: &UserOperation<AAEnvironment>) -> Result<u64> {
            let mul = if user_op.paymaster_and_data.is_eq_zero() {
                3
            } else {
                1
            };
            let required_gas = user_op
                .verification_gas_limit
                .checked_mul(mul)
                .and_then(|pre| pre.checked_add(user_op.call_gas_limit))
                .and_then(|pre| pre.checked_add(user_op.pre_verification_gas))
                .ok_or(Error::GasValuesOverflow)?;
            let res = required_gas
                .checked_mul(user_op.max_fee_per_gas)
                .ok_or(Error::GasValuesOverflow)?;
            Ok(res)
        }

        /*
         * call account.validateUserOp.
         * revert (with FailedOp) in case validateUserOp reverts, or account didn't send required prefund.
         * decrement account's deposit if needed
         */
        fn validate_account_prepayment(
            &mut self,
            op_index: u64,
            user_op: &UserOperation<AAEnvironment>,
            out_op_info: &mut UserOpInfo,
            required_prefund: Balance,
        ) -> Result<(u64, ValidationData<AAEnvironment>)> {
            let pre_gas = self.env().gas_left();
            let m_user_op = &mut out_op_info.user_op;
            let sender = m_user_op.sender;
            // TODO:
            // self.create_sender_if_needed(op_index, out_op_info, m_user_op.init_code);
            let paymaster = m_user_op.paymaster_and_data.paymaster();
            let missing_account_funds = if m_user_op.paymaster_and_data.is_eq_zero() {
                let bal = self.balance_of(sender);
                if bal > required_prefund {
                    0
                } else {
                    required_prefund - bal
                }
            } else {
                0
            };

            let account_ref: ink_aa::traits::entry_point::AccountRef<AAEnvironment> = sender.into();
            use ink_aa::traits::account::IAccount;
            let res = account_ref.validate_user_op(
                user_op.clone(),
                out_op_info.user_op_hash.into(),
                missing_account_funds,
            );
            let validation_data = match res {
                Ok(validation_data) => validation_data,
                Err(e) => {
                    return Err(Error::FailedOp {
                        op_index,
                        reason: format!("AA23 reverted: {:?}", e),
                    })
                }
            };

            if paymaster == AccountId::from([0; 32]) {
                let sender_info = self.stake_manager.get_deposit_info(sender);
                let deposit = sender_info.deposit;
                if required_prefund > deposit {
                    return Err(Error::FailedOp {
                        op_index,
                        reason: "AA21 didn't pay prefund".into(),
                    });
                }
                self.stake_manager
                    .required_prefund(sender, required_prefund)?;
            }
            let gas_used_by_validate_account_prepayment = pre_gas
                .checked_sub(self.env().gas_left())
                .ok_or(Error::GasValuesOverflow)?;
            Ok((gas_used_by_validate_account_prepayment, validation_data))
        }
        /**
         * Execute a batch of UserOperations.
         * no signature aggregator is used.
         * if any account requires an aggregator (that is, it returned an aggregator when
         * performing simulateValidation), then handleAggregatedOps() must be used instead.
         * @param ops the operations to execute
         * @param beneficiary the address to receive the fees
         */
        fn inner_handle_ops(
            &mut self,
            ops: &Vec<UserOperation<AAEnvironment>>,
            beneficiary: AccountId,
        ) -> Result<()> {
            let ops_len = ops.len();
            let mut op_infos = Vec::with_capacity(ops_len);
            for (i, op) in ops.iter().enumerate() {
                let mut op_info = UserOpInfo::default();
                match self.validate_prepayment(i as u64, op, &mut op_info) {
                    Ok((validation_data, pm_validation_data)) => {
                        if let Err(e) = self.validate_account_and_paymaster_validation_data(
                            i as u64,
                            validation_data,
                            pm_validation_data,
                            Aggregator::NoAggregator,
                        ) {
                            ink::codegen::EmitEvent::<Self>::emit_event(
                                self.env(),
                                UserOperationReturnValue {
                                    user_op_hash: op.hash().into(),
                                    success: false,
                                    result: OpaqueTypes(format!("{e:?}").into_bytes()),
                                },
                            );
                        } else {
                            op_infos.push((i, op, op_info));
                        }
                    }
                    Err(e) => {
                        ink::codegen::EmitEvent::<Self>::emit_event(
                            self.env(),
                            UserOperationReturnValue {
                                user_op_hash: op.hash().into(),
                                success: false,
                                result: OpaqueTypes(format!("{e:?}").into_bytes()),
                            },
                        );
                    }
                }
            }
            let mut collected = 0;
            ink::codegen::EmitEvent::<Self>::emit_event(self.env(), BeforeExecution {});
            for (i, op, ref mut op_info) in op_infos {
                collected += self
                    .execute_user_op(i as u64, op, op_info)
                    .unwrap_or_default();
            }
            self.compensate(beneficiary, collected as Balance)?;
            Ok(())
        }

        fn inner_get_user_op_hash(&self, user_op: &UserOperation<AAEnvironment>) -> [u8; 32] {
            use scale::Encode;
            ink_aa::core::helpers::keccak256(
                &(user_op.hash(), self.env().account_id().encode()).encode(),
            )
        }

        /**
         * In case the request has a paymaster:
         * Validate paymaster has enough deposit.
         * Call paymaster.validatePaymasterUserOp.
         * Revert with proper FailedOp in case paymaster reverts.
         * Decrement paymaster's deposit
         */

        fn validate_paymaster_prepayment(
            &mut self,
            op_index: u64,
            op: &UserOperation<AAEnvironment>,
            op_info: &UserOpInfo,
            required_pre_fund: Balance,
            gas_used_by_validate_account_prepayment: u64,
        ) -> Result<(Vec<u8>, ValidationData<AAEnvironment>)> {
            let m_user_op = &op_info.user_op;
            let verification_gas_limit = m_user_op.verification_gas_limit;
            if verification_gas_limit <= gas_used_by_validate_account_prepayment {
                return Err(Error::TooLittleVerificationGas);
            }
            // TODO:
            // let gas = verification_gas_limit
            //     .checked_sub(gas_used_by_validate_account_prepayment)
            //     .ok_or(Error::GasValuesOverflow)?;
            let paymaster = m_user_op.paymaster_and_data.paymaster();
            let paymaster_info = self.get_deposit_info(paymaster);
            let deposit = paymaster_info.deposit;
            if deposit < required_pre_fund {
                return Err(Error::PaymasterDepositTooLow);
            }

            self.stake_manager
                .required_prefund(paymaster, required_pre_fund)?;

            let paymaster_ref: PaymasterRef<AAEnvironment> = paymaster.into();

            let (context, validation_data) = paymaster_ref
                .validate_paymaster_user_op(
                    op.clone(),
                    op_info.user_op_hash.into(),
                    required_pre_fund,
                )
                .map_err(|e| Error::FailedOp {
                    op_index,
                    reason: format!("AA33 reverted: {e:?}"),
                })?;
            Ok((context, validation_data))
        }

        /**
         * revert if either account validationData or paymaster validationData is expired
         */
        fn validate_account_and_paymaster_validation_data(
            &self,
            op_index: u64,
            validation_data: ValidationData<AAEnvironment>,
            paymaster_validation_data: ValidationData<AAEnvironment>,
            expected_aggregator: Aggregator<AAEnvironment>,
        ) -> Result<()> {
            let (aggregator, out_of_time_range) = self.get_validation_data(validation_data);
            if expected_aggregator != aggregator {
                return Err(Error::FailedOp {
                    op_index,
                    reason: "AA24 signature error".into(),
                });
            }
            if out_of_time_range {
                return Err(Error::FailedOp {
                    op_index,
                    reason: "AA22 expired or not due".into(),
                });
            }
            //pmAggregator is not a real signature aggregator: we don't have logic to handle it as address.
            // non-zero address means that the paymaster fails due to some signature check (which is ok only during estimation)
            let (pm_aggregator, out_of_time_range) =
                self.get_validation_data(paymaster_validation_data);
            if pm_aggregator != Aggregator::NoAggregator {
                return Err(Error::FailedOp {
                    op_index,
                    reason: "AA34 signature error".into(),
                });
            }
            if out_of_time_range {
                return Err(Error::FailedOp {
                    op_index,
                    reason: "AA32 paymaster expired or not due".into(),
                });
            }
            Ok(())
        }

        fn get_validation_data(
            &self,
            validation_data: ValidationData<AAEnvironment>,
        ) -> (Aggregator<AAEnvironment>, bool) {
            use scale::Encode;
            if validation_data.encode().iter().all(|a| a.eq(&0)) {
                return (Aggregator::IllegalAggregator, false);
            }

            let out_of_time_range = self.env().block_timestamp() > validation_data.valid_until
                || self.env().block_timestamp() < validation_data.valid_after;
            (validation_data.aggregator, out_of_time_range)
        }

        fn inner_handle_aggregated_ops(
            &mut self,
            ops_per_aggregator: Vec<UserOpsPerAggregator>,
            beneficiary: AccountId,
        ) -> Result<()> {
            // 校验,执行每个aggregator下的user ops
            let total_ops = ops_per_aggregator
                .iter()
                .map(|opa| opa.user_ops.len())
                .sum();

            let mut op_infos = Vec::with_capacity(total_ops);

            let mut op_index = 0;
            for opa in ops_per_aggregator {
                if opa.aggregator == Aggregator::IllegalAggregator {
                    return Err(Error::InvalidAggregator);
                }

                if let Aggregator::VerifiedBy(address) = opa.aggregator {
                    let aggregator: AggregatorRef<AAEnvironment> = address.into();
                    ink_aa::traits::aggregator::IAggregator::validate_signatures(
                        &aggregator,
                        opa.user_ops.clone(),
                        opa.signature.clone(),
                    )?;
                }

                for op in opa.user_ops {
                    let mut op_info = UserOpInfo::default();

                    let (validation_data, pm_validation_data) =
                        self.validate_prepayment(op_index, &op, &mut op_info)?;

                    self.validate_account_and_paymaster_validation_data(
                        op_index,
                        validation_data,
                        pm_validation_data,
                        opa.aggregator.clone(),
                    )?;

                    op_infos.push((op_index, op, op_info));
                    op_index += 1;
                }
            }

            // 执行
            let mut collected = 0;

            for (i, op, ref mut op_info) in op_infos {
                collected += self.execute_user_op(i, &op, op_info)?;
            }

            self.compensate(beneficiary, collected as Balance)?;

            Ok(())
        }
    }
    impl IEntryPoint for EntryPoint {
        #[ink(message, payable)]
        fn handle_ops(
            &mut self,
            ops: Vec<UserOperation<AAEnvironment>>,
            beneficiary: AccountId,
        ) -> Result<()> {
            self.inner_handle_ops(&ops, beneficiary)?;
            Ok(())
        }
        #[ink(message)]
        fn handle_aggregated_ops(
            &mut self,
            ops_per_aggregator: Vec<UserOpsPerAggregator<AAEnvironment>>,
            beneficiary: AccountId,
        ) -> Result<()> {
            self.inner_handle_aggregated_ops(ops_per_aggregator, beneficiary)?;
            Ok(())
        }
        #[ink(message)]
        fn get_user_op_hash(&self, user_op: UserOperation<AAEnvironment>) -> [u8; 32] {
            self.inner_get_user_op_hash(&user_op)
        }
    }

    impl IStakeManager for EntryPoint {
        #[ink(message)]
        fn get_deposit_info(&self, account: AccountId) -> DepositInfo<AAEnvironment> {
            self.stake_manager.get_deposit_info(account)
        }
        #[ink(message)]
        fn balance_of(&self, account: AccountId) -> Balance {
            self.stake_manager.balance_of(account)
        }
        #[ink(message, payable)]
        fn deposit_to(&mut self, account: AccountId) -> Result<()> {
            self.stake_manager.deposit_to(account)
        }
        #[ink(message, payable)]
        fn add_stake(&mut self, unstake_delay_sec: Timestamp) -> Result<()> {
            self.stake_manager.add_stake(unstake_delay_sec)
        }
        #[ink(message)]
        fn unlock_stake(&mut self) -> Result<()> {
            self.stake_manager.unlock_stake()
        }
        #[ink(message, payable)]
        fn withdraw_stake(&mut self, withdraw_address: AccountId) -> Result<()> {
            self.stake_manager.withdraw_stake(withdraw_address)
        }
        #[ink(message, payable)]
        fn withdraw_to(
            &mut self,
            withdraw_address: AccountId,
            withdraw_amount: Balance,
        ) -> Result<()> {
            self.stake_manager
                .withdraw_to(withdraw_address, withdraw_amount)
        }
    }

    impl INonceManager for EntryPoint {
        #[ink(message)]
        fn get_nonce(&self, sender: AccountId, key: [u8; 24]) -> [u8; 32] {
            self.nonce_manager.get_nonce(sender, key)
        }

        #[ink(message)]
        fn increment_nonce(&mut self, key: [u8; 24]) {
            self.nonce_manager.increment_nonce(key)
        }
    }

    /// This is how you'd write end-to-end (E2E) or integration tests for ink! contracts.
    ///
    /// When running these you need to make sure that you:
    /// - Compile the tests with the `e2e-tests` feature flag enabled (`--features e2e-tests`)
    /// - Are running a Substrate node which contains `pallet-contracts` in the background
    #[cfg(all(test, feature = "e2e-tests"))]
    mod e2e_tests {
        /// Imports all the definitions from the outer scope so we can use them here.
        use super::*;

        use base_account::BaseAccountRef;
        use flip::FlipRef;
        /// A helper function used for calling contract messages.
        use ink_e2e::build_message;
        use recover_sig::RecoverSigRef;
        use simple_paymaster::SimplePaymasterRef;

        /// The End-to-End test `Result` type.
        type E2EResult<T> = std::result::Result<T, Box<dyn std::error::Error>>;

        /// We test that we can upload and instantiate the contract using its default constructor.
        #[ink_e2e::test]
        async fn default_works(mut client: ink_e2e::Client<C, E>) -> E2EResult<()> {
            let sig = RecoverSigRef::new(
                2,
                vec![ink_e2e::bob().account_id(), ink_e2e::eve().account_id()],
            );
            let sig_id = client
                .instantiate("recover_sig", &ink_e2e::bob(), 1000, None)
                .await
                .expect("uploading `recover_sig` failed")
                .code_hash;

            let constructor = FlipRef::new(false);
            let contract_account_id = client
                .instantiate("flip", &ink_e2e::bob(), constructor, 0, None)
                .await
                .expect("instantiate failed")
                .account_id;

            let get = build_message::<FlipRef>(contract_account_id.clone()).call(|flip| flip.get());
            let get_result = client.call_dry_run(&ink_e2e::bob(), &get, 0, None).await;
            assert!(matches!(get_result.return_value(), false));

            let stake_manager_hash = client
                .upload("stake_manager", &ink_e2e::alice(), None)
                .await
                .expect("uploading `stake_manager` failed")
                .code_hash;
            let nonce_manager_hash = client
                .upload("nonce_manager", &ink_e2e::alice(), None)
                .await
                .expect("uploading `nonce_manager` failed")
                .code_hash;
            let constructor = EntryPointRef::new(
                1337, // salt
                stake_manager_hash,
                nonce_manager_hash,
            );

            let entry_point_acc_id = client
                .instantiate("entry_point", &ink_e2e::alice(), constructor, 10000, None)
                .await
                .expect("instantiate failed")
                .account_id;

            let base = BaseAccountRef::new(entry_point_acc_id.clone(), sig_id.clone());
            let base_id = client
                .instantiate("base_account", &ink_e2e::bob(), base, 1000, None)
                .await
                .expect("uploading `base_account` failed")
                .code_hash;

            // When
            let flip =
                build_message::<FlipRef>(contract_account_id.clone()).call(|flip| flip.flip());
            // let _flip_result = client
            //     .call(&ink_e2e::bob(), flip, 0, None)
            //     .await
            //     .expect("flip failed");
            let op = UserOperation {
                sender: base_id,
                nonce: [0; 32],
                init_code: vec![],
                callee: contract_account_id.clone(),
                selector: [99, 58, 165, 81],
                call_data: vec![],
                call_gas_limit: 9798418432,
                verification_gas_limit: 9798418432,
                pre_verification_gas: 9798418432,
                max_fee_per_gas: 19798418432,
                max_priority_fee_per_gas: 9798418432,
                paymaster: AccountId::from([0; 32]),
                paymaster_data: vec![],
                signature: vec![],
            };

            let handle_ops = build_message::<EntryPointRef>(multi_contract_caller_acc_id.clone())
                .call(|contract| contract.handle_ops(vec![op], ink_e2e::alice().account_id()));
            let res = client
                .call(&ink_e2e::alice(), handle_ops, 2000, None)
                .await
                .return_value();
            println!("{:?}", res);

            // Then
            let get = build_message::<FlipRef>(contract_account_id.clone()).call(|flip| flip.get());
            let get_result = client.call_dry_run(&ink_e2e::bob(), &get, 0, None).await;
            assert!(matches!(get_result.return_value(), true));
            Ok(())
        }
    }
}
