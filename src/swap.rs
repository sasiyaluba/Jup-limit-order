use std::sync::Arc;

use anyhow::Result;
use jito_sdk_rust::JitoJsonRpcSDK;
use jupiter_swap_api_client::JupiterSwapApiClient;
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::address_lookup_table::state::AddressLookupTable;
use solana_sdk::address_lookup_table::AddressLookupTableAccount;
use solana_sdk::compute_budget::ComputeBudgetInstruction;
use solana_sdk::message::v0::Message;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signature::Keypair;
use solana_sdk::signer::Signer;
use solana_sdk::system_instruction;
use solana_sdk::transaction::VersionedTransaction;

use crate::jito::get_tip_account;
use crate::jup::get_swap_ix;
use crate::utils::{build_versioned_transaction, send_bundle};
use crate::SOL;

pub async fn swap_with_tax(
    jup: Arc<JupiterSwapApiClient>,
    rpc: Arc<RpcClient>,
    jito: Arc<JitoJsonRpcSDK>,
    user_keypair: &Keypair,
    tax_account: Pubkey,
    tax_bps: u16,
    amount: u64,
    input_mint: Pubkey,
    output_mint: Pubkey,
    slippage_bps: u16,
    tip_amount: Option<u64>,
) -> Result<()> {
    // 如果输入是sol，则在swap之前进行收税
    let tax_before_swap = input_mint == SOL;

    let user = user_keypair.pubkey();

    let mut ixs = vec![];

    let (amount_specified, tax) = sub_tax(amount, tax_bps);

    let swap_amount = if tax_before_swap {
        println!("交易前税收，税收为{:?}", tax);
        ixs.push(system_instruction::transfer(&user, &tax_account, tax));
        amount_specified
    } else {
        amount
    };

    // 构造swap指令
    let (out_amount, swap_resp) = get_swap_ix(
        jup.clone(),
        user,
        swap_amount,
        input_mint,
        output_mint,
        slippage_bps,
    )
    .await?;

    // 插入swap指令
    ixs.extend_from_slice(&swap_resp.setup_instructions);
    ixs.push(swap_resp.swap_instruction);

    // 交易后收税
    if !tax_before_swap && out_amount != 0 {
        let tax = sub_tax(out_amount, tax_bps).1;
        println!("交易后税收，税收数量为 {:?}", tax);
        ixs.push(system_instruction::transfer(&user, &tax_account, tax));
    }

    if let Some(clean) = swap_resp.cleanup_instruction {
        ixs.push(clean);
    }

    let blockhash = rpc.get_latest_blockhash().await?;

    let versioned_tx = build_versioned_transaction(
        rpc.clone(),
        &ixs,
        &user,
        &user_keypair,
        swap_resp.address_lookup_table_addresses,
        blockhash,
    )
    .await?;

    if let Some(tip) = tip_amount {
        let tip_tx = VersionedTransaction::try_new(
            solana_sdk::message::VersionedMessage::V0(Message::try_compile(
                &user,
                &[system_instruction::transfer(
                    &user,
                    &get_tip_account()?,
                    tip,
                )],
                &[],
                blockhash,
            )?),
            &[user_keypair],
        )?;
        let bundle_id = send_bundle(&jito, vec![versioned_tx, tip_tx]).await?;
        if let Some(id) = bundle_id {
            let status = jito.get_bundle_statuses(vec![id]).await?;
            println!("status {:?}", status);
        }
    } else {
        rpc.send_and_confirm_transaction_with_spinner(&versioned_tx)
            .await?;
    }
    Ok(())
}

pub async fn get_address_lookup_table_accounts(
    rpc: &RpcClient,
    keys: Vec<Pubkey>,
) -> Result<Vec<AddressLookupTableAccount>> {
    // 获取多个账户信息
    let account_infos = rpc.get_multiple_accounts(&keys).await?;
    let mut lookup_table_accounts = vec![];
    // 使用迭代器处理结果，构建 AddressLookupTableAccount 数组
    for (idx, acc) in account_infos.iter().enumerate() {
        if let Some(_acc) = acc {
            lookup_table_accounts.push(AddressLookupTableAccount {
                key: keys[idx],
                addresses: AddressLookupTable::deserialize(&_acc.data)?
                    .addresses
                    .to_vec(),
            });
        }
    }
    Ok(lookup_table_accounts)
}

pub fn sub_tax(amount: u64, tax_bps: u16) -> (u64, u64) {
    let tax = (amount * tax_bps as u64) / 10000;
    (amount - tax, tax)
}

#[cfg(test)]
mod example {
    use std::{env, sync::Arc};

    use anyhow::Result;
    use jito_sdk_rust::JitoJsonRpcSDK;
    use jupiter_swap_api_client::JupiterSwapApiClient;
    use solana_client::nonblocking::rpc_client::RpcClient;
    use solana_sdk::{
        pubkey, signature::Keypair, signer::Signer, system_instruction, transaction::Transaction,
    };

    use crate::{swap::swap_with_tax, SOL};
    #[tokio::test]
    async fn test1() -> Result<()> {
        dotenv::dotenv().ok();
        // 测试 jup兑换sol
        let sol = SOL;
        let _jup = pubkey!("JUPyiwrYJFskUPiHa7hkeR8VUtAeFoSYbKedZNsDvCN");

        // 客户端
        let jup = Arc::new(JupiterSwapApiClient::new(env::var("JUP_URL")?));
        let jito = Arc::new(JitoJsonRpcSDK::new(&env::var("JITO_URL")?, None));
        let rpc = Arc::new(RpcClient::new(env::var("RPC_URL")?));

        // 私钥
        let user_keypair = Keypair::from_base58_string("");
        println!("用户地址 {:?}", user_keypair.pubkey());
        let tax_keypair = Keypair::from_base58_string(&env::var("TAX_ACCOUNT")?);
        println!("tax地址 {:?}", tax_keypair.pubkey());

        swap_with_tax(
            jup,
            rpc,
            jito,
            &user_keypair,
            tax_keypair.pubkey(),
            100,
            1000,
            _jup,
            sol,
            100,
            None,
        )
        .await?;

        Ok(())
    }
}
