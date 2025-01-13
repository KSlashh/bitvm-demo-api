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
use actix_cors::Cors;


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
