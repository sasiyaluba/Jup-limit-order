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
