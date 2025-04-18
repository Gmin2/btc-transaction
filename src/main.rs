use bitcoin::consensus::encode::serialize;
use bitcoin::key::{PrivateKey, PublicKey, Secp256k1};
use bitcoin::{
    Address, Amount, Network, OutPoint, ScriptBuf, Sequence, Transaction, TxIn, TxOut, Witness,
};
use bitcoincore_rpc::{Auth, Client as RpcClient, RpcApi};
use env_logger::Builder;
use log::{debug, error, info, warn, LevelFilter};
use secp256k1::{rand, SecretKey};
use std::io::Write;
use std::str::FromStr;
use std::{thread, time};

fn main() {
    init_logger();
    
    info!("Bitcoin Transaction Chain Demonstration");
    info!("---------------------------------------");
    
    // Connect to Bitcoin node
    let rpc_url = "http://localhost:18443";
    let client = match RpcClient::new(
        rpc_url,
        Auth::UserPass("alice".to_string(), "password".to_string()),
    ) {
        Ok(client) => {
            info!("Connected to Bitcoin node at {}", rpc_url);
            client
        },
        Err(e) => {
            error!("Failed to create RPC client: {}", e);
            return;
        }
    };

    if let Err(e) = initialize_wallet(&client) {
        error!("Failed to initialize wallet: {}", e);
        return;
    }

    // Create key pair for our transactions
    info!("Generating key pair for transactions...");
    let secp = Secp256k1::new();
    let secret_key = SecretKey::new(&mut rand::thread_rng());
    let private_key = PrivateKey::new(secret_key, Network::Regtest);
    let public_key = PublicKey::from_private_key(&secp, &private_key);

    let address = Address::p2pkh(&public_key, Network::Regtest);
    info!("Generated address: {}", address);

    // Ensure we have enough blocks to generate mature coins
    match ensure_blocks_mined(&client) {
        Ok(_) => debug!("Blocks mined successfully"),
        Err(e) => {
            error!("Failed to mine initial blocks: {}", e);
            return;
        }
    }

    // Generate coinbase transaction by mining a block
    info!("Mining a block to generate a coinbase transaction...");
    let coinbase_txid = match mine_block(&client, &address) {
        Ok(txid) => {
            info!("Mined block with coinbase txid: {}", txid);
            txid
        },
        Err(e) => {
            error!("Failed to mine block: {}", e);
            return;
        }
    };

    // Mining additional blocks to mature the coinbase
    info!("Mining 100 additional blocks to mature the coinbase...");
    match client.get_new_address(None, None) {
        Ok(temp_address) => {
            let address = temp_address.require_network(Network::Regtest).unwrap();
            match client.generate_to_address(100, &address) {
                Ok(_) => info!("Successfully mined 100 blocks"),
                Err(e) => {
                    error!("Failed to mine additional blocks: {}", e);
                    return;
                }
            }
        },
        Err(e) => {
            error!("Failed to get a temporary address: {}", e);
            return;
        }
    }

    // Wait for the block to be properly processed
    debug!("Waiting for blocks to be processed...");
    thread::sleep(time::Duration::from_secs(1));

    // Create and submit first transaction (spending coinbase)
    info!("Creating first transaction to spend the coinbase output...");
    let first_tx = match create_first_transaction(&client, &coinbase_txid, &private_key, &public_key, &secp) {
        Ok(txid) => {
            info!("First transaction created and submitted: {}", txid);
            txid
        },
        Err(e) => {
            error!("Failed to create first transaction: {}", e);
            return;
        }
    };

    // Generate a block to confirm first transaction
    info!("Mining a block to confirm the first transaction...");
    match generate_block(&client) {
        Ok(block_hash) => debug!("Generated block {} to confirm first transaction", block_hash),
        Err(e) => {
            error!("Failed to generate block: {}", e);
            return;
        }
    }

    // Wait for block to be processed
    debug!("Waiting for block to be processed...");
    thread::sleep(time::Duration::from_secs(1));

    // Create and submit second transaction (spending first tx)
    info!("Creating second transaction to spend the output of the first transaction...");
    let second_tx = match create_second_transaction(&client, &first_tx, &private_key, &public_key, &secp) {
        Ok(txid) => {
            info!("Second transaction created and submitted: {}", txid);
            txid
        },
        Err(e) => {
            error!("Failed to create second transaction: {}", e);
            return;
        }
    };

    // Generate a final block to confirm second transaction
    info!("Mining a block to confirm the second transaction...");
    match generate_block(&client) {
        Ok(block_hash) => debug!("Generated block {} to confirm second transaction", block_hash),
        Err(e) => {
            error!("Failed to generate final block: {}", e);
            return;
        }
    }

    info!("Transaction chain complete!");
    info!("Summary:");
    info!("1. Coinbase transaction: {}", coinbase_txid);
    info!("2. First transaction: {}", first_tx);
    info!("3. Second transaction: {}", second_tx);
}

fn init_logger() {
    use colored::*;
    let mut builder = Builder::new();
    
    builder
        .format(|buf, record| {
            let timestamp = chrono::Local::now().format("%Y-%m-%d %H:%M:%S%.3f");
            let level = match record.level() {
                log::Level::Error => record.level().to_string().red().bold(),
                log::Level::Warn => record.level().to_string().yellow().bold(),
                log::Level::Info => record.level().to_string().green().bold(),
                log::Level::Debug => record.level().to_string().blue().bold(),
                log::Level::Trace => record.level().to_string().magenta().bold(),
            };
            
            let message = match record.level() {
                log::Level::Error => record.args().to_string().red(),
                log::Level::Warn => record.args().to_string().yellow(),
                log::Level::Info => record.args().to_string().white(),
                log::Level::Debug => record.args().to_string().bright_black(),
                log::Level::Trace => record.args().to_string().magenta(),
            };
            
            writeln!(
                buf,
                "[{}] [{}] {}",
                timestamp.to_string().bright_black(),
                level,
                message
            )
        })
        .filter(None, LevelFilter::Info)
        .init();
}

fn initialize_wallet(client: &RpcClient) -> Result<(), bitcoincore_rpc::Error> {
    info!("Initializing wallet...");
    
    // Try to create the wallet first
    match client.create_wallet("mywallet", None, None, None, None) {
        Ok(_) => {
            info!("Created new wallet 'mywallet'");
            return Ok(());
        },
        Err(e) => {
            // If wallet already exists, try to load it
            if e.to_string().contains("already exists") {
                debug!("Wallet 'mywallet' already exists, trying to load it");
                match client.load_wallet("mywallet") {
                    Ok(_) => {
                        info!("Loaded existing wallet 'mywallet'");
                        return Ok(());
                    },
                    Err(load_err) => {
                        // Check if wallet is already loaded
                        if load_err.to_string().contains("already loaded") {
                            info!("Wallet 'mywallet' is already loaded");
                            return Ok(());
                        } else {
                            warn!("Error loading wallet: {}", load_err);
                            // Try to continue by checking wallet list
                            let wallets = client.list_wallets()?;
                            if wallets.contains(&"mywallet".to_string()) {
                                info!("Wallet 'mywallet' is in the list of wallets");
                                return Ok(());
                            }
                            return Err(load_err);
                        }
                    }
                }
            } else {
                warn!("Error creating wallet: {}", e);
                // Try to continue anyway, the wallet might be loadable
                let wallets = client.list_wallets()?;
                if wallets.contains(&"mywallet".to_string()) {
                    info!("Wallet 'mywallet' is in the list of wallets");
                    return Ok(());
                }
                return Err(e);
            }
        }
    }
}

fn ensure_blocks_mined(rpc: &RpcClient) -> Result<(), bitcoincore_rpc::Error> {
    let block_count = rpc.get_block_count()?;
    debug!("Current block count: {}", block_count);

    if block_count < 101 {
        info!("Mining initial blocks to ensure we have mature coins...");
        // Get a new address for mining rewards
        let address_uncheck = rpc.get_new_address(None, None)?;
        let address = address_uncheck.require_network(Network::Regtest).unwrap();
        // Mine enough blocks to reach at least 101
        let blocks_needed = 101 - block_count;
        rpc.generate_to_address(blocks_needed, &address)?;
        info!("Mined {} blocks", blocks_needed);
    } else {
        debug!("Already have {} blocks, no need to mine more", block_count);
    }

    Ok(())
}

fn mine_block(rpc: &RpcClient, address: &Address) -> Result<String, bitcoincore_rpc::Error> {
    // Create a coinbase transaction paying to our address
    let block_hashes = rpc.generate_to_address(1, address)?;
    let block_hash = &block_hashes[0];
    debug!("Mined block with hash: {}", block_hash);

    // Get the block to find the coinbase transaction
    let block = rpc.get_block_info(block_hash)?;
    let coinbase_txid = block.tx[0].to_string();
    debug!("Coinbase transaction ID: {}", coinbase_txid);

    Ok(coinbase_txid)
}

fn generate_block(rpc: &RpcClient) -> Result<String, bitcoincore_rpc::Error> {
    // Get an address to generate to
    let address_uncheck = rpc.get_new_address(None, None)?;
    let address = address_uncheck.require_network(Network::Regtest).unwrap();
    let block_hashes = rpc.generate_to_address(1, &address)?;
    debug!("Generated block: {}", block_hashes[0]);
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
    debug!("Coinbase output value: {} satoshis", value_sats);

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
    debug!("Creating transaction with fee: {} satoshis", fee);

    // Create unsigned transaction
    let mut tx = Transaction {
        version: bitcoin::transaction::Version(2),
        lock_time: bitcoin::locktime::absolute::LockTime::ZERO,
        input: vec![txin],
        output: vec![txout],
    };

    // Sign the transaction
    debug!("Signing transaction...");
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
    debug!("Transaction serialized, submitting to network...");

    // Send raw transaction
    let txid = rpc.send_raw_transaction(tx_hex)?;
    debug!("Transaction submitted successfully");

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
    debug!("First transaction output value: {} satoshis", value_sats);

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
    debug!("Creating transaction with fee: {} satoshis", fee);

    // Create unsigned transaction
    let mut tx = Transaction {
        version: bitcoin::transaction::Version(2),
        lock_time: bitcoin::locktime::absolute::LockTime::ZERO,
        input: vec![txin],
        output: vec![txout],
    };

    // Sign the transaction
    debug!("Signing transaction...");
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
    debug!("Transaction serialized, submitting to network...");

    // Send raw transaction
    let txid = rpc.send_raw_transaction(tx_hex)?;
    debug!("Transaction submitted successfully");

    Ok(txid.to_string())
}

fn address_to_script(public_key: &PublicKey, network: Network) -> ScriptBuf {
    Address::p2pkh(public_key, network).script_pubkey()
}