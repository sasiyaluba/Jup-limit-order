use serde_json::Value;
use std::{str::FromStr, sync::Arc};

use base64::{engine::general_purpose, Engine};
use jito_sdk_rust::JitoJsonRpcSDK;
use jupiter_swap_api_client::JupiterSwapApiClient;
use reqwest::Client;
use serde_json::json;
use solana_client::{nonblocking::rpc_client::RpcClient, rpc_client::SerializableTransaction};
use solana_sdk::{
    address_lookup_table::{state::AddressLookupTable, AddressLookupTableAccount},
    bs58,
    hash::Hash,
    instruction::Instruction,
    message::v0::Message,
    pubkey::Pubkey,
    signature::{Keypair, Signature},
    transaction::VersionedTransaction,
};

use anyhow::{anyhow, Result};

use crate::jup::get_swap_ix;

/// accounts -> 地址查找表的pubkey数组
/// 返回地址查找表的账户结构
pub async fn get_address_lookup(
    rpc: Arc<RpcClient>,
    accounts: Vec<Pubkey>,
) -> Result<Vec<AddressLookupTableAccount>> {
    let mut alts = vec![];
    if !accounts.is_empty() {
        let accounts_info = rpc.get_multiple_accounts(&accounts).await?;
        for (index, account_info) in accounts_info.iter().enumerate() {
            if let Some(info) = account_info {
                let pubkey = accounts[index];
                let alt = AddressLookupTable::deserialize(&info.data)?;
                let address_lookup_table_account = AddressLookupTableAccount {
                    key: pubkey,
                    addresses: alt.addresses.into(),
                };
                alts.push(address_lookup_table_account);
            } else {
                println!("LUT 地址 {:?} 不存在", accounts[index]);
            }
        }
    }

    Ok(alts)
}

pub async fn build_versioned_transaction(
    rpc: Arc<RpcClient>,
    instructions: &[Instruction],
    user: &Pubkey,
    keypair: &Keypair,
    address_lookup_tables: Vec<Pubkey>,
    blockhash: Hash,
) -> Result<VersionedTransaction> {
    let alt = get_address_lookup(rpc.clone(), address_lookup_tables).await?;
    let v0_message = Message::try_compile(user, instructions, &alt, blockhash)?;
    let versioned_tx = VersionedTransaction::try_new(
        solana_sdk::message::VersionedMessage::V0(v0_message),
        &[keypair],
    )?;
    Ok(versioned_tx)
}

pub async fn append_swap_instructions(
    jup_client: Arc<JupiterSwapApiClient>,
    user: Pubkey,
    amount: u64,
    input_mint: Pubkey,
    output_mint: Pubkey,
    slippage_bps: u16,
    instructions: &mut Vec<Instruction>,
) -> Result<(u64, Vec<Pubkey>)> {
    let (out_amount, swap_response) = get_swap_ix(
        jup_client,
        user,
        amount,
        input_mint,
        output_mint,
        slippage_bps,
    )
    .await?;
    instructions.extend_from_slice(&swap_response.setup_instructions);
    instructions.push(swap_response.swap_instruction);
    Ok((out_amount, swap_response.address_lookup_table_addresses))
}

pub async fn send_tx_with_jito(
    tx: impl SerializableTransaction,
    jito: Arc<JitoJsonRpcSDK>,
) -> Result<Signature> {
    let serialized_tx = general_purpose::STANDARD.encode(bincode::serialize(&tx)?);
    let params = json!({
        "tx": serialized_tx
    });
    match jito.send_txn(Some(params.clone()), true).await {
        Ok(resp) => match resp["result"].as_str() {
            Some(signature) => {
                return Ok(Signature::from_str(signature)?);
            }
            None => Err(anyhow!("交易未响应")),
        },
        Err(e) => Err(e.into()),
    }
}

pub async fn send_tx(tx: impl SerializableTransaction, rpc: Arc<RpcClient>) -> Result<Signature> {
    match rpc.send_transaction(&tx).await {
        Ok(sig) => Ok(sig),
        Err(e) => Err(e.into()),
    }
}

pub async fn send_bundle(
    jito: &JitoJsonRpcSDK,
    bundle: Vec<impl SerializableTransaction>,
) -> Result<Option<String>> {
    let mut params = vec![];
    // 对每笔交易进行base64的编码
    for tx in bundle {
        params.push(bs58::encode(bincode::serialize(&tx)?).into_string());
    }
    let bundle = json!(params);
    let result = match jito.send_bundle(Some(bundle), None).await {
        Ok(resp) => match resp.get("result") {
            Some(bundle_id) => Some(bundle_id.as_str().unwrap().to_string()),
            None => None,
        },
        Err(_) => None,
    };
    Ok(result)
}

pub async fn get_price(client: Arc<Client>, mint: &str) -> Result<f32> {
    let resp = client
        .get(format!("https://api.jup.ag/price/v2?ids={}", mint))
        .send()
        .await?;

    if resp.status().is_success() {
        let resp_json: Value = resp.json().await?;

        if let Some(data) = resp_json.get("data") {
            if let Some(_data) = data.get(mint) {
                if let Some(price) = _data.get("price") {
                    return Ok(price.as_str().unwrap().parse::<f32>()?);
                }
            }
        }
    }
    Err(anyhow!("未获得代币 {} 的价格", mint))
}

use std::env;
/// 建立mysql连接
/// 需要预配置MYSQL_DATABASE_URL在.env中
pub fn establish_connection() -> diesel::MysqlConnection {
    let database_url = env::var("MYSQL_DATABASE_URL")
        .or_else(|_| env::var("DATABASE_URL"))
        .expect("DATABASE_URL must be set");
    <diesel::MysqlConnection as diesel::Connection>::establish(&database_url)
        .unwrap_or_else(|_| panic!("Error connecting to {}", database_url))
}

#[tokio::test]
async fn test() {
    let client = Arc::new(Client::new());
    let res = get_price(client, "JUPyiwrYJFskUPiHa7hkeR8VUtAeFoSYbKedZNsDvCN")
        .await
        .unwrap();
    println!("res {:?}", res);
}
