#![allow(non_snake_case)]
use dioxus::prelude::*;
use hex::ToHex;
use libwallet::{self, vault, Signature};
use log::LevelFilter;
use serde::{Deserialize, Serialize};
use serde_json::json;
use sube::builder::TxBuilder;
use sube::codec::Decode;
use sube::{builder::QueryBuilder, Error, Response};

type Wallet = libwallet::Wallet<vault::Simple>;

fn main() {
    dioxus_logger::init(LevelFilter::Trace).expect("failed to init logger");
    dioxus_web::launch(App);
}

#[derive(Decode, Debug)]
pub struct AccountInfo {
    pub nonce: u32,
    pub consumers: u32,
    pub providers: u32,
    pub sufficients: u32,
    pub data: AccountData,
}

#[derive(Decode, Debug)]
pub struct AccountData {
    pub free: u128,
    pub reserved: u128,
    pub frozen: u128,
    pub flags: u128,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Destination {
    #[serde(rename = "Id")]
    id: Vec<u8>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Transfer {
    dest: Destination,
    value: u128,
}

pub enum BalanceError {
    Decode,
    Unknown,
}

const DECIMALS: u8 = 12;

async fn get_balance(account: String) -> Result<u128, BalanceError> {
    let url = format!("wss://rococo-rpc.polkadot.io/system/account/{account}",);

    let response = QueryBuilder::default()
        .with_url(&url)
        .await
        .map_err(|_| BalanceError::Unknown)?;

    match response {
        Response::Value(value) => {
            let data = value.as_ref();
            let account_info =
                AccountInfo::decode(&mut &data[..]).map_err(|_| BalanceError::Decode)?;

            log::info!("Account info: {:?}", account_info);
            Ok(account_info.data.free)
        }
        _ => Err(BalanceError::Unknown),
    }
}

async fn transfer_balance() {
    let phrase = String::from("my_phrase");

    let (vault, phrase) = if phrase.is_empty() {
        vault::Simple::generate_with_phrase(&mut rand_core::OsRng)
    } else {
        let phrase: libwallet::Mnemonic = phrase.parse().expect("Invalid phrase");
        log::info!("phase: {:?}", phrase);
        (vault::Simple::from_phrase(&phrase), phrase)
    };

    let mut wallet = Wallet::new(vault, libwallet::AccountPath::Default);
    wallet.unlock(None).await;

    let account = wallet.default_account();
    let public = account.public();

    log::info!("Secret phrase: \"{phrase}\"");
    log::info!("Default Account: 0x{account}");

    get_balance(format!("0x{}", account.to_string())).await;

    let account = wallet.default_account();
    let account_public = wallet.default_account().public();

    let destination_public = hex::decode("your_dest").unwrap();

    log::info!("{:?}", destination_public);

    let amount: u128 = 2_000_000_000_000;

    let transfer = Transfer {
        dest: Destination {
            id: destination_public,
        },
        value: amount,
    };

    let body = json!(&transfer);
    log::info!("{:?}", body);

    let response = TxBuilder::default()
        .with_url("wss://rococo-rpc.polkadot.io/balances/transfer_Keep_Alive")
        .with_signer(|message: &[u8]| Ok(wallet.sign(message).as_bytes()))
        .with_sender(account_public.as_ref())
        .with_body(body)
        .await;

    log::info!("{:?}", response);
}

fn App(cx: Scope) -> Element {
    let balance = use_ref(cx, || 0.0);
    let error = use_ref(cx, || String::new());
    use_coroutine(cx, |mut rx: UnboundedReceiver<bool>| {
        to_owned![balance, error];

        async move {
            transfer_balance().await;
        }
    });

    cx.render(rsx! {
        div {
            "Hi, look at the console"
        }
    })
}
