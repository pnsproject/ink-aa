use crate::core::{env::AAEnvironment, error::Result};
use ink::{env::Environment, prelude::vec::Vec};

#[ink::trait_definition]
pub trait ISenderCreator {
    /// 使用 init_code 创建账户
    /// 返回创建的账户 ID
    #[ink(message)]
    fn create_sender(
        &mut self,
        init_code: Vec<u8>,
    ) -> Result<<AAEnvironment as Environment>::AccountId>;
}
