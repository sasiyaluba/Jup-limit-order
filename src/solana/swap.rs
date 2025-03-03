use std::sync::Arc;

use anyhow::{anyhow, Result};
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

use crate::common::utils::{build_versioned_transaction, send_bundle};
use crate::SOL;

use super::jito::get_tip_account;
use super::jup::get_swap_ix;

/// 在 Solana 区块链上执行带有税收的代币交换操作
///
/// 该函数通过 Jupiter Swap API 执行代币交换，并根据指定的税收百分比（以基点为单位）在交易前或交易后扣除税收。
/// 支持 Jito 捆绑交易（bundle transaction）和可选的 tip 支付。
///
/// # 参数
/// - `jup`: `Arc<JupiterSwapApiClient>` - Jupiter Swap API 客户端的线程安全引用
/// - `rpc`: `Arc<RpcClient>` - Solana RPC 客户端的线程安全引用
/// - `jito`: `Arc<JitoJsonRpcSDK>` - Jito SDK 的线程安全引用，用于捆绑交易
/// - `user_keypair`: `&Keypair` - 用户的密钥对，用于签名交易
/// - `tax_account`: `Pubkey` - 接收税收的账户公钥
/// - `tax_bps`: `u16` - 税收百分比，以基点表示（1 bps = 0.01%，10000 bps = 100%）
/// - `amount`: `u64` - 输入代币的总量
/// - `input_mint`: `Pubkey` - 输入代币的 mint 地址
/// - `output_mint`: `Pubkey` - 输出代币的 mint 地址
/// - `slippage_bps`: `u16` - 允许的滑点，以基点表示
/// - `tip_amount`: `Option<u64>` - 可选的 tip 金额，用于 Jito 捆绑交易
///
/// # 返回值
/// - `Result<()>` - 执行成功返回 `Ok(())`，失败返回错误
///
/// # 逻辑流程
/// 1. 判断税收是在交易前（输入为 SOL 时）还是交易后扣除
/// 2. 计算税收金额并构造税收转账指令
/// 3. 调用 Jupiter Swap API 获取交换指令
/// 4. 根据税收时机添加税收指令
/// 5. 构建并模拟执行交易
/// 6. 根据是否提供 tip，使用 Jito 捆绑交易或普通交易发送
///
/// # 示例
/// ```rust
/// let result = swap_with_tax(
///     jup.clone(),
///     rpc.clone(),
///     jito.clone(),
///     &keypair,
///     tax_account,
///     100, // 1% 税收
///     1_000_000, // 输入金额
///     SOL,
///     usdc_mint,
///     50, // 0.5% 滑点
///     Some(1_000_000), // tip 金额
/// ).await;
/// ```
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

    println!("开始模拟执行");
    let resp = rpc.simulate_transaction(&versioned_tx).await?;
    if resp.value.err.is_some() {
        println!("模拟执行失败，错误为 {:?}", resp);
        return Err(anyhow!("模拟执行失败"));
    } else {
        println!("模拟执行成功，开始交易");
    }

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

/// 获取多个地址查找表账户的信息
///
/// 从 Solana 区块链批量查询账户数据，并解析为 `AddressLookupTableAccount` 结构。
///
/// # 参数
/// - `rpc`: `&RpcClient` - Solana RPC 客户端引用
/// - `keys`: `Vec<Pubkey>` - 要查询的地址查找表公钥列表
///
/// # 返回值
/// - `Result<Vec<AddressLookupTableAccount>>` - 返回解析后的地址查找表账户列表
///
/// # 逻辑流程
/// 1. 使用 RPC 客户端批量获取账户信息
/// 2. 遍历查询结果，解析每个账户的地址查找表数据
/// 3. 构造并返回 `AddressLookupTableAccount` 数组
///
/// # 示例
/// ```rust
/// let lookup_tables = get_address_lookup_table_accounts(&rpc, vec![table_pubkey]).await?;
/// ```
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

/// 计算扣除税收后的金额和税收金额
///
/// 根据给定的金额和税收基点，计算实际交易金额和税收部分。
///
/// # 参数
/// - `amount`: `u64` - 总金额
/// - `tax_bps`: `u16` - 税收百分比，以基点表示（1 bps = 0.01%）
///
/// # 返回值
/// - `(u64, u64)` - 元组，第一个元素为扣税后的金额，第二个元素为税收金额
///
/// # 计算公式
/// - 税收 = (amount * tax_bps) / 10000
/// - 扣税后金额 = amount - 税收
///
/// # 示例
/// ```rust
/// let (net_amount, tax) = sub_tax(1_000_000, 100); // 1% 税收
/// assert_eq!(net_amount, 990_000);
/// assert_eq!(tax, 10_000);
/// ```
pub fn sub_tax(amount: u64, tax_bps: u16) -> (u64, u64) {
    let tax = (amount * tax_bps as u64) / 10000;
    (amount - tax, tax)
}
