use bitcoin::consensus::encode::serialize;
use bitcoin::key::{PrivateKey, PublicKey, Secp256k1};
use bitcoin::{
    Address, Amount, Network, OutPoint, ScriptBuf, Sequence, Transaction, TxIn, TxOut, Witness,
};
use bitcoincore_rpc::{Auth, Client as RpcClient, RpcApi};
use secp256k1::{rand, SecretKey};
use std::str::FromStr;
use std::{thread, time};

fn main() {
    // Connect to Bitcoin node
    let rpc_url = "http://bitcoin:18443";
    let client = RpcClient::new(
        rpc_url,
        Auth::UserPass("alice".to_string(), "password".to_string()),
    )
    .expect("Failed to create RPC client");

    println!("Connected to Bitcoin node");

    if let Err(e) = initialize_wallet(&client) {
        println!("Failed to initialize wallet: {:?}", e);
        return;
    }

    // Create key pair for our transactions
    let secp = Secp256k1::new();
    let secret_key = SecretKey::new(&mut rand::thread_rng());
    let private_key = PrivateKey::new(secret_key, Network::Regtest);
    let public_key = PublicKey::from_private_key(&secp, &private_key);

    let address = Address::p2pkh(&public_key, Network::Regtest);
    println!("Generated address: {}", address);

    // Ensure we have enough blocks to generate mature coins
    ensure_blocks_mined(&client).unwrap();

    // Generate coinbase transaction by mining a block
    let coinbase_txid = mine_block(&client, &address).unwrap();
    println!("Mined block with coinbase txid: {}", coinbase_txid);

    // Mining 100 additional blocks to mature the coinbase
    let temp_address = client
        .get_new_address(None, None)
        .unwrap()
        .require_network(Network::Regtest)
        .unwrap();
    client.generate_to_address(100, &temp_address).unwrap();
    println!("Mined 100 blocks to mature the coinbase");

    // Wait for the block to be properly processed
    thread::sleep(time::Duration::from_secs(1));

    // Create and submit first transaction (spending coinbase)
    let first_tx =
        create_first_transaction(&client, &coinbase_txid, &private_key, &public_key, &secp)
            .unwrap();
    println!("First transaction created: {}", first_tx);

    // Generate a block to confirm first transaction
    generate_block(&client).unwrap();
    println!("Generated block to confirm first transaction");

    // Wait for block to be processed
    thread::sleep(time::Duration::from_secs(1));

    // Create and submit second transaction (spending first tx)
    let second_tx =
        create_second_transaction(&client, &first_tx, &private_key, &public_key, &secp).unwrap();
    println!("Second transaction created: {}", second_tx);

    // Generate a final block to confirm second transaction
    generate_block(&client).unwrap();
    println!("Generated block to confirm second transaction");

    println!("Transaction chain complete!");
    println!("Summary:");
    println!("1. Coinbase transaction: {}", coinbase_txid);
    println!("2. First transaction: {}", first_tx);
    println!("3. Second transaction: {}", second_tx);
}

// fn initialize_wallet(client: &RpcClient) -> Result<(), bitcoincore_rpc::Error> {
//     // Check if wallet is already loaded by listing wallets
//     let wallets = client.list_wallets()?;

//     if wallets.contains(&"mywallet".to_string()) {
//         println!("Wallet 'mywallet' is already loaded");
//         return Ok(());
//     }

//     // Try to load the wallet first
//     match client.load_wallet("mywallet") {
//         Ok(_) => {
//             println!("Loaded existing wallet 'mywallet'");
//             return Ok(());
//         }
//         Err(e) => {
//             // If wallet doesn't exist, try to create it
//             if e.to_string().contains("not found") {
//                 match client.create_wallet("mywallet", None, None, None, None) {
//                     Ok(_) => {
//                         println!("Created new wallet 'mywallet'");
//                         return Ok(());
//                     }
//                     Err(create_err) => {
//                         println!("Error creating wallet: {:?}", create_err);
//                         return Err(create_err);
//                     }
//                 }
//             } else if e.to_string().contains("already loaded") {
//                 // Wallet is already loaded
//                 println!("Wallet 'mywallet' is already loaded");
//                 return Ok(());
//             } else {
//                 // Some other error occurred
//                 println!("Error loading wallet: {:?}", e);
//                 return Err(e);
//             }
//         }
//     }
// }

fn initialize_wallet(client: &RpcClient) -> Result<(), bitcoincore_rpc::Error> {
    println!("Initializing wallet...");
    
    // Try to create the wallet first
    match client.create_wallet("mywallet", None, None, None, None) {
        Ok(_) => {
            println!("Created new wallet 'mywallet'");
            return Ok(());
        },
        Err(e) => {
            // If wallet already exists, try to load it
            if e.to_string().contains("already exists") {
                println!("Wallet 'mywallet' already exists, trying to load it");
                match client.load_wallet("mywallet") {
                    Ok(_) => {
                        println!("Loaded existing wallet 'mywallet'");
                        return Ok(());
                    },
                    Err(load_err) => {
                        // Check if wallet is already loaded
                        if load_err.to_string().contains("already loaded") {
                            println!("Wallet 'mywallet' is already loaded");
                            return Ok(());
                        } else {
                            println!("Error loading wallet: {:?}", load_err);
                            return Err(load_err);
                        }
                    }
                }
            } else {
                println!("Error creating wallet: {:?}", e);
                // Try to continue anyway, the wallet might be loadable
                let wallets = client.list_wallets()?;
                if wallets.contains(&"mywallet".to_string()) {
                    println!("Wallet 'mywallet' is in the list of wallets");
                    return Ok(());
                }
                return Err(e);
            }
        }
    }
}

fn ensure_blocks_mined(rpc: &RpcClient) -> Result<(), bitcoincore_rpc::Error> {
    let block_count = rpc.get_block_count()?;

    if block_count < 101 {
        println!("Mining initial blocks to ensure we have mature coins...");
        // Get a new address for mining rewards
        let address_uncheck = rpc.get_new_address(None, None)?;
        let address = address_uncheck.require_network(Network::Regtest).unwrap();
        // Mine enough blocks to reach at least 101
        let blocks_needed = 101 - block_count;
        rpc.generate_to_address(blocks_needed, &address)?;
        println!("Mined {} blocks", blocks_needed);
    } else {
        println!("Already have {} blocks, no need to mine more", block_count);
    }

    Ok(())
}

fn mine_block(rpc: &RpcClient, address: &Address) -> Result<String, bitcoincore_rpc::Error> {
    // Create a coinbase transaction paying to our address
    let block_hashes = rpc.generate_to_address(1, address)?;
    let block_hash = &block_hashes[0];

    // Get the block to find the coinbase transaction
    let block = rpc.get_block_info(block_hash)?;
    let coinbase_txid = block.tx[0].to_string();

    Ok(coinbase_txid)
}

fn generate_block(rpc: &RpcClient) -> Result<String, bitcoincore_rpc::Error> {
    // Get an address to generate to
    let address_uncheck = rpc.get_new_address(None, None)?;
    let address = address_uncheck.require_network(Network::Regtest).unwrap();
    let block_hashes = rpc.generate_to_address(1, &address)?;
    Ok(block_hashes[0].to_string())
}

fn create_first_transaction(
    rpc: &RpcClient,
    coinbase_txid: &str,
    private_key: &PrivateKey,
    public_key: &PublicKey,
    secp: &Secp256k1<bitcoin::secp256k1::All>,
) -> Result<String, bitcoincore_rpc::Error> {
    // Get coinbase transaction details
    let txid = bitcoin::Txid::from_str(coinbase_txid).unwrap();
    let tx_info = rpc.get_raw_transaction_info(&txid, None)?;

    // Get value from the first output (assuming coinbase has only one output to our address)
    let value_sats = tx_info.vout[0].value.to_sat();
    let vout_idx = 0; // Usually coinbase has just one output

    // Create input from coinbase
    let outpoint = OutPoint::new(txid, vout_idx);
    let txin = TxIn {
        previous_output: outpoint,
        script_sig: ScriptBuf::new(),
        sequence: Sequence::MAX,
        witness: Witness::new(),
    };

    // Create output (sending to the same address, but with a slightly smaller amount for fees)
    let fee = 1000; // 1000 satoshis fee
    let txout = TxOut {
        value: Amount::from_sat(value_sats - fee),
        script_pubkey: address_to_script(public_key, Network::Regtest),
    };

    // Create unsigned transaction
    let mut tx = Transaction {
        version: bitcoin::transaction::Version(2),
        lock_time: bitcoin::locktime::absolute::LockTime::ZERO,
        input: vec![txin],
        output: vec![txout],
    };

    // Sign the transaction
    let script_code = address_to_script(public_key, Network::Regtest);
    let sighash = bitcoin::sighash::SighashCache::new(&tx)
        .legacy_signature_hash(
            0,
            &script_code,
            bitcoin::sighash::EcdsaSighashType::All.to_u32(),
        )
        .unwrap();

    let message = bitcoin::secp256k1::Message::from_digest_slice(&sighash[..]).unwrap();
    let signature = secp.sign_ecdsa(&message, &private_key.inner);

    let mut sig_ser = signature.serialize_der().to_vec();
    sig_ser.push(bitcoin::sighash::EcdsaSighashType::All.to_u32() as u8);

    // Create a proper PushBytesBuf
    let mut push_bytes_buf = bitcoin::script::PushBytesBuf::with_capacity(sig_ser.len());
    push_bytes_buf.extend_from_slice(&sig_ser).unwrap();

    let script_sig = ScriptBuf::builder()
        .push_slice(push_bytes_buf)
        .push_key(public_key)
        .into_script();

    tx.input[0].script_sig = script_sig;

    // Serialize and submit transaction
    let tx_hex = hex::encode(serialize(&tx));

    // Send raw transaction
    let txid = rpc.send_raw_transaction(tx_hex)?;

    Ok(txid.to_string())
}

fn create_second_transaction(
    rpc: &RpcClient,
    first_txid: &str,
    private_key: &PrivateKey,
    public_key: &PublicKey,
    secp: &Secp256k1<bitcoin::secp256k1::All>,
) -> Result<String, bitcoincore_rpc::Error> {
    // Get the first transaction details
    let txid = bitcoin::Txid::from_str(first_txid).unwrap();
    let tx_info = rpc.get_raw_transaction_info(&txid, None)?;

    // Get value from the output
    let value_sats = tx_info.vout[0].value.to_sat();
    let vout_idx = 0;

    // Create input from first transaction
    let outpoint = OutPoint::new(txid, vout_idx);
    let txin = TxIn {
        previous_output: outpoint,
        script_sig: ScriptBuf::new(),
        sequence: Sequence::MAX,
        witness: Witness::new(),
    };

    // Create output (sending to the same address, minus fees)
    let fee = 1000; // 1000 satoshis fee
    let txout = TxOut {
        value: Amount::from_sat(value_sats - fee),
        script_pubkey: address_to_script(public_key, Network::Regtest),
    };

    // Create unsigned transaction
    let mut tx = Transaction {
        version: bitcoin::transaction::Version(2),
        lock_time: bitcoin::locktime::absolute::LockTime::ZERO,
        input: vec![txin],
        output: vec![txout],
    };

    // Sign the transaction
    let script_code = address_to_script(public_key, Network::Regtest);
    let sighash = bitcoin::sighash::SighashCache::new(&tx)
        .legacy_signature_hash(
            0,
            &script_code,
            bitcoin::sighash::EcdsaSighashType::All.to_u32(),
        )
        .unwrap();

    let message = bitcoin::secp256k1::Message::from_digest_slice(&sighash[..]).unwrap();
    let signature = secp.sign_ecdsa(&message, &private_key.inner);

    let mut sig_ser = signature.serialize_der().to_vec();
    sig_ser.push(bitcoin::sighash::EcdsaSighashType::All.to_u32() as u8);

    // Create a proper PushBytesBuf
    let mut push_bytes_buf = bitcoin::script::PushBytesBuf::with_capacity(sig_ser.len());
    push_bytes_buf.extend_from_slice(&sig_ser).unwrap();

    let script_sig = ScriptBuf::builder()
        .push_slice(push_bytes_buf)
        .push_key(public_key)
        .into_script();

    tx.input[0].script_sig = script_sig;

    // Serialize and submit transaction
    let tx_hex = hex::encode(serialize(&tx));

    // Send raw transaction
    let txid = rpc.send_raw_transaction(tx_hex)?;

    Ok(txid.to_string())
}

fn address_to_script(public_key: &PublicKey, network: Network) -> ScriptBuf {
    Address::p2pkh(public_key, network).script_pubkey()
}