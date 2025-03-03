use std::{collections::HashMap, env, sync::Arc, time::Duration};

use anyhow::{anyhow, Context, Ok, Result};
use jito_sdk_rust::JitoJsonRpcSDK;
use jupiter_swap_api_client::JupiterSwapApiClient;
use reqwest::Client;
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::{pubkey::Pubkey, signature::Keypair, signer::Signer};
use tokio::sync::oneshot::{self, Sender};
use uuid::Uuid;

use crate::{common::utils::get_price, solana::swap::swap_with_tax};

#[derive(Debug, Clone)]
pub struct Order {
    pub order_id: Uuid,
    pub price: f32,
    pub input_mint: String,
    pub output_mint: String,
    pub amount: u64,
    pub slippage_bps: u16,
    pub tip_amount: Option<u64>,
}

pub struct OrderBook {
    pub orders: HashMap<Uuid, Order>,
    pub tokens: HashMap<Pubkey, f32>,
    /// 以基点的方式进行税收，100 => 1%
    pub tax_account: Pubkey,
    pub tax_bps: u16,
    pub cancel_tasks: HashMap<Uuid, Sender<()>>,
    pub http: Arc<Client>,
    pub jito: Arc<JitoJsonRpcSDK>,
    pub jup: Arc<JupiterSwapApiClient>,
    pub rpc: Arc<RpcClient>,
}

impl OrderBook {
    pub fn new() -> Result<OrderBook> {
        let rpc = Arc::new(RpcClient::new(env::var("RPC_URL")?));
        let http = Arc::new(Client::new());
        let jito = Arc::new(JitoJsonRpcSDK::new(&env::var("JITO_URL")?, None));
        let jup = Arc::new(JupiterSwapApiClient::new(env::var("JUP_URL")?));
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
        })
    }
    // 开单
    pub async fn place_order(
        &mut self,
        keypair_str: String,
        input_mint: String,
        output_mint: String,
        price: f32,
        amount: u64,
        slippage_bps: u16,
        tip_amount: Option<u64>,
    ) -> Result<Uuid> {
        let order_id = Uuid::new_v4();
        let order = Order {
            order_id,
            price,
            input_mint,
            output_mint,
            amount,
            slippage_bps,
            tip_amount,
        };

        self.orders.insert(order_id.clone(), order.clone());

        let (tx, rx) = oneshot::channel();
        self.cancel_tasks.insert(order_id.clone(), tx);

        let rpc = self.rpc.clone();
        let http = self.http.clone();
        let jito = self.jito.clone();
        let jup = self.jup.clone();
        let tax_account = self.tax_account;
        let tax_bps = self.tax_bps;
        let slippage_bps = order.slippage_bps;
        let keypair = Keypair::from_base58_string(&keypair_str);
        tokio::spawn(async move {
            let result = tokio::select! {
                _ = rx => {
                    Err(anyhow!("Task canceled"))
                }
                res = _order(
                    rpc,
                    jito,
                    jup,
                    &keypair,
                    tax_account,
                    tax_bps,
                    slippage_bps,
                    tip_amount,
                    http,
                    order,
                )
                => res,
            };
            if let Err(_) = result {
                println!("Deal task failed or was canceled");
            }
        });

        Ok(order_id)
    }

    // 取消订单
    pub async fn cancel_order(&mut self, order_id: Uuid) -> Result<()> {
        if let Some(tx) = self.cancel_tasks.remove(&order_id) {
            let _ = tx.send(());
            println!("订单 {:?} 成功取消", order_id);
            Ok(())
        } else {
            Err(anyhow!("订单未找到"))
        }
    }
}

async fn _order(
    rpc: Arc<RpcClient>,
    jito: Arc<jito_sdk_rust::JitoJsonRpcSDK>,
    jup: Arc<JupiterSwapApiClient>,
    user_keypair: &Keypair,
    tax_account: Pubkey,
    tax_bps: u16,
    slippage_bps: u16,
    tip_amount: Option<u64>,
    http: Arc<Client>,
    order: Order,
) -> Result<()> {
    let until_price = order.price;
    let input_mint: Pubkey = order.input_mint.parse()?;
    let output_mint: Pubkey = order.output_mint.parse()?;
    let amount = order.amount;
    loop {
        let now_price = get_price(http.clone(), &order.input_mint).await?;
        println!("now price {:?}", now_price);
        if (now_price - until_price).abs() < 0.001 {
            swap_with_tax(
                jup,
                rpc,
                jito,
                &user_keypair,
                tax_account,
                tax_bps,
                amount,
                input_mint,
                output_mint,
                slippage_bps,
                tip_amount,
            )
            .await
            .context("交易失败")?;
            return Ok(());
        }
        tokio::time::sleep(Duration::from_millis(800)).await;
    }
}
