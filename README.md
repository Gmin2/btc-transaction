# Bitcoin Transaction Chain Demonstration

This project demonstrates creating a chain of Bitcoin transactions using the rust-bitcoin library.
It creates three transactions:
1. A coinbase transaction (mining reward)
2. A transaction spending the coinbase output
3. A transaction spending the previous transaction's output

## Prerequisites

- Docker
- Rust

## Running the script with docker

### Using Docker

```bash
chmod +x init.sh
./init.sh
```

## Running locally in linux
If you prefer to run without Docker:

1. Install Bitcoin Core
2. Configure Bitcoin Core for regtest in the 
bitcoin.conf:
```
regtest=1
txindex=1
server=1
rpcuser=bitcoinrpc
rpcpassword=rpcpassword
```

3. Start Bitcoin Core: 
```shell
bitcoind -daemon
```

5. Build the application: 
```shell
cargo build --release
```

6. Run the application: 
```shell
./target/release/p2pool-test
```

