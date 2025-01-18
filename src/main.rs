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
use bitvm::treepp::Script;
use clap::{Command, Arg};
use actix_web::{web, App, HttpResponse, HttpServer};
use actix_cors::Cors;
use log::{info, warn, error, debug, trace};

// export RUST_MIN_STACK=8388608

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    log4rs::init_file("log4rs.yml", Default::default()).unwrap();

    if !setup::check_setup() {
        info!("Initializing ......");
        setup::setup_all();
    };

    if !utils::check_rpc().await {
        error!("ERROR: bitcoin node is down/incomplete/misconfigured!");
        std::process::exit(2);
    }

    info!("load statics");
    let _ = &transactions::CONNECTOR_C_TAPSCRIPTS;
    let _ = &transactions::BITCOM_LOCK_SCRIPTS;
    let _ = &transactions::BITCOM_UNLOCK_SCRIPTS;
    let _ = &transactions::REVEALERS_ADDRESS;
    let _ = &transactions::CONNECTOR_C_SPEND_INFO;

    let ip = config::BIND_IP;
    let port = config::BIND_PORT;
    info!("Listening to {ip}:{port} ......");
    HttpServer::new(|| App::new()
        .service(api::get_named_inputs_outputs)
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
        .service(api::send_disprove)
        .wrap(
            Cors::default()
                .allow_any_origin() 
                .allow_any_method() 
                .allow_any_header() 
        )
    )
    .bind((ip, port))?
    .run()
    .await
}

#[tokio::test]
async fn send_pegin_kickoff1() {
    let rpc = utils::new_rpc_client().await.unwrap();
    let pegin_txid = transactions::peg_in(&rpc);
    let kickoff1_txid = transactions::kick_off_1(&rpc);
    println!("pegin_txid: {pegin_txid} \nkickoff1_txid: {kickoff1_txid}");
}

#[test]
fn test_nest_segwit() {
    let addr = utils::address_from_str("2NG9hHGn4HRTG4ncjZkAdQGe6wbbhz4pmEs").unwrap();
    let script_pubkey = addr.script_pubkey();
    let regtest_addr = Address::from_script(&script_pubkey, bitcoin::Network::Regtest).unwrap();
    let testnet_addr = Address::from_script(&script_pubkey, bitcoin::Network::Testnet).unwrap();
    let mainnet_addr = Address::from_script(&script_pubkey, bitcoin::Network::Bitcoin).unwrap();
    println!("regtest_addr: {regtest_addr}\ntestnet_addr: {testnet_addr}\nmainnet_addr: {mainnet_addr}\nscript_pubkey: {script_pubkey}");

    let addr = utils::address_from_str("n4RRb3SZw64nYwmNV5rYBcWESihXYP4W7X").unwrap();
    let script_pubkey = addr.script_pubkey();
    let regtest_addr = Address::from_script(&script_pubkey, bitcoin::Network::Regtest).unwrap();
    let testnet_addr = Address::from_script(&script_pubkey, bitcoin::Network::Testnet).unwrap();
    let mainnet_addr = Address::from_script(&script_pubkey, bitcoin::Network::Bitcoin).unwrap();
    println!("\nregtest_addr: {regtest_addr}\ntestnet_addr: {testnet_addr}\nmainnet_addr: {mainnet_addr}\nscript_pubkey: {script_pubkey}");

    let addr = utils::address_from_str("mtbsBabgFMZxz792s8LhkhbVEx5AoBKn2d").unwrap();
    let script_pubkey = addr.script_pubkey();
    let regtest_addr = Address::from_script(&script_pubkey, bitcoin::Network::Regtest).unwrap();
    let testnet_addr = Address::from_script(&script_pubkey, bitcoin::Network::Testnet).unwrap();
    let mainnet_addr = Address::from_script(&script_pubkey, bitcoin::Network::Bitcoin).unwrap();
    println!("\nregtest_addr: {regtest_addr}\ntestnet_addr: {testnet_addr}\nmainnet_addr: {mainnet_addr}\nscript_pubkey: {script_pubkey}");

    let addr = utils::address_from_str("tb1q9paymy2jzfdhlh95sgu65nu7mcmpd7qqsytc28").unwrap();
    let script_pubkey = addr.script_pubkey();
    let regtest_addr = Address::from_script(&script_pubkey, bitcoin::Network::Regtest).unwrap();
    let testnet_addr = Address::from_script(&script_pubkey, bitcoin::Network::Testnet).unwrap();
    let mainnet_addr = Address::from_script(&script_pubkey, bitcoin::Network::Bitcoin).unwrap();
    println!("\nregtest_addr: {regtest_addr}\ntestnet_addr: {testnet_addr}\nmainnet_addr: {mainnet_addr}\nscript_pubkey: {script_pubkey}");
}

#[allow(dead_code)]
async fn disprove_opt_test(corrupt_index: u32) {
    use std::time::SystemTime;

    let now = SystemTime::now();
    let connector_c_addr = transactions::get_precomputed_connector_c_address();
    let bitcom_lock_scripts = transactions::borrow_bitcom_lock_scripts();
    let connector_c_tapscripts = transactions::borrow_assert_tapscripts();
    let _ = &transactions::REVEALERS_ADDRESS;
    let _ = &transactions::CONNECTOR_C_SPEND_INFO;
    let _ = &transactions::REVEALERS_ADDRESS;
    let duration = now.elapsed().unwrap().as_millis().to_string();
    println!("\nonce-cell load cost: [{duration}]ms");

    let now = SystemTime::now();
    let rpc = utils::new_rpc_client().await.unwrap();
    println!("\npeg-in......");
    let peg_in_txid = transactions::peg_in(&rpc);
    println!("peg-in txid: {peg_in_txid}");
    println!("\nkick_off......");
    let kick_off_1_txid = transactions::kick_off_1(&rpc);
    println!("kick_off_1_txid: {kick_off_1_txid}");
    let kick_off_2_txid = transactions::kick_off_2(&rpc, kick_off_1_txid, bitcom_lock_scripts).await.unwrap();
    println!("kick_off_2_txid: {kick_off_2_txid}");
    let duration = now.elapsed().unwrap().as_millis().to_string();
    println!("\npegin+kickoff cost: [{duration}]ms");

    let now = SystemTime::now();
    println!("\nfake {corrupt_index}th assertions");
    println!("assert......");
    let (assert_txid, _) = transactions::assert(&rpc, kick_off_2_txid, &bitcom_lock_scripts, &connector_c_tapscripts, Some(corrupt_index), Some(connector_c_addr.clone())).await.unwrap();
    println!("assert_txid: {assert_txid}");
    let duration = now.elapsed().unwrap().as_millis().to_string();
    println!("\nassert cost: [{duration}]ms");

    let now = SystemTime::now();
    println!("\ndisprove......");
    let disprove_txid = transactions::disprove(&rpc, assert_txid, &connector_c_tapscripts, Some(connector_c_addr), Some(corrupt_index as usize)).await.unwrap();
    println!("disprove_txid: {disprove_txid}");
    let duration = now.elapsed().unwrap().as_millis().to_string();
    println!("\ndisprove cost: [{duration}]ms");
}

/* 
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
*/
