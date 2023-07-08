use super::env::AAEnvironment;
use ink::env::{
    call::{
        build_call,
        utils::{Argument, ArgumentList, EmptyArgumentList},
        Call, CallParams, ExecutionInput,
    },
    CallFlags, Environment,
};
use ink::prelude::vec;
use ink::prelude::vec::Vec;

#[derive(scale::Decode, scale::Encode, Clone, Hash)]
#[cfg_attr(
    feature = "std",
    derive(
        Debug,
        PartialEq,
        Eq,
        scale_info::TypeInfo,
        ink::storage::traits::StorageLayout
    )
)]
pub struct Transaction<E: Environment = AAEnvironment> {
    /// The `AccountId` of the contract that is called in this transaction.
    pub callee: E::AccountId,
    /// The selector bytes that identifies the function of the callee that should be
    /// called.
    pub selector: [u8; 4],
    /// The SCALE encoded parameters that are passed to the called function.
    pub input: Vec<u8>,
    /// The amount of chain balance that is transferred to the callee.
    pub transferred_value: E::Balance,
    /// Gas limit for the execution of the call.
    pub gas_limit: u64,
    /// If set to true the transaction will be allowed to re-enter the multisig
    /// contract. Re-entrancy can lead to vulnerabilities. Use at your own
    /// risk.
    pub allow_reentry: bool,
}

type Args = ArgumentList<Argument<OpaqueTypes>, EmptyArgumentList>;

impl<E: Environment> Transaction<E> {
    pub fn new(
        callee: E::AccountId,
        selector: [u8; 4],
        call_data: Vec<u8>,
        gas_limit: u64,
    ) -> Self {
        use num_traits::identities::Zero;
        Self {
            callee,
            selector,
            input: call_data,
            gas_limit,
            // TODO:
            transferred_value: E::Balance::zero(),
            allow_reentry: false,
        }
    }
    pub fn call(self) -> CallParams<E, Call<E>, Args, OpaqueTypes> {
        build_call::<E>()
            .call(self.callee)
            .gas_limit(self.gas_limit)
            .transferred_value(self.transferred_value)
            .call_flags(CallFlags::default().set_allow_reentry(self.allow_reentry))
            .exec_input(ExecutionInput::new(self.selector.into()).push_arg(OpaqueTypes(self.input)))
            .returns::<OpaqueTypes>()
            .params()
    }
}

#[cfg_attr(
    feature = "std",
    derive(
        PartialEq,
        Eq,
        scale_info::TypeInfo,
        ink::storage::traits::StorageLayout
    )
)]
#[derive(Clone, Debug)]
pub struct OpaqueTypes(pub Vec<u8>);

impl scale::Encode for OpaqueTypes {
    #[inline]
    fn size_hint(&self) -> usize {
        self.0.len()
    }

    #[inline]
    fn encode_to<O: scale::Output + ?Sized>(&self, output: &mut O) {
        output.write(&self.0);
    }
}

impl scale::Decode for OpaqueTypes {
    #[inline]
    fn decode<I: scale::Input>(input: &mut I) -> Result<Self, scale::Error> {
        let len = input.remaining_len()?;

        let mut bytes;

        if let Some(len) = len {
            bytes = vec![0; len];
            input.read(&mut bytes[..len])?;
        } else {
            bytes = Vec::new();
            loop {
                match input.read_byte() {
                    Ok(b) => bytes.push(b),
                    Err(_) => break,
                }
            }
        };

        Ok(OpaqueTypes(bytes))
    }
}

#[cfg(test)]

mod tests {
    use super::*;
    use ink::env::DefaultEnvironment;
    use ink::prelude::vec;

    #[test]
    fn test_call() {
        let call: Transaction<DefaultEnvironment> = Transaction {
            callee: [0u8; 32].into(),
            selector: [0u8; 4].into(),
            input: vec![1, 2, 3, 4, 5, 6],
            transferred_value: 1,
            gas_limit: 1,
            allow_reentry: false,
        };
        println!("{call:?}");
    }
    #[test]
    fn test_opaque_types() {
        fn test_opaque_types<T>(value: T)
        where
            T: scale::Encode + scale::Decode + PartialEq + std::fmt::Debug,
        {
            let encoded = value.encode();
            let opaque_types = OpaqueTypes(encoded.clone());
            assert_eq!(scale::Encode::encode(&opaque_types), encoded);
            let decoded: OpaqueTypes =
                scale::Decode::decode(&mut &encoded[..]).expect("decode failed");
            assert_eq!(decoded.0, opaque_types.0);
            let value = T::decode(&mut &decoded.0[..]).expect("decode failed");
            println!("{}: {value:?}", core::any::type_name::<T>());
        }
        test_opaque_types(true);
        test_opaque_types(123);
        test_opaque_types(456i64);
        test_opaque_types("hello".to_owned());
        test_opaque_types(vec![1, 2, 3]);
        test_opaque_types((true, "world".to_owned()));
    }
}
