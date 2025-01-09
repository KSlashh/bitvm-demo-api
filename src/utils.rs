
extern crate bitcoin_origin;
extern crate bitcoin_hashes_origin;
extern crate bitcoin_hashes;

use actix_web::Error;
use shh::stdout as shh_stdout;
use tokio::time::{sleep, Duration};
use core::str::FromStr;
use bitcoin_hashes::Hash;
use bitcoin_hashes_origin::hex::{FromHex, ToHex};
use bitcoincore_rpc::{Client, Auth, RpcApi};
use bitcoin::consensus::{encode, deserialize};
use bitcoin::{
    Address, Amount, OutPoint, Transaction, TxIn, 
    TxOut, Txid, ScriptBuf, Witness, Sequence,
    EcdsaSighashType, secp256k1::{Message, SecretKey, rand}, sighash::SighashCache,
};
use bitvm::treepp::*;
use bitvm::bridge::scripts;
use bitvm::bridge::contexts::base;
use crate::config;

pub fn address_from_str(addr_str: &str) -> Result<Address, String> {
    match Address::from_str(addr_str) {
        Ok(addr) => Ok(addr.assume_checked()),
        Err(e) => Err(format!("fail parse address: {}",e)),
    }
}

pub fn txid_from_str(txid_str: &str) -> Result<Txid, String> {
    match hex::decode(txid_str) {
        Ok(txid_slice) => {
            let mut txid_slice = txid_slice.to_vec();
            txid_slice.reverse();
            match Txid::from_slice(txid_slice.as_slice()) {
                Ok(v) => Ok(v),
                Err(e) => Err(format!("fail to decode txid: {}",e)),
            }
        },
        Err(e) => Err(format!("fail to decode txid: {}",e)),
    }
}

pub async fn check_rpc() -> bool {
    println!("\nChecking if bitcoind rpc is working......");
    let rpc = match new_rpc_client().await {
        Ok(v) => v,
        Err(e) => { 
            println!("fail to connect: {}",e);
            return false
        },
    };
    match mint_block(&rpc, 1) {
        Ok(_) => true,
        Err(e) => { 
            println!("fail to mint block: {}",e);
            false
        },
    }
}

pub async fn new_rpc_client() -> Result<Client, String> {
    match Client::new(config::RPC_URL, Auth::UserPass(config::RPCUSER.to_string(), config::RPCPASSWORD.to_string())) {
        Ok(v) => Ok(v),
        Err(e) => Err(format!("fail to connect to node: {}",e)),
    }
}

pub fn dead_address() -> Address {
    Address::p2sh(&script!{OP_RETURN}.compile(), config::network()).unwrap()
} 

pub async fn wait_tx() {
    sleep(Duration::from_secs(config::TX_WAIT_TIME)).await;
} 

pub fn tx_wrapper(tx: &Transaction) -> String {
    encode::serialize_hex(tx)
}

pub fn tx_unwrapper(tx: &bitcoin_origin::Transaction) -> Transaction {
    let tx_bytes = bitcoin_origin::consensus::serialize(tx);
    deserialize(&tx_bytes).unwrap()
}

pub fn address_wrapper(address: &Address) -> bitcoin_origin::Address {
    bitcoin_origin::Address::from_str(&address.to_string()).unwrap()
}

pub fn amount_wrapper(amount: Amount) -> bitcoin_origin::Amount {
    bitcoin_origin::Amount::from_sat(amount.to_sat())
}

pub fn amount_unwrapper(amount: bitcoin_origin::Amount) -> Amount {
    Amount::from_sat(amount.to_sat())
}

pub fn txid_wrapper(txid: Txid) -> bitcoin_origin::Txid {
    let hash_hex = txid.to_hex();
    bitcoin_origin::Txid::from_hash(bitcoin_hashes_origin::sha256d::Hash::from_hex(&hash_hex).unwrap())
}

pub fn txid_unwrapper(txid: bitcoin_origin::Txid) -> Txid {
    let mut hash_bytes = hex::decode(txid.to_hex()).unwrap();
    hash_bytes.reverse();
    Txid::from_slice(hash_bytes.as_slice()).unwrap()
}

pub fn broadcast_tx(rpc: &Client, tx: &Transaction) -> Result<bool, String> {
    match rpc.send_raw_transaction(tx_wrapper(tx)) {
        Ok(_) => Ok(true),
        Err(e) => Err(format!("fail to broadcast_tx: {}",e)),
    }
}

pub fn mint_block(rpc: &Client, block_num: u64) -> Result<bool, String> {
    let wallet_address = match rpc.get_new_address(None, None) {
        Ok(v) => v,
        Err(e) => return Err(format!("fail to generat wallet address: {}",e))
    };
    match rpc.generate_to_address(block_num, &wallet_address) {
        Ok(_) => Ok(true),
        Err(e) => Err(format!("fail to mint block: {}",e))
    }
}

pub fn validate_tx(rpc: &Client, txid: Txid) -> Result<bool, String> {
    match rpc.get_tx_out(&txid_wrapper(txid), 0, None) {
        Ok(res) => {
            match res {
                Some(tx_out_res) => Ok(tx_out_res.confirmations > 0),
                _ => Ok(false)
            }
        },
        Err(e) => Err(format!("fail to get tx_info: {}",e))
    }
}

pub fn get_utxo_value(rpc: &Client, txid: Txid, vout: u32) -> Result<Amount, String> {
    match rpc.get_tx_out(&txid_wrapper(txid), vout, Some(true)) {
        Ok(res_option) => match res_option {
            Some(res) => Ok(amount_unwrapper(res.value)),
            _ => Err("fail when get_utxo_value: no such outpoint".to_string()),
        },
        Err(e) => Err(format!("fail to get tx out: {}",e))
    }
}

pub fn get_raw_tx(rpc: &Client, txid: Txid) -> Result<Transaction, String> {
    let tx = match rpc.get_raw_transaction(&txid_wrapper(txid), None) {
        Ok(v) => v,
        Err(e) => return Err(format!("fail to get_raw_tx: {}",e))
    };
    Ok(tx_unwrapper(&tx))
}

pub fn decode_txid(txid_hex: &str) -> Result<Txid, String> {
    let mut txid_bytes = match hex::decode(txid_hex) {
        Ok(v) => v,
        Err(e) => return Err(format!("fail to decode txid: {:?}: {:?}", txid_hex, e)),
    };
    txid_bytes.reverse();
    match Txid::from_slice(txid_bytes.as_slice()) {
        Ok(v) => Ok(v),
        Err(e) => Err(format!("invalid txid: {:?}: {:?}", txid_hex, e)),
    }
}

pub fn generate_stub_outpoint_batch(
    rpc: &Client,
    funding_utxo_addresses: &Vec<Address>,
    input_values: &Vec<Amount>,
) -> Result<Vec<OutPoint>, String> { 
    // init fund
    assert!(funding_utxo_addresses.len() == input_values.len());
    let res_num = funding_utxo_addresses.len() as u32;
    let total_amount = input_values.iter().map(|amount| amount.to_sat()).sum::<u64>() + 1_000_000;
    let network = config::network();
    let tmp_secret = SecretKey::new(&mut rand::thread_rng()).secret_bytes().to_hex();
    let (secp, keypair, public_key) = base::generate_keys_from_secret(network, &tmp_secret);
    let bulk_address = scripts::generate_pay_to_pubkey_script_address(network, &public_key);
    let fund_outpoint = match fund_utxo(rpc, &bulk_address, Amount::from_sat(total_amount)) {
        Ok(v) => v,
        Err(e) => return Err(e),
    };  
    
    // construct bulk tx
    let bulk_input = vec![
        TxIn {
            previous_output: fund_outpoint,
            script_sig: ScriptBuf::new(),
            sequence: Sequence::MAX,
            witness: Witness::default(),
        },
    ];
    let bulk_output = funding_utxo_addresses.iter()
        .zip(input_values.iter())
        .map(|(addr, amt)| TxOut { value: *amt, script_pubkey: addr.script_pubkey()})
        .collect();
    let mut bulk_tx = Transaction{
        version: bitcoin::transaction::Version(2),
        lock_time: bitcoin::absolute::LockTime::ZERO,
        input: bulk_input,
        output: bulk_output,
    };

    // sign 
    let scr = scripts::generate_pay_to_pubkey_script(&public_key);
    let value = Amount::from_sat(total_amount);
    let sighash_type = EcdsaSighashType::All;
    let mut sighash_cache = SighashCache::new(&mut bulk_tx);
    let sighash = sighash_cache
        .p2wsh_signature_hash(0, &scr, value, sighash_type)
        .expect("Failed to construct sighash");
    let signature = secp.sign_ecdsa(&Message::from(sighash), &keypair.secret_key());
    let signature = bitcoin::ecdsa::Signature {
        signature,
        sighash_type,
    };
    bulk_tx.input[0].witness.push_ecdsa_signature(&signature);
    bulk_tx.input[0].witness.push(scr);

    // broadcast
    if let Err(e) = mint_block(rpc, 1) {
        return Err(format!("fail to mint block: {}", e))
    };
    if let Err(e) = broadcast_tx(rpc, &bulk_tx) {
        return Err(format!("fail to broadcast faucet tx: {}", e))
    };
    let txid = bulk_tx.compute_txid();
    Ok((0..res_num).map(|vout| OutPoint{txid,vout}).collect())
}

pub fn generate_stub_outpoint(
    rpc: &Client,
    funding_utxo_address: &Address,
    input_value: Amount,
) -> Result<OutPoint, String> {
    fund_utxo(rpc, funding_utxo_address, input_value)
}

pub fn fund_utxo(rpc: &Client, address: &Address, amount: Amount) -> Result<OutPoint, String> {
    let txid = match rpc.send_to_address(&address_wrapper(address), amount_wrapper(amount), None, None, None, None, None, None) {
        Ok(v) => v,
        Err(e) => return Err(format!("fail to send from faucet: {}", e)),
    };
    let txinfo = match rpc.get_transaction(&txid, None) {
        Ok(v) => v,
        Err(e) => return Err(format!("fail to get fundind tx info: {}", e)),
    };
    Ok(OutPoint {
        txid: txid_unwrapper(txid),
        vout: txinfo.details[0].vout,
    })
}

pub fn suppress_output<F>(f: F)
where
    F: FnOnce(),
{
    let shh = shh_stdout().unwrap();
    f();  
    drop(shh);
}

