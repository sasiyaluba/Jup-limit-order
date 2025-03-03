use std::{collections::HashMap, env, sync::Arc};

use crate::types::OrderBook;
use anyhow::Result;
use jito_sdk_rust::JitoJsonRpcSDK;
use jupiter_swap_api_client::JupiterSwapApiClient;
use reqwest::Client;
use rocket::{post, serde::json::Json, State};
use serde::{Deserialize, Serialize};
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::{
    pubkey::Pubkey, signature::Keypair, signer::Signer, system_instruction,
    transaction::Transaction,
};
use uuid::Uuid;

#[derive(Deserialize)]
struct PlaceOrderRequest {
    user: String,
    input_mint: String,
    output_mint: String,
    price: f32,
    amount: u64,
    slippage_bps: u16,
    tip_amount: Option<u64>,
}
#[derive(Serialize, Deserialize)]
struct ApiResponse<T> {
    success: bool,
    data: Option<T>,
    error: Option<String>,
}

#[derive(Serialize, Deserialize)]
struct Data {
    tx: Transaction,
    order_id: Uuid,
}

// 用于 POST /cancel_order 的请求体
#[derive(Deserialize)]
struct CancelOrderRequest {
    order_id: Uuid,
}

// 开单 API
#[post("/place_order", data = "<request>")]
pub async fn place_order(
    request: Json<PlaceOrderRequest>,
    order_book: &State<tokio::sync::Mutex<OrderBook>>,
) -> Json<ApiResponse<Data>> {
    let mut order_book = order_book.lock().await;
    let ix = system_instruction::transfer(
        &request.user.parse().unwrap(),
        &order_book.keypair.pubkey(),
        request.amount,
    );
    let tx = Transaction::new_with_payer(&[ix], Some(&request.user.parse().unwrap()));
    let result = order_book
        .place_order(
            request.user.clone(),
            request.input_mint.clone(),
            request.output_mint.clone(),
            request.price,
            request.amount,
            request.slippage_bps,
            request.tip_amount,
        )
        .await;

    match result {
        Ok(order_id) => Json(ApiResponse {
            success: true,
            data: Some(Data { tx, order_id }),
            error: None,
        }),
        Err(e) => Json(ApiResponse {
            success: false,
            data: None,
            error: Some(e.to_string()),
        }),
    }
}

// 撤单 API
#[post("/cancel_order", data = "<request>")]
pub async fn cancel_order(
    request: Json<CancelOrderRequest>,
    order_book: &State<tokio::sync::Mutex<OrderBook>>,
) -> Json<ApiResponse<String>> {
    let mut order_book = order_book.lock().await;
    let result = order_book.cancel_order(request.order_id).await;

    match result {
        Ok(()) => Json(ApiResponse {
            success: true,
            data: Some("Order canceled successfully".to_string()),
            error: None,
        }),
        Err(e) => Json(ApiResponse {
            success: false,
            data: None,
            error: Some(e.to_string()),
        }),
    }
}

pub fn init_order_book() -> Result<OrderBook> {
    let rpc = Arc::new(RpcClient::new(env::var("RPC_URL")?));
    let http = Arc::new(Client::new());
    let jito = Arc::new(JitoJsonRpcSDK::new(&env::var("JITO_URL")?, None));
    let jup = Arc::new(JupiterSwapApiClient::new(env::var("JUP_URL")?));
    let keypair = Arc::new(Keypair::from_base58_string(&env::var("ROUTE_PK")?)); // 替换为实际密钥对
    let tax_account = env::var("TAX_ACCOUNT")?.parse::<Pubkey>()?; // 替换为实际税收账户
    let tax_bps = env::var("TAX_BPS")?.parse::<u16>()?; // 替换为实际税收账户

    Ok(OrderBook {
        orders: HashMap::new(),
        tokens: HashMap::new(),
        tax_account,
        tax_bps,
        cancel_tasks: HashMap::new(),
        http,
        jito,
        jup,
        rpc,
        keypair,
    })
}
