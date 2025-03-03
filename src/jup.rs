use std::{env, str::FromStr, sync::Arc};

use anyhow::Result;
use jito_sdk_rust::JitoJsonRpcSDK;
use jupiter_swap_api_client::{
    quote::QuoteRequest,
    swap::{SwapInstructionsResponse, SwapRequest},
    transaction_config::TransactionConfig,
    JupiterSwapApiClient,
};

use solana_sdk::pubkey::Pubkey;

/// jup 交易
/// use -> 交易发起者
pub async fn get_swap_ix(
    jup: Arc<JupiterSwapApiClient>,
    user: Pubkey,
    amount: u64,
    input_mint: Pubkey,
    output_mint: Pubkey,
    slippage_bps: u16,
) -> Result<(u64, SwapInstructionsResponse)> {
    let quote_request = QuoteRequest {
        amount,
        input_mint,
        output_mint,
        slippage_bps,
        ..QuoteRequest::default()
    };
    let quote_response = jup.quote(&quote_request).await.unwrap();
    println!("quote resp {:?}", quote_response);
    let out_amount = quote_response.out_amount;
    let swap_ix_response = jup
        .swap_instructions(&SwapRequest {
            user_public_key: user,
            quote_response,
            config: TransactionConfig::default(),
        })
        .await?;
    Ok((out_amount, swap_ix_response))
}
