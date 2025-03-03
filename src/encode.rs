use aes_gcm::aead::{Aead, KeyInit};
use aes_gcm::{Aes256Gcm, Key, Nonce}; // AES-GCM 256-bit 密钥
use rand::Rng;

pub fn encrypt(key: &[u8; 32], plaintext: &[u8]) -> Vec<u8> {
    let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(key)); // 添加泛型提示
    let nonce_bytes: [u8; 12] = rand::thread_rng().gen(); // 生成随机 nonce
    let nonce = Nonce::from_slice(&nonce_bytes);
    let mut ciphertext = cipher
        .encrypt(nonce, plaintext)
        .expect("encryption failure");
    ciphertext.splice(0..0, nonce_bytes.iter().cloned()); // 在密文前面加上 nonce
    ciphertext
}

// 解密函数
pub fn decrypt(key: &[u8; 32], ciphertext: &[u8]) -> Vec<u8> {
    let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(key)); // 添加泛型提示
    let nonce = Nonce::from_slice(&ciphertext[..12]); // 提取 nonce
    cipher
        .decrypt(nonce, &ciphertext[12..])
        .expect("decryption failure")
}
fn generate_random_key() -> [u8; 32] {
    let mut key = [0u8; 32];
    rand::thread_rng().fill(&mut key);
    key
}

#[cfg(test)]
mod test {
    use super::{encrypt, generate_random_key};

    fn test() {
        let key = generate_random_key();
    }
}
