use limit_order::{
    app::{cancel_order, place_order},
    types::init_order_book,
};
use rocket::routes;

#[rocket::main]
async fn main() {
    dotenv::dotenv().ok();
    rocket::build()
        .manage(tokio::sync::Mutex::new(init_order_book().unwrap()))
        .mount("/", routes![place_order, cancel_order])
        .launch()
        .await
        .unwrap();
}
