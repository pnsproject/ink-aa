#![cfg_attr(not(feature = "std"), no_std, no_main)]

#[ink::contract(env = env::AccountAbstractionEnvironment)]
mod sender_creator {
    use core::mem::size_of;

    #[ink(storage)]
    pub struct SenderCreator;

    impl SenderCreator {
        #[ink(constructor)]
        pub fn new() -> Self {
            Self {}
        }

        #[ink(message)]
        pub fn create_sender(&self, init_code: Vec<u8>) -> AccountId {
            const LEN: usize = size_of::<AccountId>();
            let factory = AccountId::try_from(&init_code[0..LEN]);
            if factory.is_err() {
                return AccountId::from([0; LEN]);
            }
            let factory = factory.unwrap();
            let init_call_data = &init_code[LEN..];
            let selector = ink::env::call::Selector::new([0u8; 4]); // Selector for the constructor function

            let call = ink::env::call::build_call::<env::AccountAbstractionEnvironment>()
                .call(factory)
                .gas_limit(self.env().gas_left())
                .exec_input(ink::env::call::ExecutionInput::new(selector).push_arg(init_call_data))
                .returns::<AccountId>()
                .try_invoke();

            match call {
                Ok(Ok(result)) => result,
                _ => AccountId::from([0; LEN]),
            }
        }
    }

    // TODO:
    #[cfg(all(test, feature = "e2e-tests"))]
    mod e2e_tests {
        use super::*;

        type E2EResult<T> = std::result::Result<T, Box<dyn std::error::Error>>;

        #[ink_e2e::test]
        async fn create_sender_works(mut client: ink_e2e::Client<C, E>) -> E2EResult<()> {
            // Deploy the SenderCreator contract
            let constructor = SenderCreator::new();
            let contract_account = client
                .instantiate("sender_creator", &ink_e2e::alice(), constructor, 0, None)
                .await
                .expect("failed to deploy contract");

            // Create a new Sender contract instance using create_sender
            let factory = Key::from(ink_e2e::alice()).into_bytes();
            let constructor_selector = [0u8; 4];
            let init_data = Vec::new();
            let init_code = [&factory[..], &constructor_selector[..], &init_data[..]].concat();
            let sender_account_id =
                ContractAccount::<Sender>::deploy(&mut client, &ink_e2e::bob(), &init_code).await?;

            // Call the get function on the SenderCreator contract to check if the new Sender contract instance has been added
            let get_sender =
                build_message::<SenderCreatorRef>(contract_account.account_id().clone())
                    .call(|creator| creator.get_sender(sender_account_id.clone()));
            let sender_added = client
                .call_dry_run(&ink_e2e::bob(), &get_sender, 0, None)
                .await;
            assert!(matches!(sender_added.return_value(), true));

            Ok(())
        }
    }
}
