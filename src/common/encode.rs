use crate::common::AES_KEY;
use aes_gcm::aead::{Aead, KeyInit};
use aes_gcm::{Aes256Gcm, Key, Nonce};
use base64::engine::general_purpose;
use base64::Engine;
// AES-GCM 256-bit 密钥
use anyhow::{anyhow, Result};
use rand::Rng;
/// 使用 AES-256-GCM 算法对输入数据进行加密，并将结果编码为 Base64 字符串。
///
/// 该函数首先生成一个随机的 12 字节 nonce，将其与加密后的密文拼接在一起，然后将整个结果编码为 Base64 字符串。
/// 加密使用的密钥是从 `crate::common::AES_KEY` 导入的静态 256 位密钥。
///
/// # 参数
/// * `plaintext` - 要加密的明文数据，以字节数组形式传入。
///
/// # 返回值
/// 返回一个 Base64 编码的字符串，包含 nonce 和密文。
///
/// # 异常
/// 如果加密过程中发生错误（例如输入数据过长或密钥无效），函数会通过 `expect` panic。
/// 在生产环境中，建议使用 `Result` 类型替换 `expect` 以更好地处理错误。
///
/// # 示例
/// ```rust
/// let plaintext = b"my secret data";
/// let encrypted = encrypt(plaintext);
/// println!("Encrypted: {}", encrypted);
/// ```
pub fn encrypt(plaintext: &[u8]) -> String {
    let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(&AES_KEY)); // 添加泛型提示
    let nonce_bytes: [u8; 12] = rand::thread_rng().gen(); // 生成随机 nonce
    let nonce = Nonce::from_slice(&nonce_bytes);
    let mut ciphertext = cipher
        .encrypt(nonce, plaintext)
        .expect("encryption failure");
    ciphertext.splice(0..0, nonce_bytes.iter().cloned()); // 在密文前面加上 nonce
    general_purpose::STANDARD.encode(&ciphertext)
}

/// 解密使用 AES-256-GCM 算法加密并以 Base64 编码的密文，返回解密后的字符串。
///
/// 该函数首先将输入的 Base64 字符串解码为字节数组，从中提取前 12 字节作为 nonce，
/// 然后使用剩余的字节作为密文进行解密。解密后的字节数组会被转换为 UTF-8 字符串。
/// 解密使用的密钥是从 `crate::common::AES_KEY` 导入的静态 256 位密钥。
///
/// # 参数
/// * `ciphertext_bs64` - Base64 编码的密文字符串，包含 nonce 和加密数据。
///
/// # 返回值
/// 返回一个 `Result<String>`，其中：
/// - `Ok(String)`: 成功解密后的明文字符串。
/// - `Err(anyhow::Error)`: 如果解密失败（例如 Base64 解码失败、nonce 无效或密文损坏）。
///
/// # 错误
/// - 如果 Base64 解码失败，会通过 `expect` panic（建议在生产环境中替换为错误返回）。
/// - 如果解密失败（例如密文被篡改或密钥不匹配），返回 `Err` 并包含错误信息。
/// - 如果解密结果不是有效的 UTF-8 字符串，使用 `from_utf8_lossy` 可能会导致部分数据丢失。
///
/// # 示例
/// ```rust
/// let encrypted = "some_base64_encoded_string";
/// match decrypt(encrypted) {
///     Ok(plain) => println!("Decrypted: {}", plain),
///     Err(e) => eprintln!("Decryption failed: {:?}", e),
/// }
/// ```
pub fn decrypt(ciphertext_bs64: &str) -> Result<String> {
    let ciphertext = general_purpose::STANDARD
        .decode(ciphertext_bs64)
        .expect("base64 decode failure");

    let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(&AES_KEY)); // 添加泛型提示
    let nonce = Nonce::from_slice(&ciphertext[..12]); // 提取 nonce
    let res = cipher
        .decrypt(nonce, &ciphertext[12..])
        .map_err(|e| anyhow!("解码私钥失败 {:?}", e))?;
    Ok(String::from_utf8_lossy(&res).to_string())
}
