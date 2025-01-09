#![allow(unused_imports)]
pub mod config;
pub mod utils;
pub mod setup;
pub mod transactions;
pub mod api;
pub mod sql;

use std::io::Write;
use std::fs::File;
use std::str::FromStr;
use std::thread::Builder;
use bitcoin::{Address, Amount, Txid};
use bitcoin_hashes::Hash;
use bitvm::bridge::transactions::kick_off_1;
use clap::{Command, Arg};
use actix_web::{web, App, HttpResponse, HttpServer};


// export RUST_MIN_STACK=8388608

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    if !setup::check_setup() {
        println!("\nInitializing ......");
        setup::setup_all();
    };

    if !utils::check_rpc().await {
        println!("ERROR: bitcoin node is down/incomplete/misconfigured!");
        std::process::exit(2);
    }

    let ip = config::BIND_IP;
    let port = config::BIND_PORT;
    println!("\nListening to {ip}:{port} ......");
    HttpServer::new(|| App::new()
        .service(api::get_user_workflow)
        .service(api::get_workflow_info)
        .service(api::request_btc)
        .service(api::get_unsigned_pegin_tx)
        .service(api::post_pegin_txid)
        .service(api::post_fake_index)
        .service(api::get_unsigned_kickoff1_tx)
        .service(api::send_kickoff_2)
        .service(api::send_challenge)
        .service(api::send_take_1)
        .service(api::send_assert)
        .service(api::send_take_2)
        .service(api::send_disprove))
    .bind((ip, port))?
    .run()
    .await


}

#[tokio::test]
async fn test_pegin_kickoff1() {
    let rpc = utils::new_rpc_client().await.unwrap();
    let peg_in_txid = transactions::peg_in(&rpc);
    dbg!(peg_in_txid);
    let kickoff_1_txid = transactions::kick_off_1(&rpc);
    dbg!(kickoff_1_txid);
}

#[tokio::test]
async fn test_pegin() {
    use serde::Serialize;
    #[derive(Serialize)]
    struct AddressOutput {
        testnet_address: Address,
        regtest_address: Address,
        value: Amount, 
    }
    #[derive(Serialize)]
    struct Utxo {
        txid: Txid,
        vout: u32,
        value: Amount,
    }

    let addr = "tb1qkms7g4x8vpnp39m7e8nstrdfgg75n8cqxt6679";
    let addr = Address::from_str(addr).unwrap().assume_checked();
    let _mainnet_addr = Address::from_script(&addr.script_pubkey(), bitcoin::Network::Bitcoin);
    let _regtest_addr = Address::from_script(&addr.script_pubkey(), bitcoin::Network::Regtest);
    let _testnet_addr = Address::from_script(&addr.script_pubkey(), bitcoin::Network::Testnet);
    let _signet_addr = Address::from_script(&addr.script_pubkey(), bitcoin::Network::Signet);

    let rpc = utils::new_rpc_client().await.unwrap();
    let (faucet_1, _) = transactions::faucet(&rpc, &addr).unwrap();
    let pegin = transactions::peg_in_prepare(&rpc, faucet_1.txid, faucet_1.vout).unwrap();
    let pegin_raw_tx_json = serde_json::to_string_pretty(&pegin).unwrap();
    println!("address: {addr}");
    println!("\npegin_raw_tx_json: \n{pegin_raw_tx_json}");
    let faucet_1_amount = utils::get_utxo_value(&rpc, faucet_1.txid, faucet_1.vout).unwrap();
    let pegin_input_json = serde_json::to_string_pretty(&Utxo{
        txid: faucet_1.txid,
        vout: faucet_1.vout,
        value: faucet_1_amount,
    }).unwrap();
    println!("\n\npegin_input_json: \n{pegin_input_json}");
    let pegin_output_json = serde_json::to_string_pretty(&AddressOutput {
        testnet_address: Address::from_script(&pegin.output[0].script_pubkey, bitcoin::Network::Testnet).unwrap(),
        regtest_address: Address::from_script(&pegin.output[0].script_pubkey, bitcoin::Network::Regtest).unwrap(),
        value: pegin.output[0].value,
    }).unwrap();
    println!("\n\npegin_output_json: \n{pegin_output_json}");
}


#[tokio::test]
async fn test_kickoff() {
    use serde::Serialize;
    #[derive(Serialize)]
    struct AddressOutput {
        testnet_address: Address,
        regtest_address: Address,
        value: Amount, 
    }
    #[derive(Serialize)]
    struct Utxo {
        txid: Txid,
        vout: u32,
        value: Amount,
    }

    let addr = "tb1qqkwwqeraapk0jekl53jk5zznp6u0yemjalqk6e";
    let addr = Address::from_str(addr).unwrap().assume_checked();
    let _mainnet_addr = Address::from_script(&addr.script_pubkey(), bitcoin::Network::Bitcoin);
    let _regtest_addr = Address::from_script(&addr.script_pubkey(), bitcoin::Network::Regtest);
    let _testnet_addr = Address::from_script(&addr.script_pubkey(), bitcoin::Network::Testnet);
    let _signet_addr = Address::from_script(&addr.script_pubkey(), bitcoin::Network::Signet);

    let rpc = utils::new_rpc_client().await.unwrap();
    let (_, faucet_2) = transactions::faucet(&rpc, &addr).unwrap();
    let kickoff1 = transactions::kickoff_1_prepare(&rpc, faucet_2.txid, faucet_2.vout).unwrap();
    let kickoff1_raw_tx_json = serde_json::to_string_pretty(&kickoff1).unwrap();
    println!("address: {addr}");
    println!("\nkickoff1_raw_tx_json: \n{kickoff1_raw_tx_json}");

    let faucet_2_amount = utils::get_utxo_value(&rpc, faucet_2.txid, faucet_2.vout).unwrap();
    let kickoff1_input_json = serde_json::to_string_pretty(&Utxo{
        txid: faucet_2.txid,
        vout: faucet_2.vout,
        value: faucet_2_amount,
    }).unwrap();
    println!("\n\nkickoff1_input_json: \n{kickoff1_input_json}");
    let mut kickoff1_outputs = vec![];
    for i in 0..kickoff1.output.len() {
        let kickoff1_output_i = AddressOutput {
            testnet_address: Address::from_script(&kickoff1.output[i].script_pubkey, bitcoin::Network::Testnet).unwrap(),
            regtest_address: Address::from_script(&kickoff1.output[i].script_pubkey, bitcoin::Network::Regtest).unwrap(),
            value: kickoff1.output[i].value,
        };
        kickoff1_outputs.push(kickoff1_output_i);
    };
    let kickoff1_outputs_json = serde_json::to_string_pretty(&kickoff1_outputs).unwrap();
    println!("\n\nkickoff1_outputs_json: \n{kickoff1_outputs_json}");
}

/* 
#[tokio::main]
async fn main() {

    let connector_c_address = transactions::get_precomputed_connector_c_address();
    dbg!(&connector_c_address);

    if !check_setup() {
        println!("initialization is not complete, please init first");
        std::process::exit(2);
    };
    let rpc = utils::new_rpc_client().await;

    println!("\npeg-in......");
    let (peg_in_txid, peg_in_tx_weight) = transactions::peg_in(&rpc);
    println!("peg-in txid: {peg_in_txid}, weight:{} WU", peg_in_tx_weight.to_wu());

    let bitcom_lock_scripts = transactions::get_bitcom_lock_scripts();

    println!("\nkick_off......");
    let ((kick_off_1_txid, kickoff_1_tx_weight), (kick_off_2_txid, kickoff_2_tx_weight)) = transactions::kick_off(&rpc, &bitcom_lock_scripts);
    println!("kick_off_1 txid: {kick_off_1_txid}, weight:{} WU", kickoff_1_tx_weight.to_wu());
    println!("kick_off_2 txid: {kick_off_2_txid}, weight:{} WU", kickoff_2_tx_weight.to_wu());

    println!("\nchallenge......");
    let (challenge_txid, challenge_tx_weight) = transactions::challenge(&rpc, kick_off_1_txid);
    println!("challenge txid: {challenge_txid}, weight:{} WU", challenge_tx_weight.to_wu());


    println!("\nassert...... (this may take several minutes)");
    let connector_c_tapscripts = transactions::get_assert_tapscripts();
    let ((assert_txid, assert_tx_weight), connector_c_address) = transactions::assert(&rpc, kick_off_2_txid, &bitcom_lock_scripts, &connector_c_tapscripts, None, Some(connector_c_address));
    println!("assert txid: {assert_txid}, weight:{} WU", assert_tx_weight.to_wu());

    println!("\ntake_2......");
    let (take_2_txid, take_2_tx_weight) = transactions::take_2(&rpc, peg_in_txid, assert_txid, &connector_c_tapscripts, Some(connector_c_address));
    println!("take_2 txid: {take_2_txid}, weight:{} WU", take_2_tx_weight.to_wu());

    let total_weight = peg_in_tx_weight.to_wu() + kickoff_1_tx_weight.to_wu() + kickoff_2_tx_weight.to_wu() + challenge_tx_weight.to_wu() + assert_tx_weight.to_wu() + take_2_tx_weight.to_wu();
    let fee_rate = 20;
    let fee_sat = total_weight * fee_rate / 4;
    let fee= (fee_sat as f64) / 1_000_000_000.0;
    println!("\ntotal_cost: {total_weight} WU, estimate_fee: {fee} BTC / {fee_sat} sats (fee_rate: {fee_rate} sats/vB)")



    let test_addresses = [
            "1QJVDzdqb1VpbDK7uDeyVXy9mR27CJiyhY",
            "1J4LVanjHMu3JkXbVrahNuQCTGCRRgfWWx",
            "33iFwdLuRpW1uK1RTRqsoi8rR4NpDzk66k",
            "3QBRmWNqqBGme9er7fMkGqtZtp4gjMFxhE",
            "bc1zw508d6qejxtdg4y5r3zarvaryvaxxpcs",
            "bc1qvzvkjn4q3nszqxrv3nraga2r822xjty3ykvkuw",
            "bc1p5cyxnuxmeuwuvkwfem96lqzszd02n6xdcjrs20cac6yqjjwudpxqkedrcr",
            "bc1pgllnmtxs0g058qz7c6qgaqq4qknwrqj9z7rqn9e2dzhmcfmhlu4sfadf5e",
        ];
    let user0 = Address::from_str(test_addresses[0]).unwrap().assume_checked();
    let user1 = Address::from_str(test_addresses[1]).unwrap().assume_checked();
    let user2 = Address::from_str(test_addresses[2]).unwrap().assume_checked();
    let user3 = Address::from_str(test_addresses[3]).unwrap().assume_checked();

    let txids = [
        "4df8b62f2c39ca30c66d1cbc03353ec0698b0c1840b688baa821c31f460d6dd0",
        "98373644ce96f3181753854e5ccc61016524983e437aa491e1bfee7acfd66369",
        "4592bdbe89a994ae94d3c7bb8d2347149f1e18620573793c2699d684dd66b96c",
        "4802ed49128158780119644e24f947b505df7a0a70d90200d96d32bfd0e6a9df",
    ];
    let pegin_txid = Txid::from_slice(&hex::decode(txids[0]).unwrap()).unwrap();

    let db = sql::open_db().unwrap();

    let user0_id = sql::get_user_id(&db, &user0).unwrap().unwrap();
    let mut user0_data = sql::get_user_data(&db, user0_id).unwrap().unwrap();
    dbg!(&user0_data);

    user0_data.status += 1;
    user0_data.corrupt = Some((true, 10));
    user0_data.pegin = Some(pegin_txid);
    let user0_id = sql::get_user_id(&db, &user0).unwrap().unwrap();
    sql::update_user_data(&db, user0_id, &user0_data).unwrap();
    let mut user0_data = sql::get_user_data(&db, user0_id).unwrap().unwrap();
    dbg!(&user0_data);
    dbg!(user0_id);

}
*/

/*
pegin_input: 
{
    "txid": "“0370cb67eee1b2268d1e619de9f7bccad77044a547426e9ecbcc7a68fb5b9389",
    "vout": 0
}
“txid”: “0370cb67eee1b2268d1e619de9f7bccad77044a547426e9ecbcc7a68fb5b9389:0"
*/