use anyhow::Result;
use ink::env::Environment;
use ink_e2e::{
    subxt::{
        config::ExtrinsicParams,
        ext::{sp_core::sr25519, sp_runtime},
        Config, OnlineClient,
    },
    Client, PolkadotConfig, Signer,
};

use core::fmt::Debug;

use ink_aa::core::env::AAEnvironment;

#[tokio::main]
async fn main() -> Result<()> {
    let mut bundler = Bundler::<PolkadotConfig, AAEnvironment>::new("ws://127.0.0.1:9944").await?;

    match bundler.deploy(ink_e2e::alice()).await {
        Ok(_) => {
            println!("ok");
        }
        Err(err) => eprintln!("{err:?}"),
    }

    Ok(())
}

pub struct Bundler<C, E>
where
    C: Config,
    E: Environment,
{
    client: Client<C, E>,
}

const CONTRACTS: [&str; 8] = [
    "./output/base_account.wasm",
    "./output/base_paymaster.wasm",
    "./output/entry_point.wasm",
    "./output/flip.wasm",
    "./output/stake_mananer.wasm",
    "./output/nonce_manager.wasm",
    "./output/recover_sig.wasm",
    "./output/simple_paymaster.wasm",
];

impl<C, E> Bundler<C, E>
where
    C: Config,
    C::AccountId:
        From<sp_runtime::AccountId32> + scale::Codec + serde::de::DeserializeOwned + Debug,
    C::Signature: From<sr25519::Signature>,
    <C::ExtrinsicParams as ExtrinsicParams<C::Index, C::Hash>>::OtherParams: Default,
    E: Environment,
    E::AccountId: Debug,
    E::Balance: Debug + scale::HasCompact + serde::Serialize,
    E::Hash: Debug + scale::Encode,
{
    async fn new(ws_url: impl AsRef<str>) -> Result<Self> {
        let online_client = OnlineClient::<C>::from_url(ws_url).await?;
        let client = Client::<C, E>::new(online_client, CONTRACTS).await;
        Ok(Self { client })
    }
}

impl<C> Bundler<C, AAEnvironment>
where
    C: Config,
    C::Signature: From<sr25519::Signature>,
    <C::ExtrinsicParams as ExtrinsicParams<C::Index, C::Hash>>::OtherParams: Default,
    C::AccountId: From<sp_runtime::AccountId32>
        + scale::Codec
        + serde::de::DeserializeOwned
        + Debug
        + Into<ink_e2e::subxt::utils::AccountId32>,
{
    async fn deploy(&mut self, signer: Signer<C>) -> Result<(), ink_e2e::Error<C, AAEnvironment>> {
        let client = &mut self.client;
        let nonce_manager_code_hash = client.upload("nonce_manager", &signer, None).await?;

        let stake_manager_code_hash = client.upload("stake_manager", &signer, None).await?;

        let constructor = entry_point::EntryPointRef::new(
            12,
            stake_manager_code_hash.code_hash,
            nonce_manager_code_hash.code_hash,
        );

        let entry_point_contract = client
            .instantiate("entry_point", &signer, constructor, 0, None)
            .await?;

        println!("entry_point: {:?}", entry_point_contract.account_id);

        let simple_paymaster_constructor = simple_paymaster::SimplePaymasterRef::new();

        let simple_paymaster_contract = client
            .instantiate(
                "simple_paymaster",
                &signer,
                simple_paymaster_constructor,
                0,
                None,
            )
            .await?;

        let base_paymaster_constructor = base_paymaster::BasePaymasterRef::new(
            entry_point_contract.account_id,
            convert_account(signer.account_id().clone()),
            simple_paymaster_contract.account_id,
        );

        let base_paymaster_contract = client
            .instantiate(
                "base_paymaster",
                &signer,
                base_paymaster_constructor,
                0,
                None,
            )
            .await?;

        let wallet_constructor =
            recover_sig::RecoverSigRef::new(1, vec![convert_account(signer.account_id().clone())]);

        let wallet_contract = client
            .instantiate("recover_sig", &signer, wallet_constructor, 0, None)
            .await?;

        let base_account_constructor = base_account::BaseAccountRef::new(
            entry_point_contract.account_id,
            wallet_contract.account_id,
        );

        let base_account_contract = client
            .instantiate("base_account", &signer, base_account_constructor, 0, None)
            .await?;

        Ok(())
    }
}

pub fn convert_account<A: Into<ink_e2e::subxt::utils::AccountId32>>(
    account: A,
) -> <AAEnvironment as Environment>::AccountId {
    let account: ink_e2e::subxt::utils::AccountId32 = account.into();
    <AAEnvironment as Environment>::AccountId::from(account.0)
}
