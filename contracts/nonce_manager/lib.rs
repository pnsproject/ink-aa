#![cfg_attr(not(feature = "std"), no_std, no_main)]

#[ink::contract]
mod nonce_manager {
    use ink::storage::Mapping;
    use nonce_manager_trait::INonceManager;

    /// Defines the storage of your contract.
    /// Add new fields to the below struct in order
    /// to add new static storage fields to your contract.
    #[ink(storage)]
    pub struct NonceManager {
        /// Stores a single `bool` value on the storage.
        nonce_sequence_number: Mapping<(AccountId, [u8; 24]), [u8; 32]>,
    }

    impl NonceManager {
        /// Constructor that initializes the `bool` value to the given `init_value`.
        #[ink(constructor)]
        pub fn new() -> Self {
            Self {
                nonce_sequence_number: Mapping::default(),
            }
        }

        // fn validateAndUpdateNonce(&mut self,sender: AccountId,nonce: [u8; 32]) -> bool {
        //     let key = nonce >> 64;
        //     let seq = u64::from_be_bytes(nonce);

        //     let mut nonce_sequence_number = self.get_nonce(sender,key);
        //     increment_bytes(&mut nonce_sequence_number);
        //     self.nonce_sequence_number
        //         .insert((sender, key), &nonce_sequence_number);

        //     return nonceSequenceNumber[sender][key]++ == seq;
        // }
    }

    impl INonceManager for NonceManager {
        #[ink(message)]
        fn get_nonce(&self, sender: AccountId, key: [u8; 24]) -> [u8; 32] {
            self.nonce_sequence_number.get((sender, key)).unwrap_or({
                let mut h = [0; 32];
                h[..24].copy_from_slice(&key);
                h
            })
        }

        #[ink(message)]
        fn increment_nonce(&mut self, key: [u8; 24]) {
            let mut nonce = self.get_nonce(self.env().caller(), key);
            increment_bytes(&mut nonce);
            self.nonce_sequence_number
                .insert((self.env().caller(), key), &nonce);
        }
    }
    fn increment_bytes(bytes: &mut [u8; 32]) {
        let mut carry = true;
        for i in (0..bytes.len()).rev() {
            if carry {
                if bytes[i] == 255 {
                    bytes[i] = 0;
                } else {
                    bytes[i] += 1;
                    carry = false;
                }
            }
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

        /// A helper function used for calling contract messages.
        use ink_e2e::build_message;

        /// The End-to-End test `Result` type.
        type E2EResult<T> = std::result::Result<T, Box<dyn std::error::Error>>;

        /// We test that we can upload and instantiate the contract using its default constructor.
        #[ink_e2e::test]
        async fn default_works(mut client: ink_e2e::Client<C, E>) -> E2EResult<()> {
            // Given
            let constructor = NonceManagerRef::default();

            // When
            let contract_account_id = client
                .instantiate("nonce_manager", &ink_e2e::alice(), constructor, 0, None)
                .await
                .expect("instantiate failed")
                .account_id;

            // Then
            let get = build_message::<NonceManagerRef>(contract_account_id.clone())
                .call(|nonce_manager| nonce_manager.get());
            let get_result = client.call_dry_run(&ink_e2e::alice(), &get, 0, None).await;
            assert!(matches!(get_result.return_value(), false));

            Ok(())
        }

        /// We test that we can read and write a value from the on-chain contract contract.
        #[ink_e2e::test]
        async fn it_works(mut client: ink_e2e::Client<C, E>) -> E2EResult<()> {
            // Given
            let constructor = NonceManagerRef::new(false);
            let contract_account_id = client
                .instantiate("nonce_manager", &ink_e2e::bob(), constructor, 0, None)
                .await
                .expect("instantiate failed")
                .account_id;

            let get = build_message::<NonceManagerRef>(contract_account_id.clone())
                .call(|nonce_manager| nonce_manager.get());
            let get_result = client.call_dry_run(&ink_e2e::bob(), &get, 0, None).await;
            assert!(matches!(get_result.return_value(), false));

            // When
            let flip = build_message::<NonceManagerRef>(contract_account_id.clone())
                .call(|nonce_manager| nonce_manager.flip());
            let _flip_result = client
                .call(&ink_e2e::bob(), flip, 0, None)
                .await
                .expect("flip failed");

            // Then
            let get = build_message::<NonceManagerRef>(contract_account_id.clone())
                .call(|nonce_manager| nonce_manager.get());
            let get_result = client.call_dry_run(&ink_e2e::bob(), &get, 0, None).await;
            assert!(matches!(get_result.return_value(), true));

            Ok(())
        }
    }
}
