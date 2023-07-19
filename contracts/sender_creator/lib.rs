#![cfg_attr(not(feature = "std"), no_std, no_main)]

#[ink::contract(env = ink_aa::core::env::AAEnvironment)]
mod sender_creator {
    use core::mem::size_of;
    use ink::prelude::vec::Vec;
    use ink_aa::core::error::Result;
    use ink_aa::traits::sender_creator::ISenderCreator;
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

            let call = ink::env::call::build_call::<ink_aa::core::env::AAEnvironment>()
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

    // impl ISenderCreator for SenderCreator {
    //     #[ink(message)]
    //     fn create_sender(&mut self, init_code: Vec<u8>) -> Result<AccountId> {
    //         // 从 init_code 中取出 factory 地址
    //         let factory = AccountId::from_slice(&init_code[0..32]);

    //         // 构造调用参数
    //         let salt = [0x42; 32];
    //         let endowment = 0;
    //         let gas_limit = 0; // no limit
    //         let exec_input = ink::env::call::ExecutionInput::new(construct_selector());

    //         // 调用 CreateBuilder 实例化
    //         let account_id = ink::env::call::build_create()
    //             .gas_limit(gas_limit)
    //             .endowment(endowment)
    //             .code_hash(factory.code_hash())
    //             .salt_bytes(salt)
    //             .exec_input(exec_input)
    //             .returns::<AccountId>()
    //             .try_instantiate()
    //             .map_err(|e| e.into())?;

    //         Ok(account_id)
    //     }
    // }
}
