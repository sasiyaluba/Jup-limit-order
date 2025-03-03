use anyhow::Context;
use limit_order::app::{cancel_order, place_order};
use limit_order::common::types::OrderBook;
use rocket::{launch, routes};
use tokio::sync::Mutex;

#[launch]
fn rocket() -> _ {
    dotenv::dotenv().ok();
    let order_book = OrderBook::new().context("环境变量配置失败").unwrap();
    let order_book_state = Mutex::new(order_book);

    // 配置并启动 Rocket 实例
    rocket::build()
        .manage(order_book_state) // 将 OrderBook 添加到 Rocket 的托管状态中
        .mount("/", routes![place_order, cancel_order]) // 挂载路由
}
