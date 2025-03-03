# 开单测试

## 有 tip

    curl -X POST \
    http://localhost:8000/place_order \
    -H 'Content-Type: application/json' \
    -d '{
        "input_mint": "JUPyiwrYJFskUPiHa7hkeR8VUtAeFoSYbKedZNsDvCN",
        "output_mint": "So11111111111111111111111111111111111111112",
        "price":0.738401,
        "amount": 1000,
        "slippage_bps": 50,
        "encrypt_pk": "c3wVtufBPy2EHVAP/RjjQoZOb8wzyAgtxp0mPXwJ4CO7K53ot5t4hkKNjYzepxZxzuPB+Q8xFt3ft11xzISVdWly7VKqX6h2QOLzCT7GLWCwcopyNFa0jMCSUoUUBLHCAmAYOulDKV+q/2oaK6iSs9QBxHo=",
        "tip_amount":1000
    }'

## 无 tip

    curl -X POST \
    http://localhost:8000/place_order \
    -H 'Content-Type: application/json' \
    -d '{
        "input_mint": "JUPyiwrYJFskUPiHa7hkeR8VUtAeFoSYbKedZNsDvCN",
        "output_mint": "So11111111111111111111111111111111111111112",
        "price":0.731976,
        "amount": 1000,
        "slippage_bps": 50,
        "encrypt_pk": "c3wVtufBPy2EHVAP/RjjQoZOb8wzyAgtxp0mPXwJ4CO7K53ot5t4hkKNjYzepxZxzuPB+Q8xFt3ft11xzISVdWly7VKqX6h2QOLzCT7GLWCwcopyNFa0jMCSUoUUBLHCAmAYOulDKV+q/2oaK6iSs9QBxHo="
    }'

# 撤单

    curl -X POST \
    http://localhost:8000/cancel_order \
    -H 'Content-Type: application/json' \
    -d '{
    "order_id": "3e702c25-9c50-422d-a9dd-949df32b26c5"
    }'

# 注意

在`common mod.rs`需要配置真正的加密私钥

# 加解密函数的 python，js 语言重构

```js
const crypto = require("crypto");

const AES_KEY = Buffer.from("32_byte_key_here_1234567890abcd", "utf8");

function encrypt(plaintext) {
  const nonce = crypto.randomBytes(12);
  const cipher = crypto.createCipheriv("aes-256-gcm", AES_KEY, nonce);
  const ciphertext = Buffer.concat([cipher.update(plaintext), cipher.final()]);
  const tag = cipher.getAuthTag();
  const result = Buffer.concat([nonce, ciphertext, tag]);
  return result.toString("base64");
}

function decrypt(ciphertext_bs64) {
  const data = Buffer.from(ciphertext_bs64, "base64");
  const nonce = data.slice(0, 12);
  const ciphertext = data.slice(12, -16);
  const tag = data.slice(-16);
  const decipher = crypto.createDecipheriv("aes-256-gcm", AES_KEY, nonce);
  decipher.setAuthTag(tag);
  const plaintext = Buffer.concat([
    decipher.update(ciphertext),
    decipher.final(),
  ]);
  return plaintext.toString("utf8");
}
```

```python
from Crypto.Cipher import AES
import base64
import os

AES_KEY = b"32_byte_key_here_1234567890abcd"  # 32 字节密钥

def encrypt(plaintext: bytes) -> str:
    cipher = AES.new(AES_KEY, AES.MODE_GCM)
    ciphertext, tag = cipher.encrypt_and_digest(plaintext)
    result = cipher.nonce + ciphertext + tag  # Nonce + 密文 + 标签
    return base64.b64encode(result).decode("utf-8")

def decrypt(ciphertext_bs64: str) -> str:
    data = base64.b64decode(ciphertext_bs64)
    nonce, ciphertext = data[:12], data[12:-16]  # 提取 Nonce 和密文（标签长度 16 字节）
    tag = data[-16:]
    cipher = AES.new(AES_KEY, AES.MODE_GCM, nonce=nonce)
    plaintext = cipher.decrypt_and_verify(ciphertext, tag)
    return plaintext.decode("utf-8")
```
