use std::str::FromStr;

use anyhow::{anyhow, Result};
use rand::{rng, seq::IteratorRandom};
use solana_sdk::pubkey::Pubkey;

pub fn get_tip_account() -> Result<Pubkey> {
    let accounts = [
        "ADaUMid9yfUytqMBgopwjb2DTLSokTSzL1zt6iGPaS49",
        "DttWaMuVvTiduZRnguLF7jNxTgiMBZ1hyAumKUiL2KRL",
        "Cw8CFyM9FkoMi7K7Crf6HNQqf4uEMzpKw6QNghXLvLkY",
        "ADuUkR4vqLUMWXxW9gh6D6L8pMSawimctcNZ5pGwDcEt",
        "HFqU5x63VTqvQss8hp11i4wVV8bD44PvwucfZ2bU7gRe",
        "DfXygSm4jCyNCybVYYK6DwvWqjKee8pbDmJGcLWNDXjh",
        "3AVi9Tg9Uo68tJfuvoKvqKNWKkC5wPdSSdeBnizKZ6jT",
        "96gYZGLnJYVFmbjzopPSU6QiEV5fGqZNyN9nmNhvrZU5",
    ];
    let mut rng = rng();
    match accounts.iter().choose(&mut rng) {
        Some(acc) => Ok(Pubkey::from_str(acc)?),
        None => Err(anyhow!("jito: no tip accounts available")),
    }
}
