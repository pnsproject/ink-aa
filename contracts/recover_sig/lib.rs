#![cfg_attr(not(feature = "std"), no_std, no_main)]

pub use self::recover_sig::{ConfirmationStatus, RecoverSig};

#[ink::contract(env = ink_aa::core::env::AAEnvironment)]
mod recover_sig {
    use base_account::BaseAccountTrait;
    use ink::{prelude::vec::Vec, storage::Mapping};
    use ink_aa::core::user_operation::UserOperation;
    use ink_aa::core::{env::AAEnvironment, helpers::ValidationData};
    use ink_aa::core::{
        error::{Error, Result},
        helpers::Aggregator,
    };
    const MAX_OWNERS: u32 = 50;

    #[derive(Clone, Copy, scale::Decode, scale::Encode)]
    #[cfg_attr(
        feature = "std",
        derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout)
    )]
    pub enum ConfirmationStatus {
        /// The transaction is already confirmed.
        Confirmed,
        /// Indicates how many confirmations are remaining.
        ConfirmationsNeeded(u32),
    }

    /// Emitted when an owner is added to the wallet.
    #[ink(event)]
    pub struct OwnerAddition {
        /// The owner that was added.
        #[ink(topic)]
        owner: AccountId,
    }

    /// Emitted when an owner is removed from the wallet.
    #[ink(event)]
    pub struct OwnerRemoval {
        /// The owner that was removed.
        #[ink(topic)]
        owner: AccountId,
    }

    /// Emitted when the requirement changed.
    #[ink(event)]
    pub struct RequirementChange {
        /// The new requirement value.
        new_requirement: u32,
    }

    #[ink(storage)]
    #[derive(Default)]
    pub struct RecoverSig {
        owners: Vec<AccountId>,
        is_owner: Mapping<AccountId, ()>,
        requirement: u32,
    }

    impl RecoverSig {
        #[ink(constructor)]
        pub fn new(requirement: u32, mut owners: Vec<AccountId>) -> Self {
            let mut contract = RecoverSig::default();
            owners.sort_unstable();
            owners.dedup();
            ensure_requirement_is_valid(owners.len() as u32, requirement);

            for owner in &owners {
                contract.is_owner.insert(owner, &());
            }

            contract.owners = owners;
            contract.requirement = requirement;
            contract
        }

        #[ink(message)]
        pub fn add_owner(&mut self, new_owner: AccountId) {
            self.ensure_from_wallet();
            self.ensure_no_owner(&new_owner);
            ensure_requirement_is_valid(self.owners.len() as u32 + 1, self.requirement);
            self.is_owner.insert(new_owner, &());
            self.owners.push(new_owner);
            self.env().emit_event(OwnerAddition { owner: new_owner });
        }

        #[ink(message)]
        pub fn remove_owner(&mut self, owner: AccountId) {
            self.ensure_from_wallet();
            self.ensure_owner(&owner);
            let len = self.owners.len() as u32 - 1;
            let requirement = u32::min(len, self.requirement);
            ensure_requirement_is_valid(len, requirement);
            let owner_index = self.owner_index(&owner) as usize;
            self.owners.swap_remove(owner_index);
            self.is_owner.remove(owner);
            self.requirement = requirement;
            self.env().emit_event(OwnerRemoval { owner });
        }

        #[ink(message)]
        pub fn replace_owner(&mut self, old_owner: AccountId, new_owner: AccountId) {
            self.ensure_from_wallet();
            self.ensure_owner(&old_owner);
            self.ensure_no_owner(&new_owner);
            let owner_index = self.owner_index(&old_owner);
            self.owners[owner_index as usize] = new_owner;
            self.is_owner.remove(old_owner);
            self.is_owner.insert(new_owner, &());
            self.env().emit_event(OwnerRemoval { owner: old_owner });
            self.env().emit_event(OwnerAddition { owner: new_owner });
        }

        #[ink(message)]
        pub fn change_requirement(&mut self, new_requirement: u32) {
            self.ensure_from_wallet();
            ensure_requirement_is_valid(self.owners.len() as u32, new_requirement);
            self.requirement = new_requirement;
            self.env().emit_event(RequirementChange { new_requirement });
        }

        /// Panics if `owner` is not found in `self.owners`.
        fn owner_index(&self, owner: &AccountId) -> u32 {
            self.owners.iter().position(|x| *x == *owner).expect(
                "This is only called after it was already verified that the id is
                 actually an owner.",
            ) as u32
        }

        fn ensure_caller_is_owner(&self) {
            self.ensure_owner(&self.env().caller());
        }

        fn ensure_from_wallet(&self) {
            assert_eq!(self.env().caller(), self.env().account_id());
        }

        fn ensure_owner(&self, owner: &AccountId) {
            assert!(self.is_owner.contains(owner));
        }

        fn ensure_no_owner(&self, owner: &AccountId) {
            assert!(!self.is_owner.contains(owner));
        }
    }

    fn ensure_requirement_is_valid(owners: u32, requirement: u32) {
        assert!(0 < requirement && requirement <= owners && owners <= MAX_OWNERS);
    }

    impl BaseAccountTrait for RecoverSig {
        #[ink(message, payable)]
        fn validate_signature(
            &self,
            _op: UserOperation<AAEnvironment>,
            _user_op_hash: Hash,
        ) -> Result<ValidationData<AAEnvironment>> {
            // TODO:
            Ok(ValidationData {
                aggregator: Aggregator::VerifiedBySelf,
                valid_after: self.env().block_timestamp(),
                valid_until: self.env().block_timestamp() + 5000,
            })
        }

        #[ink(message)]
        fn validate_nonce(&self, _nonce: [u8; 32]) -> Result<()> {
            Ok(())
        }
    }
}
