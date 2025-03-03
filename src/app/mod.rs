use std::sync::atomic;

use anyhow::anyhow;
use rocket::{post, serde::json::Json, State};
use serde::{Deserialize, Serialize};
use solana_sdk::{pubkey::Pubkey, signature::Keypair, signer::Signer};
use tokio::sync::Mutex;
use uuid::Uuid;

use crate::common::{
    encode::{decrypt, encrypt},
    types::{Order, OrderBook},
};

#[derive(Deserialize)]
pub struct PlaceOrderRequest {
    /// 输出代币
    pub input_mint: String,
    /// 输出代币
    pub output_mint: String,
    /// 卖出价格
    pub price: f32,
    /// 数量
    pub amount: u64,
    /// 滑点
    pub slippage_bps: u16,
    /// 是否有小费给jito
    pub tip_amount: Option<u64>,
    /// 加密后的pk
    pub encrypt_pk: String,
}

#[derive(Serialize, Deserialize)]
struct ApiResponse<T> {
    success: bool,
    data: Option<T>,
    error: Option<String>,
}
/// 创建新订单的 API 端点。
///
/// 该端点接受一个下单请求，解密私钥后在订单簿中创建订单，并返回订单的 UUID。
///
/// # 参数
/// * `request` - 下单请求的 JSON 数据，包含交易参数和加密私钥。
/// * `order_book` - 订单簿的共享状态，使用 `Mutex` 保护以支持并发访问。
///
/// # 返回值
/// 返回一个 `Json<ApiResponse<Uuid>>`，其中：
/// - `success: true` 和 `data: Some(uuid)` 表示订单创建成功。
/// - `success: false` 和 `error: Some(msg)` 表示创建失败。
///
/// # 示例
/// ```bash
/// curl -X POST http://localhost:8000/place_order \
///   -H 'Content-Type: application/json' \
///   -d '{"input_mint": "So11111111111111111111111111111111111111112", "output_mint": "JUPyiwrYJFskUPiHa7hkeR8VUtAeFoSYbKedZNsDvCN", "price": 0.5, "amount": 1000000000, "slippage_bps": 50, "encrypt_pk": "SGVsbG8gV29ybGQ="}'
/// ```
/// 响应：
/// ```json
/// {
///     "success": true,
///     "data": "550e8400-e29b-41d4-a716-446655440000",
///     "error": null
/// }
/// ```
#[post("/place_order", data = "<request>")]
pub async fn place_order(
    request: Json<PlaceOrderRequest>,
    order_book: &State<Mutex<OrderBook>>,
) -> Json<ApiResponse<Uuid>> {
    match decrypt(&request.encrypt_pk) {
        Ok(prik) => {
            let mut order_book = order_book.lock().await;
            let result = order_book
                .place_order(
                    prik,
                    request.input_mint.clone(),
                    request.output_mint.clone(),
                    request.price,
                    request.amount,
                    request.slippage_bps,
                    request.tip_amount,
                )
                .await;

            match result {
                Ok(id) => Json(ApiResponse {
                    success: true,
                    data: Some(id),
                    error: None,
                }),
                Err(e) => Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some(format!("开单失败 {:?}", e)),
                }),
            }
        }
        Err(e) => Json(ApiResponse {
            success: false,
            data: None,
            error: Some(format!("私钥解析失败")),
        }),
    }
}

#[derive(Deserialize)]
struct CancelOrderRequest {
    pub order_id: Uuid,
}

/// 取消订单的 API 端点。
///
/// 该端点接受一个撤单请求，根据订单 ID 在订单簿中取消指定订单。
///
/// # 参数
/// * `request` - 撤单请求的 JSON 数据，包含订单 ID。
/// * `order_book` - 订单簿的共享状态，使用 `Mutex` 保护以支持并发访问。
///
/// # 返回值
/// 返回一个 `Json<ApiResponse<String>>`，其中：
/// - `success: true` 和 `data: Some("撤单成功")` 表示订单取消成功。
/// - `success: false` 和 `error: Some(msg)` 表示取消失败。
///
/// # 示例
/// ```bash
/// curl -X POST http://localhost:8000/cancel_order \
///   -H 'Content-Type: application/json' \
///   -d '{"order_id": "550e8400-e29b-41d4-a716-446655440000"}'
/// ```
/// 响应：
/// ```json
/// {
///     "success": true,
///     "data": "撤单成功",
///     "error": null
/// }
/// ```
#[post("/cancel_order", data = "<request>")]
pub async fn cancel_order(
    request: Json<CancelOrderRequest>,
    order_book: &State<Mutex<OrderBook>>,
) -> Json<ApiResponse<String>> {
    let mut order_book = order_book.lock().await;
    let result = order_book.cancel_order(request.order_id).await;

    match result {
        Ok(()) => Json(ApiResponse {
            success: true,
            data: Some("撤单成功".to_string()),
            error: None,
        }),
        Err(e) => Json(ApiResponse {
            success: false,
            data: None,
            error: Some(e.to_string()),
        }),
    }
}
