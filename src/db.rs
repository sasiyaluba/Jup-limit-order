use anyhow::Result;
use diesel::query_dsl::methods::FilterDsl;
use diesel::{Insertable, Queryable};
use diesel::{MysqlConnection, RunQueryDsl};
use solana_sdk::signature::Keypair;

use crate::utils::establish_connection;

// 表定义保持不变
diesel::table! {
    KeyRecord (id) {
        id -> Integer,
        pubkey -> Varchar,
        prikey -> Varchar,
    }
}

// 插入数据的结构体保持不变
#[derive(Insertable)]
#[diesel(table_name = KeyRecord)]
struct NewKeyRecord<'a> {
    pubkey: &'a str,
    prikey: &'a str,
}

// 插入函数保持不变
pub fn insert_keypair(public_key: &str, private_key: &str) -> Result<usize> {
    use crate::db::KeyRecord::dsl::*;
    let mut conn = establish_connection();
    let new_record = NewKeyRecord {
        pubkey: public_key,
        prikey: private_key,
    };

    diesel::insert_into(KeyRecord)
        .values(&new_record)
        .execute(&mut conn)
        .map_err(Into::into)
}

// 修改后的查询函数，只返回私钥字符串
pub fn query_private_key(target_pubkey: &str) -> Result<Keypair> {
    let mut conn = establish_connection();

    use crate::db::KeyRecord::dsl::*;
    use diesel::ExpressionMethods;
    let (_, _, pk) = KeyRecord
        .filter(pubkey.eq(target_pubkey))
        .first::<(i32, String, String)>(&mut conn)?;
    let keypair = Keypair::from_base58_string(&pk);
    Ok(keypair)
}
