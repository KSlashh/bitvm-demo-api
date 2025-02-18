use actix_web::{get, post, web,  http::header::ContentType, HttpResponse, HttpServer, Responder};
use bitcoin::{ Address, Amount, OutPoint, Transaction, Txid};
use bitvm::bridge::{connectors::{connector_c, revealer}, transactions::{kick_off_1, peg_in_refund}, graphs::base::DUST_AMOUNT};
use rusqlite::Connection;
use serde::Serialize;
use log::{info, warn, error};
use crate::{config, sql::{self, update_user_data, UserData}, transactions, utils};

#[derive(Serialize)]
struct TxInput {
    txid: Txid,
    vout: u32,
    script_pubkey: String,
    value: Amount,
}
#[derive(Serialize)]
struct TxOutput {
    testnet_address: Address,
    regtest_address: Address,
    value: Amount,
}

#[get("/get-named-inputs-outputs/{tx_type}/{txid}")]
async fn get_named_inputs_outputs(path: web::Path<(u8, String)>) -> impl Responder {
    #[derive(Serialize)]
    struct ResponseStruct {
        tx_name: String,
        inputs: Vec<(String, Address, Amount)>,
        outputs: Vec<(String, Address, Amount)>,
    }

    let (tx_type, txid) = path.into_inner();
    info!("new REQUEST: /get-named-inputs-outputs/{tx_type}/{txid}");
    let txid = match utils::txid_from_str(&txid) {
        Ok(v) => v,
        Err(e) => { 
            error!("/get-named-inputs-outputs/{tx_type}/{txid}: fail to decode txid: {}", e);
            return HttpResponse::BadRequest().body(e.to_string())
        }
    };

    let rpc = match utils::new_rpc_client().await {
        Ok(v) => v,
        Err(e) => { 
            error!("/get-named-inputs-outputs/{tx_type}/{txid}: fail to connect bitcoind: {}", e);
            return HttpResponse::InternalServerError().body(e.to_string())
        }
    };

    let (inputs, outputs) = match tx_type {
        2 => { // PegIn
            match get_inputs_outputs(&rpc, tx_type, txid) {
                Ok(v) => v,
                Err(e) => return HttpResponse::InternalServerError().body(e.to_string())
            }
        },
        3 => { // Kickoff_1
            match get_inputs_outputs(&rpc, tx_type, txid) {
                Ok(v) => v,
                Err(e) => return HttpResponse::InternalServerError().body(e.to_string())
            }
        },
        4 => { // Kickoff_2
            match get_inputs_outputs(&rpc, tx_type, txid) {
                Ok(v) => v,
                Err(e) => return HttpResponse::InternalServerError().body(e.to_string())
            }
        },
        5 => { // Challenge
            match get_inputs_outputs(&rpc, tx_type, txid) {
                Ok(v) => v,
                Err(e) => return HttpResponse::InternalServerError().body(e.to_string())
            }
        },
        6 => { // Take_1
            match get_inputs_outputs(&rpc, tx_type, txid) {
                Ok(v) => v,
                Err(e) => return HttpResponse::InternalServerError().body(e.to_string())
            }
        },
        7 => { // Assert
            let revealers_num = transactions::REVEALERS_ADDRESS.len();
            let r = transactions::REVEALERS_ADDRESS.clone()
                .into_iter()
                .zip(vec![Amount::from_sat(DUST_AMOUNT); revealers_num].into_iter())
                // .zip(transactions::get_revealers_script_pubkey().clone())
                // .map(|((x,y),z)| (x,y,z))
                .collect();
            let inputs = [
                vec![(transactions::get_precomputed_connector_b_address(), transactions::get_connector_b_amount())],
                r,
            ].concat();
            let outputs = vec![
                (transactions::get_precomputed_connector_4_address(), transactions::get_connector_4_amount()),
                (transactions::get_precomputed_connector_5_address(), transactions::get_connector_5_amount()), 
                (transactions::get_precomputed_connector_c_address(), transactions::get_connector_c_amount()),
            ];
            (inputs, outputs)
        },
        8 => { // Take_2
            let tx= match utils::get_raw_tx(&rpc, txid) {
                Ok(v) => v,
                Err(e) => { 
                    error!("/get-named-inputs-outputs/{tx_type}/{txid}: fail to get tx:{txid} : {}", e);
                    return HttpResponse::InternalServerError().body(e.to_string())
                }
            };
            let inputs = vec![
                (transactions::get_precomputed_connector_0_address(), transactions::get_connector_0_amount()), 
                (transactions::get_precomputed_connector_4_address(), transactions::get_connector_4_amount()),
                (transactions::get_precomputed_connector_5_address(), transactions::get_connector_5_amount()), 
                (transactions::get_precomputed_connector_c_address(), transactions::get_connector_c_amount()),

            ];
            let mut outputs = vec![];
            for i in 0..tx.output.len() {
                let output_i_addr = match Address::from_script(&tx.output[i].script_pubkey, config::network()) {
                    Ok(v) => v,
                    Err(e) => { 
                        error!("/get-named-inputs-outputs/{tx_type}/{txid}: fail to calc address: {}", e);
                        return HttpResponse::InternalServerError().body(e.to_string())
                    }
                };
                // let output_i_scrpub = hex::encode(tx.output[i].script_pubkey.clone().into_bytes());
                let output_i_amount = tx.output[i].value;
                outputs.push((output_i_addr, output_i_amount));
            }
            (inputs, outputs)
        },  
        9 => { // Disprove
            let tx= match utils::get_raw_tx(&rpc, txid) {
                Ok(v) => v,
                Err(e) => { 
                    error!("/get-named-inputs-outputs/{tx_type}/{txid}: fail to get tx:{txid} : {}", e);
                    return HttpResponse::InternalServerError().body(e.to_string())
                }
            };
            let inputs = vec![
                (transactions::get_precomputed_connector_5_address(), transactions::get_connector_5_amount()), 
                (transactions::get_precomputed_connector_c_address(), transactions::get_connector_c_amount()),
            ];
            let mut outputs = vec![];
            for i in 0..tx.output.len() {
                let output_i_addr = match Address::from_script(&tx.output[i].script_pubkey, config::network()) {
                    Ok(v) => v,
                    Err(e) => { 
                        error!("/get-named-inputs-outputs/{tx_type}/{txid}: fail to calc address: {}", e);
                        return HttpResponse::InternalServerError().body(e.to_string())
                    }
                };
                // let output_i_scrpub = hex::encode(tx.output[i].script_pubkey.clone().into_bytes());
                let output_i_amount = tx.output[i].value;
                outputs.push((output_i_addr, output_i_amount));
            }
            (inputs, outputs)
        },
        _ => { // Unidentified
            error!("/get-named-inputs-outputs/{tx_type}/{txid}: Unidentified tx type");
            return HttpResponse::BadRequest().body("Unidentified tx type".to_string())
        }
    };

    fn get_inputs_outputs(rpc: &bitcoincore_rpc::Client, tx_type: u8, txid: Txid) -> Result<(Vec<(Address, Amount)>, Vec<(Address, Amount)>), String> {
        let tx= match utils::get_raw_tx(&rpc, txid) {
            Ok(v) => v,
            Err(e) => { 
                error!("/get-named-inputs-outputs/{tx_type}/{txid}: fail to get tx:{txid} : {}", e);
                return Err(e.to_string())
            }
        };
        let mut inputs: Vec<(Address, Amount)> = vec![];
        let mut outputs: Vec<(Address, Amount)> = vec![];
        let mut prev_tx_cache: (Txid, Transaction) = (txid, tx.clone());
        for i in 0..tx.input.len() {
            let prev_txid = tx.input[i].previous_output.txid;
            if prev_txid != prev_tx_cache.0 {
                prev_tx_cache = match utils::get_raw_tx(&rpc, prev_txid) {
                    Ok(v) => (prev_txid, v),
                    Err(e) => { 
                        error!("/get-named-inputs-outputs/{tx_type}/{txid}: fail to get tx:{prev_txid} : {}", e);
                        return Err(e.to_string())
                    }
                };
            };
            let prev_vout = tx.input[i].previous_output.vout;
            let prev_outpoint = match prev_tx_cache.1.output.get(prev_vout as usize) {
                Some(v) => v,
                _ => { 
                    error!("/get-named-inputs-outputs/{tx_type}/{txid}: fail to get prev_txout");
                    return Err("fail to get prev_txout".to_string())
                },
            };
            let input_i_addr = match Address::from_script(&prev_outpoint.script_pubkey, config::network()) {
                Ok(v) => v,
                Err(e) => { 
                    error!("/get-named-inputs-outputs/{tx_type}/{txid}: fail to calc address: {}", e);
                    return Err(e.to_string())
                }
            };
            // let input_i_scrpub = hex::encode(prev_outpoint.script_pubkey.clone().into_bytes());
            let input_i_amount = prev_outpoint.value;
            inputs.push((input_i_addr, input_i_amount));
        }
        for i in 0..tx.output.len() {
            let output_i_addr = match Address::from_script(&tx.output[i].script_pubkey, config::network()) {
                Ok(v) => v,
                Err(e) => { 
                    error!("/get-named-inputs-outputs/{tx_type}/{txid}: fail to calc address: {}", e);
                    return Err(e.to_string())
                }
            };
            // let output_i_scrpub = hex::encode(tx.output[i].script_pubkey.clone().into_bytes());
            let output_i_amount = tx.output[i].value;
            outputs.push((output_i_addr, output_i_amount));
        }
        Ok((inputs, outputs))
    }

    let revealers = vec!["revealer"; 59];
    let (tx_name, input_names, output_names) = match (inputs.len(), outputs.len()) {
        (1,1) => (
            "Peg-In",
            vec!["User"],
            vec!["connector-0"],
        ),
        (1,3) => (
            "Kickoff-1",
            vec!["Operator"],
            vec!["connector-a","connector-1","connector-2"],
        ),
        (1,61) => (
            "Kickoff-2",
            vec!["connector-1"],
            [vec!["connector-3","connector-b"], revealers].concat(),
        ),
        (2,1) => (
            "Challenge",
            vec!["connector-a","Challenger"],
            vec!["Operator"],
        ),
        (60,3) => (
            "Assert",
            [vec!["connector-b"], revealers].concat(),
            vec!["connector-4","connector-5","connector-c"]
        ),
        (2,2) => (
            "Disprove",
            vec!["connector-5","connector-c"],
            vec!["Challenger","Burn"],
        ),
        (4,1) => {
            let input_3_addr = inputs[3].0.clone();
            let connector_c_addr = transactions::get_precomputed_connector_c_address();
            if input_3_addr != connector_c_addr {
                (
                    "Take-1",
                    vec!["connector-0","connector-3","connector-a","connector-b"],
                    vec!["Operator"],
                )
            } else {
                (
                    "Take-2",
                    vec!["connector-0","connector-4","connector-5","connector-c"],
                    vec!["Operator"],
                )
            }
        },
        (x,y) => (
            "Unidentified",
            vec![""; x],
            vec![""; y],
        )
    };

    if tx_name == "Unidentified" {
        warn!("/get-named-inputs-outputs/{tx_type}/{txid}: unidentified tx");
    };

    let inputs = input_names.into_iter()
        .zip(inputs.into_iter())
        .map(|(x,(y,z))| (x.to_string(),y,z))
        .collect();

    let outputs = output_names.into_iter()
        .zip(outputs.into_iter())
        .map(|(x,(y,z))| (x.to_string(),y,z))
        .collect();

    let tx_name = tx_name.to_string();

    let body = serde_json::to_string_pretty(&ResponseStruct{tx_name, inputs, outputs}).unwrap();
    info!("/get-named-inputs-outputs/{tx_type}/{txid}: ok");
    HttpResponse::Ok()
        .content_type(ContentType::json())
        .body(body)
}

/* 
#[get("/get-tx-inputs-outputs/{txid}")]
async fn get_tx_inputs_outputs(path: web::Path<String>) -> impl Responder {
    #[derive(Serialize)]
    struct ResponseStruct {
        inputs: Vec<(Address, Amount)>,
        outputs: Vec<(Address, Amount)>,
    }

    let txid = path.into_inner();
    info!("new REQUEST: /get-tx-inputs-outputs/{txid}");
    let txid = match utils::txid_from_str(&txid) {
        Ok(v) => v,
        Err(e) => { 
            error!("/get-tx-inputs-outputs/{txid}: fail to decode txid: {}", e);
            return HttpResponse::BadRequest().body(e.to_string())
        }
    };

    let rpc = match utils::new_rpc_client().await {
        Ok(v) => v,
        Err(e) => { 
            error!("/get-tx-inputs-outputs/{txid}: fail to connect bitcoind: {}", e);
            return HttpResponse::InternalServerError().body(e.to_string())
        }
    };

    let tx= match utils::get_raw_tx(&rpc, txid) {
        Ok(v) => v,
        Err(e) => { 
            error!("/get-tx-inputs-outputs/{txid}: fail to get tx:{txid} : {}", e);
            return HttpResponse::InternalServerError().body(e.to_string())
        }
    };

    let mut inputs = vec![];
    let mut outputs = vec![];
    let mut prev_tx_cache: (Txid, Transaction) = (txid, tx.clone());

    for i in 0..tx.input.len() {
        let prev_txid = tx.input[i].previous_output.txid;
        if prev_txid != prev_tx_cache.0 {
            prev_tx_cache = match utils::get_raw_tx(&rpc, prev_txid) {
                Ok(v) => (prev_txid, v),
                Err(e) => { 
                    error!("/get-tx-inputs-outputs/{txid}: fail to get tx:{prev_txid} : {}", e);
                    return HttpResponse::InternalServerError().body(e.to_string())
                }
            };
        };
        let prev_vout = tx.input[i].previous_output.vout;
        let prev_outpoint = match prev_tx_cache.1.output.get(prev_vout as usize) {
            Some(v) => v,
            _ => { 
                error!("/get-tx-inputs-outputs/{txid}: fail to get prev_txout");
                return HttpResponse::InternalServerError().body("fail to get prev_txout")
            },
        };
        let input_i_addr = match Address::from_script(&prev_outpoint.script_pubkey, config::network()) {
            Ok(v) => v,
            Err(e) => { 
                error!("/get-tx-inputs-outputs/{txid}: fail to calc address: {}", e);
                return HttpResponse::InternalServerError().body(e.to_string())
            }
        };
        let input_i_amount = prev_outpoint.value;
        inputs.push((input_i_addr, input_i_amount));
    }
    for i in 0..tx.output.len() {
        let output_i_addr = match Address::from_script(&tx.output[i].script_pubkey, config::network()) {
            Ok(v) => v,
            Err(e) => { 
                error!("/get-tx-inputs-outputs/{txid}: fail to calc address: {}", e);
                return HttpResponse::InternalServerError().body(e.to_string())
            }
        };
        let output_i_amount = tx.output[i].value;
        outputs.push((output_i_addr, output_i_amount));
    }

    let body = serde_json::to_string_pretty(&ResponseStruct{inputs, outputs}).unwrap();
    info!("/get-tx-inputs-outputs/{txid}: ok");
    HttpResponse::Ok()
        .content_type(ContentType::json())
        .body(body)
}
*/

#[post("/get-user-workflow/{user_address}")]
async fn get_user_workflow(path: web::Path<String>) -> impl Responder {
    #[derive(Serialize)]
    struct UserDataLite {
        pub status: u8,
        pub fake_index: Option<u32>,
        pub faucet_1: Option<Txid>,
        pub faucet_2: Option<Txid>,
        pub pegin: Option<Txid>,
        pub kickoff_1: Option<Txid>,
        pub kickoff_2: Option<Txid>,
        pub challenge: Option<Txid>,
        pub assert: Option<Txid>,
        pub disprove: Option<Txid>,
        pub take_1: Option<Txid>,
        pub take_2: Option<Txid>,
    }
    #[derive(Serialize)]
    struct ResponseStruct {
        workflow_id: i32,
        workflow: UserDataLite,
    }

    let user_addr = path.into_inner();
    info!("new REQUEST: /get-user-workflow/{user_addr}");
    let user_addr = match utils::address_from_str(&user_addr) {
        Ok(v) => v,
        Err(e) => { 
            error!("/get-user-workflow/{user_addr}: fail to deocde address: {}",e);
            return HttpResponse::BadRequest().body(e.to_string())
        }
    };
    let db = match sql::open_db() {
        Ok(v) => v,
        Err(e) => { 
            error!("/get-user-workflow/{user_addr}: fail to connect db: {}",e);
            return HttpResponse::InternalServerError().body(e.to_string())
        }
    };
    let workflow_id = match sql::get_user_id(&db, &user_addr) {
        Ok(id_option) => match id_option {
            Some(id) => id,
            _ => {
                if let Err(e) = sql::new_user(&db, &user_addr) {
                    error!("/get-user-workflow/{user_addr}: fail to new user workflow: {}",e);
                    return HttpResponse::InternalServerError().body(e.to_string())
                };
                match sql::get_user_id(&db, &user_addr) {
                    Ok(id) => id.unwrap(),
                    Err(e) => { 
                        error!("/get-user-workflow/{user_addr}: fail to get user id after new_user: {}",e);
                        return HttpResponse::InternalServerError().body(e.to_string())
                    }
                }
            }
        },
        Err(e) => { 
            error!("/get-user-workflow/{user_addr}: fail to get user id: {}",e);
            return HttpResponse::InternalServerError().body(e.to_string())
        }
    };
    let workflow = match sql::get_user_data(&db, workflow_id) {
        Ok(user_data_option) => { match user_data_option {
                Some(data) => data,
                _ => { 
                    error!("/get-user-workflow/{user_addr}: workflow {workflow_id} does not exisit");
                    return HttpResponse::InternalServerError().body(format!("workflow {workflow_id} does not exisit"))
                }
            }
        },
        Err(e) => { 
            error!("/get-user-workflow/{user_addr}: fail to get user data: {}",e);
            return HttpResponse::InternalServerError().body(e.to_string())
        }
    };
    let workflow = UserDataLite {
        status: workflow.status,
        fake_index: workflow.fake_index,
        faucet_1: match workflow.faucet_1 {
            Some((txid, _)) => Some(txid),
            _ => None,
        },
        faucet_2: match workflow.faucet_2 {
            Some((txid, _)) => Some(txid),
            _ => None,
        },
        pegin: workflow.pegin,
        kickoff_1: workflow.kickoff_1,
        kickoff_2: workflow.kickoff_2,
        challenge: workflow.challenge,
        assert: workflow.assert,
        disprove: workflow.disprove,
        take_1: workflow.take_1,
        take_2: workflow.take_2,
    };

    let body = serde_json::to_string_pretty(&ResponseStruct{workflow_id,workflow}).unwrap();
    info!("/get-user-workflow/{user_addr}: ok");
    HttpResponse::Ok()
        .content_type(ContentType::json())
        .body(body)
}

#[get("/get-workflow-info/{workflow_id}")]
async fn get_workflow_info(path: web::Path<i32>) -> impl Responder {
    #[derive(Serialize)]
    struct ResponseStruct {
        pub status: u8,
        pub fake_index: Option<u32>,
        pub faucet_1: Option<Txid>,
        pub faucet_2: Option<Txid>,
        pub pegin: Option<Txid>,
        pub kickoff_1: Option<Txid>,
        pub kickoff_2: Option<Txid>,
        pub challenge: Option<Txid>,
        pub assert: Option<Txid>,
        pub disprove: Option<Txid>,
        pub take_1: Option<Txid>,
        pub take_2: Option<Txid>,
    }

    let workflow_id = path.into_inner();
    info!("new REQUEST: /get-workflow-info/{workflow_id}");
    let db = match sql::open_db() {
        Ok(v) => v,
        Err(e) => { 
            error!("/get-workflow-info/{workflow_id}: fail to connect db: {}",e);
            return HttpResponse::InternalServerError().body(e.to_string())
        }
    };

    let user_data = match sql::get_user_data(&db, workflow_id) {
        Ok(user_data_option) => { match user_data_option {
                Some(data) => data,
                _ => {
                    error!("/get-workflow-info/{workflow_id}: workflow {workflow_id} does not exisit");
                    return HttpResponse::BadRequest().body(format!("workflow {workflow_id} does not exisit"))
                }
            }
        },
        Err(e) => { 
            error!("/get-workflow-info/{workflow_id}: fail to get user data: {}",e);
            return HttpResponse::InternalServerError().body(e.to_string())
        }
    };

    let workflow = ResponseStruct {
        status: user_data.status,
        fake_index: user_data.fake_index,
        faucet_1: match user_data.faucet_1 {
            Some((txid, _)) => Some(txid),
            _ => None,
        },
        faucet_2: match user_data.faucet_2 {
            Some((txid, _)) => Some(txid),
            _ => None,
        },
        pegin: user_data.pegin,
        kickoff_1: user_data.kickoff_1,
        kickoff_2: user_data.kickoff_2,
        challenge: user_data.challenge,
        assert: user_data.assert,
        disprove: user_data.disprove,
        take_1: user_data.take_1,
        take_2: user_data.take_2,
    };

    let body = serde_json::to_string_pretty(&workflow).unwrap();
    info!("/get-workflow-info/{workflow_id}: ok");
    HttpResponse::Ok()
        .content_type(ContentType::json())
        .body(body)
}

#[post("/request-btc/{user_address}")]
async fn request_btc(path: web::Path<String>) -> impl Responder {
    fn create_new_user(db: &Connection,user_addr: &Address) -> Result<(i32, UserData), String> {
        if let Err(e) = sql::new_user(&db, &user_addr) {
            return Err(e.to_string())
        };
        let workflow_id = match sql::get_user_id(&db, &user_addr) {
            Ok(id) => id.unwrap(),
            Err(e) => return Err(e.to_string()),
        };
        let user_data = match sql::get_user_data(&db, workflow_id) {
            Ok(user_data_option) => { match user_data_option {
                    Some(data) => data,
                    _ => return Err(format!("workflow {workflow_id} does not exisit")),
                }
            },
            Err(e) => return Err(e.to_string()),
        };
        Ok((workflow_id, user_data))
    }

    #[derive(Serialize)]
    struct ResponseStruct {
        workflow_id: i32,
        faucet_txid: Txid,
    }

    let user_addr = path.into_inner();
    info!("new REQUEST: /request-btc/{user_addr}");
    let user_addr = match utils::address_from_str(&user_addr) {
        Ok(v) => v,
        Err(e) => {
            error!("/request-btc/{user_addr}: fail to deocde address: {}",e);
            return HttpResponse::BadRequest().body(e.to_string())
        }
    };

    let db = match sql::open_db() {
        Ok(v) => v,
        Err(e) => { 
            error!("/request-btc/{user_addr}: fail to connect db: {}",e);
            return HttpResponse::InternalServerError().body(e.to_string())
        }
    };

    let (workflow_id, mut user_data) = match sql::get_user_id(&db, &user_addr) {
        Ok(id_option) => {
            match id_option {
                Some(workflow_id) => {
                    let user_data = match sql::get_user_data(&db, workflow_id) {
                        Ok(user_data_option) => { match user_data_option {
                                Some(data) => data,
                                _ => {
                                    error!("/request-btc/{user_addr}: workflow {workflow_id} does not exisit");
                                    return HttpResponse::BadRequest().body(format!("workflow {workflow_id} does not exisit"))
                                }
                            }
                        },
                        Err(e) => { 
                            error!("/request-btc/{user_addr}: fail to get user data: {}",e);
                            return HttpResponse::InternalServerError().body(e.to_string())
                        }
                    };
                    if user_data.status != sql::STATUS::EMPTY as u8 {
                        match create_new_user(&db, &user_addr) {
                            Ok(v) => v,
                            Err(e) => { 
                                error!("/request-btc/{user_addr}: fail to create new user: {}",e);
                                return HttpResponse::InternalServerError().body(e.to_string())
                            }
                        }
                    } else {
                        (workflow_id, user_data)
                    }
                },
                _ =>  {
                    match create_new_user(&db, &user_addr) {
                        Ok(v) => v,
                        Err(e) => { 
                            error!("/request-btc/{user_addr}: fail to create new user: {}",e);
                            return HttpResponse::InternalServerError().body(e.to_string())
                        }
                    }
                },          
            }
        },
        Err(e) => { 
            error!("/request-btc/{user_addr}: fail to get user id & data: {}",e);
            return HttpResponse::InternalServerError().body(e.to_string())
        }
    };

    let rpc = match utils::new_rpc_client().await {
        Ok(v) => v,
        Err(e) => { 
            error!("/request-btc/{user_addr}: fail to connect bitcoind: {}", e);
            return HttpResponse::InternalServerError().body(e.to_string())
        }
    };
    let (faucet_outpoint_1, faucet_outpoint_2) = match transactions::faucet(&rpc, &user_addr) {
        Ok(v) => v,
        Err(e) => { 
            error!("/request-btc/{user_addr}: fail to send faucet tx: {}", e);
            return HttpResponse::InternalServerError().body(e.to_string())
        }
    };

    user_data.status = sql::STATUS::FAUCET as u8;
    user_data.faucet_1 = Some((faucet_outpoint_1.txid, faucet_outpoint_1.vout));
    user_data.faucet_2 = Some((faucet_outpoint_2.txid, faucet_outpoint_2.vout));

    if let Err(e) = sql::update_user_data(&db, workflow_id, &user_data) {
        error!("/request-btc/{user_addr}: fail to update user data: {}", e);
        return HttpResponse::InternalServerError().body(e.to_string())
    }

    let faucet_txid = faucet_outpoint_1.txid;
    let body = serde_json::to_string_pretty(&ResponseStruct{workflow_id, faucet_txid}).unwrap();
    info!("/request-btc/{user_addr}: ok");
    HttpResponse::Ok()
        .content_type(ContentType::json())
        .body(body)
}

#[get("/get-unsigned-pegin-tx/{workflow_id}")]
async fn get_unsigned_pegin_tx(path: web::Path<i32>) -> impl Responder {
    #[derive(Serialize)]
    struct ResponseStruct {
        input: TxInput,
        outputs: Vec<TxOutput>,
    }

    let workflow_id = path.into_inner();
    info!("new REQUEST: /get-unsigned-pegin-tx/{workflow_id}");
    let db = match sql::open_db() {
        Ok(v) => v,
        Err(e) => { 
            error!("/get-unsigned-pegin-tx/{workflow_id}: fail to connect db: {}",e);
            return HttpResponse::InternalServerError().body(e.to_string())
        }
    };

    let user_data = match sql::get_user_data(&db, workflow_id) {
        Ok(user_data_option) => { match user_data_option {
                Some(data) => data,
                _ => {
                    error!("/get-unsigned-pegin-tx/{workflow_id}: workflow {workflow_id} does not exisit");
                    return HttpResponse::BadRequest().body(format!("workflow {workflow_id} does not exisit"))
                }
            }
        },
        Err(e) => { 
            error!("/get-unsigned-pegin-tx/{workflow_id}: fail to get user data: {}",e);
            return HttpResponse::InternalServerError().body(e.to_string())
        }
    };

    if user_data.status != sql::STATUS::FAUCET as u8 {
        error!("/get-unsigned-pegin-tx/{workflow_id}: workflow {workflow_id} not currently at faucet stage");
        return HttpResponse::BadRequest().body(format!("workflow {workflow_id} not currently at faucet stage"))
    };

    let (faucet_1_txid, faucet_1_vout) = match user_data.faucet_1 {
        Some(v) => v,
        _ => {
            error!("/get-unsigned-pegin-tx/{workflow_id}: workflow {workflow_id} missing faucet_1_txid");
            return HttpResponse::InternalServerError().body("workflow {workflow_id} missing faucet_1_txid".to_string())
        }
    };

    let rpc = match utils::new_rpc_client().await {
        Ok(v) => v,
        Err(e) => { 
            error!("/get-unsigned-pegin-tx/{workflow_id}: fail to connect bitcoind: {}",e);
            return HttpResponse::InternalServerError().body(e.to_string())
        }
    };

    let pegin_tx = match transactions::peg_in_prepare(&rpc, faucet_1_txid, faucet_1_vout) {
        Ok(v) => v,
        Err(e) => { 
            error!("/get-unsigned-pegin-tx/{workflow_id}: fail to prepare pegin tx: {}",e);
            return HttpResponse::InternalServerError().body(e.to_string())
        }
    };

    let txid = pegin_tx.input[0].previous_output.txid;
    let vout = pegin_tx.input[0].previous_output.vout;
    let (script_pubkey, value) = match utils::get_utxo_script_pubkey_value(&rpc, txid, vout) {
        Ok(v) => v,
        Err(e) => { 
            error!("/get-unsigned-pegin-tx/{workflow_id}: fail to get_utxo_value: {}",e);
            return HttpResponse::InternalServerError().body(e.to_string())
        }
    };
    let input = TxInput {
        txid,
        vout,
        script_pubkey,
        value,
    };

    let mut outputs = vec![];
    for i in 0..pegin_tx.output.len() {
        let testnet_address = Address::from_script(&pegin_tx.output[i].script_pubkey, bitcoin::Network::Testnet).unwrap();
        let regtest_address = Address::from_script(&pegin_tx.output[i].script_pubkey, bitcoin::Network::Regtest).unwrap();
        let value = pegin_tx.output[i].value;
        let output_i = TxOutput {
            testnet_address,
            regtest_address,
            value,
        };
        outputs.push(output_i)
    };

    let body = serde_json::to_string_pretty(&ResponseStruct{input, outputs}).unwrap();
    info!("/get-unsigned-pegin-tx/{workflow_id}: ok");
    HttpResponse::Ok()
        .content_type(ContentType::json())
        .body(body)
}

#[post("/post-pegin-txid/{workflow_id}/{pegin_txid}")]
async fn post_pegin_txid(path: web::Path<(i32, String)>) -> impl Responder {
    #[derive(Serialize)]
    struct ResponseStruct {
        success: bool,
    }

    let (workflow_id, pegin_txid) = path.into_inner();
    info!("new REQUEST: /post-pegin-txid/{workflow_id}/{pegin_txid}");
    let pegin_txid = match utils::txid_from_str(&pegin_txid) {
        Ok(v) => v,
        Err(e) => { 
            error!("/post-pegin-txid/{workflow_id}/{pegin_txid}: fail to decode txid: {}", e);
            return HttpResponse::BadRequest().body(e.to_string())
        }
    };

    let db = match sql::open_db() {
        Ok(v) => v,
        Err(e) => { 
            error!("/post-pegin-txid/{workflow_id}/{pegin_txid}: fail to connect db: {}", e);
            return HttpResponse::InternalServerError().body(e.to_string())
        }
    };

    match sql::is_workflow_locked(&db, workflow_id) {
        Ok(v) => {
            if v {
                error!("/post-pegin-txid/{workflow_id}/{pegin_txid}: workflow:{workflow_id} is processing a task, please wait");
                return HttpResponse::Locked().body("workflow:{workflow_id} is processing a task, please wait".to_string())
            };
        },
        Err(e) => {
            error!("/post-pegin-txid/{workflow_id}/{pegin_txid}: fail to get workflow lock: {}", e);
            return HttpResponse::InternalServerError().body(e.to_string())
        }
    };

    if let Err(e) = sql::lock_workflow(&db, workflow_id) {
        error!("/post-pegin-txid/{workflow_id}/{pegin_txid}: fail to lock workflow: {}", e);
        return HttpResponse::InternalServerError().body(e.to_string());
    };

    let mut user_data = match sql::get_user_data(&db, workflow_id) {
        Ok(user_data_option) => { match user_data_option {
                Some(data) => data,
                _ => {
                    error!("/post-pegin-txid/{workflow_id}/{pegin_txid}: workflow {workflow_id} does not exisit");
                    return HttpResponse::BadRequest().body(format!("workflow {workflow_id} does not exisit"))
                }
            }
        },
        Err(e) => { 
            error!("/post-pegin-txid/{workflow_id}/{pegin_txid}: fail to get user data: {}", e);
            return HttpResponse::InternalServerError().body(e.to_string())
        }
    };

    if user_data.status != sql::STATUS::FAUCET as u8 {
        error!("/post-pegin-txid/{workflow_id}/{pegin_txid}: workflow {workflow_id} not currently at faucet stage");
        return HttpResponse::BadRequest().body(format!("workflow {workflow_id} not currently at faucet stage"))
    };

    user_data.status = sql::STATUS::PEGIN as u8;
    user_data.pegin = Some(pegin_txid);

    if let Err(e) = sql::update_user_data(&db, workflow_id, &user_data) {
        error!("/post-pegin-txid/{workflow_id}/{pegin_txid}: fail to update user data: {}", e);
        return HttpResponse::InternalServerError().body(e.to_string())
    }

    if let Err(e) = sql::unlock_workflow(&db, workflow_id) {
        error!("/post-pegin-txid/{workflow_id}/{pegin_txid}: fail to unlock workflow: {}", e);
        return HttpResponse::InternalServerError().body(e.to_string())
    };

    let body = serde_json::to_string_pretty(&ResponseStruct{success: true}).unwrap();
    info!("/post-pegin-txid/{workflow_id}/{pegin_txid}: ok");
    HttpResponse::Ok()
        .content_type(ContentType::json())
        .body(body)
}

#[post("/post-fake-index/{workflow_id}/{fake_index}")]
async fn post_fake_index(path: web::Path<(i32, u32)>) -> impl Responder {
    #[derive(Serialize)]
    struct ResponseStruct {
        success: bool,
    }

    let (workflow_id, fake_index) = path.into_inner();
    info!("new REQUEST: /post-fake-index/{workflow_id}/{fake_index}");
    let db = match sql::open_db() {
        Ok(v) => v,
        Err(e) => {
            error!("/post-fake-index/{workflow_id}/{fake_index}: fail to connect db: {}",e);
            return HttpResponse::InternalServerError().body(e.to_string())
        }
    };

    match sql::is_workflow_locked(&db, workflow_id) {
        Ok(v) => {
            if v {
                error!("/post-fake-index/{workflow_id}/{fake_index}: workflow:{workflow_id} is processing a task, please wait");
                return HttpResponse::Locked().body("workflow:{workflow_id} is processing a task, please wait".to_string())
            };
        },
        Err(e) => {
            error!("/post-fake-index/{workflow_id}/{fake_index}: fail to get workflow lock: {}", e);
            return HttpResponse::InternalServerError().body(e.to_string())
        }
    };

    if let Err(e) = sql::lock_workflow(&db, workflow_id) {
        error!("/post-fake-index/{workflow_id}/{fake_index}: fail to lock workflow: {}", e);
        return HttpResponse::InternalServerError().body(e.to_string());
    };

    let mut user_data = match sql::get_user_data(&db, workflow_id) {
        Ok(user_data_option) => { match user_data_option {
                Some(data) => data,
                _ => {
                    error!("/post-fake-index/{workflow_id}/{fake_index}: workflow {workflow_id} does not exisit");
                    return HttpResponse::BadRequest().body(format!("workflow {workflow_id} does not exisit"))
                }
            }
        },
        Err(e) => {
            error!("/post-fake-index/{workflow_id}/{fake_index}: fail to get user data: {}",e);
            return HttpResponse::InternalServerError().body(e.to_string())
        }
    };

    if user_data.status != sql::STATUS::PEGIN as u8 {
        error!("/post-fake-index/{workflow_id}/{fake_index}: workflow {workflow_id} not currently at pegin stage");
        return HttpResponse::BadRequest().body(format!("workflow {workflow_id} not currently at pegin stage"))
    };

    let fake_index = if fake_index > 614 { 
        warn!("/post-fake-index/{workflow_id}/{fake_index}: index out of range");
        1 
    } else {
        fake_index 
    };
    user_data.fake_index = Some(fake_index);

    if let Err(e) = sql::update_user_data(&db, workflow_id, &user_data) {
        error!("/post-fake-index/{workflow_id}/{fake_index}: fail to update user data: {}",e);
        return HttpResponse::InternalServerError().body(e.to_string())
    }

    if let Err(e) = sql::unlock_workflow(&db, workflow_id) {
        error!("/post-fake-index/{workflow_id}/{fake_index}: fail to unlock workflow: {}", e);
        return HttpResponse::InternalServerError().body(e.to_string());
    };

    let body = serde_json::to_string_pretty(&ResponseStruct{success:true}).unwrap();
    info!("/post-fake-index/{workflow_id}/{fake_index}: ok");
    HttpResponse::Ok()
        .content_type(ContentType::json())
        .body(body)
}

#[get("/get-unsigned-kickoff1-tx/{workflow_id}")]
async fn get_unsigned_kickoff1_tx(path: web::Path<i32>) -> impl Responder {
    #[derive(Serialize)]
    struct ResponseStruct {
        input: TxInput,
        outputs: Vec<TxOutput>,
    }

    let workflow_id = path.into_inner();
    info!("new REQUEST: /get-unsigned-kickoff1-tx/{workflow_id}");
    let db = match sql::open_db() {
        Ok(v) => v,
        Err(e) => { 
            error!("/get-unsigned-kickoff1-tx/{workflow_id}: fail to connect db: {}",e);
            return HttpResponse::InternalServerError().body(e.to_string())
        }
    };

    let user_data = match sql::get_user_data(&db, workflow_id) {
        Ok(user_data_option) => { match user_data_option {
                Some(data) => data,
                _ => {
                    error!("/get-unsigned-kickoff1-tx/{workflow_id}: workflow {workflow_id} does not exisit");
                    return HttpResponse::BadRequest().body(format!("workflow {workflow_id} does not exisit"))
                }
            }
        },
        Err(e) => return HttpResponse::InternalServerError().body(e.to_string()),
    };

    if user_data.status != sql::STATUS::PEGIN as u8 {
        error!("/get-unsigned-kickoff1-tx/{workflow_id}: workflow {workflow_id} not currently at pegin stage");
        return HttpResponse::BadRequest().body(format!("workflow {workflow_id} not currently at pegin stage"))
    };

    let (faucet_2_txid, faucet_2_vout) = match user_data.faucet_2 {
        Some(v) => v,
        _ => {
            error!("/get-unsigned-kickoff1-tx/{workflow_id}: workflow {workflow_id} missing faucet_2_txid");
            return HttpResponse::InternalServerError().body("workflow {workflow_id} missing faucet_2_txid".to_string())
        }
    };

    let rpc = match utils::new_rpc_client().await {
        Ok(v) => v,
        Err(e) => { 
            error!("/get-unsigned-kickoff1-tx/{workflow_id}: fail to connect bitcoind: {}",e);
            return HttpResponse::InternalServerError().body(e.to_string())
        }
    };

    let kickoff_1_tx = match transactions::kickoff_1_prepare(&rpc, faucet_2_txid, faucet_2_vout) {
        Ok(v) => v,
        Err(e) => { 
            error!("/get-unsigned-kickoff1-tx/{workflow_id}: fail to prepare kickoff1 tx: {}",e);
            return HttpResponse::InternalServerError().body(e.to_string())
        }
    };

    let txid = kickoff_1_tx.input[0].previous_output.txid;
    let vout = kickoff_1_tx.input[0].previous_output.vout;
    let (script_pubkey, value) = match utils::get_utxo_script_pubkey_value(&rpc, txid, vout) {
        Ok(v) => v,
        Err(e) => { 
            error!("/get-unsigned-kickoff1-tx/{workflow_id}: fail to get_utxo_value: {}",e);
            return HttpResponse::InternalServerError().body(e.to_string())
        }
    };
    let input = TxInput {
        txid,
        vout,
        script_pubkey,
        value,
    };

    let mut outputs = vec![];

    for i in 0..kickoff_1_tx.output.len() {
        let testnet_address = Address::from_script(&kickoff_1_tx.output[i].script_pubkey, bitcoin::Network::Testnet).unwrap();
        let regtest_address = Address::from_script(&kickoff_1_tx.output[i].script_pubkey, bitcoin::Network::Regtest).unwrap();
        let value = kickoff_1_tx.output[i].value;
        let output_i = TxOutput {
            testnet_address,
            regtest_address,
            value,
        };
        outputs.push(output_i)
    };

    let body = serde_json::to_string_pretty(&ResponseStruct{input, outputs}).unwrap();
    info!("/get-unsigned-kickoff1-tx/{workflow_id}: ok");
    HttpResponse::Ok()
        .content_type(ContentType::json())
        .body(body)
}

#[post("/send-kickoff2/{workflow_id}/{kickoff_1_txid}")]
async fn send_kickoff_2(path: web::Path<(i32, String)>) -> impl Responder {
    #[derive(Serialize)]
    struct ResponseStruct {
        kick_off_2_txid: Txid,
    }

    let (workflow_id, kickoff_1_txid) = path.into_inner();
    info!("new REQUEST: /send-kickoff2/{workflow_id}/{kickoff_1_txid}");
    let kick_off_1_txid = match utils::txid_from_str(&kickoff_1_txid) {
        Ok(v) => v,
        Err(e) => { 
            error!("/send-kickoff2/{workflow_id}/{kickoff_1_txid}: fail to decode txid: {}", e);
            return HttpResponse::BadRequest().body(e.to_string())
        }
    };

    let db = match sql::open_db() {
        Ok(v) => v,
        Err(e) => { 
            error!("/send-kickoff2/{workflow_id}/{kickoff_1_txid}: fail to connect db: {}", e);
            return HttpResponse::InternalServerError().body(e.to_string())
        }
    };

    match sql::is_workflow_locked(&db, workflow_id) {
        Ok(v) => {
            if v {
                error!("/send-kickoff2/{workflow_id}/{kickoff_1_txid}: workflow:{workflow_id} is processing a task, please wait");
                return HttpResponse::Locked().body("workflow:{workflow_id} is processing a task, please wait".to_string())
            };
        },
        Err(e) => {
            error!("/send-kickoff2/{workflow_id}/{kickoff_1_txid}: fail to get workflow lock: {}", e);
            return HttpResponse::InternalServerError().body(e.to_string())
        }
    };

    if let Err(e) = sql::lock_workflow(&db, workflow_id) {
        error!("/send-kickoff2/{workflow_id}/{kickoff_1_txid}: fail to lock workflow: {}", e);
        return HttpResponse::InternalServerError().body(e.to_string());
    };

    let mut user_data = match sql::get_user_data(&db, workflow_id) {
        Ok(user_data_option) => { match user_data_option {
                Some(data) => data,
                _ => {
                    error!("/send-kickoff2/{workflow_id}/{kickoff_1_txid}: workflow {workflow_id} does not exisit");
                    return HttpResponse::BadRequest().body(format!("workflow {workflow_id} does not exisit"))
                }
            }
        },
        Err(e) => { 
            error!("/send-kickoff2/{workflow_id}/{kickoff_1_txid}: fail to get user data: {}", e);
            return HttpResponse::InternalServerError().body(e.to_string())
        }
    };

    if user_data.status == sql::STATUS::PEGIN as u8 {
        user_data.status = sql::STATUS::KICKOFF1 as u8;
    } else if user_data.status != sql::STATUS::KICKOFF1 as u8 {
        error!("/send-kickoff2/{workflow_id}/{kickoff_1_txid}: workflow {workflow_id} not currently at pegin/kickoff1 stage");
        return HttpResponse::BadRequest().body(format!("workflow {workflow_id} not currently at pegin/kickoff1 stage"))
    };

    user_data.kickoff_1 = Some(kick_off_1_txid);

    if let Err(e) = sql::update_user_data(&db, workflow_id, &user_data) {
        error!("/send-kickoff2/{workflow_id}/{kickoff_1_txid}: fail to update user data: {}", e);
        return HttpResponse::InternalServerError().body(e.to_string())
    }

    let rpc = match utils::new_rpc_client().await {
        Ok(v) => v,
        Err(e) => { 
            error!("/send-kickoff2/{workflow_id}/{kickoff_1_txid}: fail to connect bitcoind: {}", e);
            return HttpResponse::InternalServerError().body(e.to_string())
        }
    };
    let bitcom_lock_scripts = transactions::borrow_bitcom_lock_scripts();
    let kick_off_2_txid = match transactions::kick_off_2(&rpc, kick_off_1_txid, bitcom_lock_scripts).await {
        Ok(v) => v,
        Err(e) => { 
            error!("/send-kickoff2/{workflow_id}/{kickoff_1_txid}: fail to send kickoff2 tx: {}", e);
            return HttpResponse::InternalServerError().body(e.to_string())
        }
    };

    user_data.status = sql::STATUS::KICKOFF2 as u8;
    user_data.kickoff_2 = Some(kick_off_2_txid);

    if let Err(e) = sql::update_user_data(&db, workflow_id, &user_data) {
        error!("/send-kickoff2/{workflow_id}/{kickoff_1_txid}: fail to update user data: {}", e);
        return HttpResponse::InternalServerError().body(e.to_string())
    }

    if let Err(e) = sql::unlock_workflow(&db, workflow_id) {
        error!("/send-kickoff2/{workflow_id}/{kickoff_1_txid}: fail to unlock workflow: {}", e);
        return HttpResponse::InternalServerError().body(e.to_string());
    };

    let body = serde_json::to_string_pretty(&ResponseStruct{kick_off_2_txid}).unwrap();
    info!("/send-kickoff2/{workflow_id}/{kickoff_1_txid}: ok");
    HttpResponse::Ok()
        .content_type(ContentType::json())
        .body(body)
}

#[post("/send-challenge/{workflow_id}")]
async fn send_challenge(path: web::Path<i32>) -> impl Responder {
    #[derive(Serialize)]
    struct ResponseStruct {
        challenge_txid: Txid,
    }

    let workflow_id = path.into_inner();
    info!("new REQUEST: /send-challenge/{workflow_id}");
    let db = match sql::open_db() {
        Ok(v) => v,
        Err(e) => { 
            error!("/send-challenge/{workflow_id}: fail to connect db: {}", e);
            return HttpResponse::InternalServerError().body(e.to_string())
        }
    };

    match sql::is_workflow_locked(&db, workflow_id) {
        Ok(v) => {
            if v {
                error!("/send-challenge/{workflow_id}: workflow:{workflow_id} is processing a task, please wait");
                return HttpResponse::Locked().body("workflow:{workflow_id} is processing a task, please wait".to_string())
            };
        },
        Err(e) => {
            error!("/send-challenge/{workflow_id}: fail to get workflow lock: {}", e);
            return HttpResponse::InternalServerError().body(e.to_string())
        }
    };

    if let Err(e) = sql::lock_workflow(&db, workflow_id) {
        error!("/send-challenge/{workflow_id}: fail to lock workflow: {}", e);
        return HttpResponse::InternalServerError().body(e.to_string());
    };

    let mut user_data = match sql::get_user_data(&db, workflow_id) {
        Ok(user_data_option) => { match user_data_option {
                Some(data) => data,
                _ => {
                    error!("/send-challenge/{workflow_id}: workflow {workflow_id} does not exisit");
                    return HttpResponse::BadRequest().body(format!("workflow {workflow_id} does not exisit"))
                }
            }
        },
        Err(e) => { 
            error!("/send-challenge/{workflow_id}: fail to get user data: {}", e);
            return HttpResponse::InternalServerError().body(e.to_string())
        }
    };

    if user_data.status != sql::STATUS::KICKOFF2 as u8 {
        error!("/send-challenge/{workflow_id}: workflow {workflow_id} not currently at kickoff2 stage");
        return HttpResponse::BadRequest().body(format!("workflow {workflow_id} not currently at kickoff2 stage"))
    };

    let kick_off_1_txid = match user_data.kickoff_1 {
        Some(txid) => txid,
        _ => {
            error!("/send-challenge/{workflow_id}: workflow {workflow_id} missing kickoff_1_txid");
            return HttpResponse::InternalServerError().body("workflow {workflow_id} missing kickoff_1_txid".to_string())
        }
    };

    let rpc = match utils::new_rpc_client().await {
        Ok(v) => v,
        Err(e) => { 
            error!("/send-challenge/{workflow_id}: fail to connect bitcoind: {}", e);
            return HttpResponse::InternalServerError().body(e.to_string())
        }
    };
    let challenge_txid = match transactions::challenge(&rpc, kick_off_1_txid).await {
        Ok(v) => v,
        Err(e) => { 
            error!("/send-challenge/{workflow_id}: fail to send challenge tx: {}", e);
            return HttpResponse::InternalServerError().body(e.to_string())
        }
    };

    user_data.status = sql::STATUS::CHALLENGE as u8;
    user_data.challenge = Some(challenge_txid);

    if let Err(e) = sql::update_user_data(&db, workflow_id, &user_data) {
        error!("/send-challenge/{workflow_id}: fail to update user data: {}", e);
        return HttpResponse::InternalServerError().body(e.to_string())
    }

    if let Err(e) = sql::unlock_workflow(&db, workflow_id) {
        error!("/send-challenge/{workflow_id}: fail to unlock workflow: {}", e);
        return HttpResponse::InternalServerError().body(e.to_string());
    };

    let body = serde_json::to_string_pretty(&ResponseStruct{challenge_txid}).unwrap();
    info!("/send-challenge/{workflow_id}: ok");
    HttpResponse::Ok()
        .content_type(ContentType::json())
        .body(body)
}

#[post("/send-take1/{workflow_id}")]
async fn send_take_1(path: web::Path<i32>) -> impl Responder {
    #[derive(Serialize)]
    struct ResponseStruct {
        take_1_txid: Txid,
    }

    let workflow_id = path.into_inner();
    info!("new REQUEST: /send-take1/{workflow_id}");
    let db = match sql::open_db() {
        Ok(v) => v,
        Err(e) => { 
            error!("/send-take1/{workflow_id}: fail to connect db: {}", e);
            return HttpResponse::InternalServerError().body(e.to_string())
        }
    };

    match sql::is_workflow_locked(&db, workflow_id) {
        Ok(v) => {
            if v {
                error!("/send-take1/{workflow_id}: workflow:{workflow_id} is processing a task, please wait");
                return HttpResponse::Locked().body("workflow:{workflow_id} is processing a task, please wait".to_string())
            };
        },
        Err(e) => {
            error!("/send-take1/{workflow_id}: fail to get workflow lock: {}", e);
            return HttpResponse::InternalServerError().body(e.to_string())
        }
    };

    if let Err(e) = sql::lock_workflow(&db, workflow_id) {
        error!("/send-take1/{workflow_id}: fail to lock workflow: {}", e);
        return HttpResponse::InternalServerError().body(e.to_string());
    };

    let mut user_data = match sql::get_user_data(&db, workflow_id) {
        Ok(user_data_option) => { match user_data_option {
                Some(data) => data,
                _ => {
                    error!("/send-take1/{workflow_id}: workflow {workflow_id} does not exisit");
                    return HttpResponse::BadRequest().body(format!("workflow {workflow_id} does not exisit"))
                }
            }
        },
        Err(e) => { 
            error!("/send-take1/{workflow_id}: fail to get user data: {}", e);
            return HttpResponse::InternalServerError().body(e.to_string())
        }
    };

    if user_data.status != sql::STATUS::KICKOFF2 as u8 {
        error!("/send-take1/{workflow_id}: workflow {workflow_id} not currently at kickoff2 stage");
        return HttpResponse::BadRequest().body(format!("workflow {workflow_id} not currently at kickoff2 stage"))
    };

    let peg_in_txid = match user_data.pegin {
        Some(txid) => txid,
        _ => {
            error!("/send-take1/{workflow_id}: workflow {workflow_id} missing pegin_txid");
            return HttpResponse::InternalServerError().body("workflow {workflow_id} missing pegin_txid".to_string())
        }
    };
    let kick_off_1_txid = match user_data.kickoff_1 {
        Some(txid) => txid,
        _ => {
            error!("/send-take1/{workflow_id}: workflow {workflow_id} missing kickoff1_txid");
            return HttpResponse::InternalServerError().body("workflow {workflow_id} missing kickoff1_txid".to_string())
        }
    };
    let kick_off_2_txid = match user_data.kickoff_2 {
        Some(txid) => txid,
        _ => {
            error!("/send-take1/{workflow_id}: workflow {workflow_id} missing kickoff2_txid");
            return HttpResponse::InternalServerError().body("workflow {workflow_id} missing kickoff2_txid".to_string())
        }
    };

    let user_address = match sql::get_user_address(&db, workflow_id) {
        Ok(user_addr_option) => { match user_addr_option {
                Some(addr) => addr,
                _ => {
                    error!("/send-take1/{workflow_id}: workflow {workflow_id} does not exisit");
                    return HttpResponse::BadRequest().body(format!("workflow {workflow_id} does not exisit"))
                }
            }
        },
        Err(e) => { 
            error!("/send-take1/{workflow_id}: fail to get user address: {}", e);
            return HttpResponse::InternalServerError().body(e.to_string())
        }
    };

    let rpc = match utils::new_rpc_client().await {
        Ok(v) => v,
        Err(e) => { 
            error!("/send-take1/{workflow_id}: fail to connect bitcoind: {}", e);
            return HttpResponse::InternalServerError().body(e.to_string())
        }
    };
    let take_1_txid = match transactions::take_1(&rpc, peg_in_txid, kick_off_1_txid, kick_off_2_txid, user_address).await {
        Ok(v) => v,
        Err(e) => { 
            error!("/send-take1/{workflow_id}: fail to send take1 tx: {}", e);
            return HttpResponse::InternalServerError().body(e.to_string())
        }
    };

    user_data.status = sql::STATUS::TAKE1 as u8;
    user_data.take_1 = Some(take_1_txid);

    if let Err(e) = sql::update_user_data(&db, workflow_id, &user_data) {
        error!("/send-take1/{workflow_id}: fail to update user data: {}", e);
        return HttpResponse::InternalServerError().body(e.to_string())
    }

    if let Err(e) = sql::unlock_workflow(&db, workflow_id) {
        error!("/send-take1/{workflow_id}: fail to unlock workflow: {}", e);
        return HttpResponse::InternalServerError().body(e.to_string());
    };

    let body = serde_json::to_string_pretty(&ResponseStruct{take_1_txid}).unwrap();
    info!("/send-take1/{workflow_id}: ok");
    HttpResponse::Ok()
        .content_type(ContentType::json())
        .body(body)
}

#[post("/send-assert/{workflow_id}")]
async fn send_assert(path: web::Path<i32>) -> impl Responder {
    #[derive(Serialize)]
    struct ResponseStruct {
        assert_txid: Txid,
    }

    let workflow_id = path.into_inner();
    info!("new REQUEST: /send-assert/{workflow_id}");
    let db = match sql::open_db() {
        Ok(v) => v,
        Err(e) => { 
            error!("/send-assert/{workflow_id}: fail to connect db: {}", e);
            return HttpResponse::InternalServerError().body(e.to_string())
        }
    };

    match sql::is_workflow_locked(&db, workflow_id) {
        Ok(v) => {
            if v {
                error!("/send-assert/{workflow_id}: workflow:{workflow_id} is processing a task, please wait");
                return HttpResponse::Locked().body("workflow:{workflow_id} is processing a task, please wait".to_string())
            };
        },
        Err(e) => {
            error!("/send-assert/{workflow_id}: fail to get workflow lock: {}", e);
            return HttpResponse::InternalServerError().body(e.to_string())
        }
    };

    if let Err(e) = sql::lock_workflow(&db, workflow_id) {
        error!("/send-assert/{workflow_id}: fail to lock workflow: {}", e);
        return HttpResponse::InternalServerError().body(e.to_string());
    };

    let user_data = match sql::get_user_data(&db, workflow_id) {
        Ok(user_data_option) => { match user_data_option {
                Some(data) => data,
                _ => {
                    error!("/send-assert/{workflow_id}: workflow {workflow_id} does not exisit");
                    return HttpResponse::BadRequest().body(format!("workflow {workflow_id} does not exisit"))
                }
            }
        },
        Err(e) => { 
            error!("/send-assert/{workflow_id}: fail to get user data: {}", e);
            return HttpResponse::InternalServerError().body(e.to_string())
        }
    };

    if user_data.status != sql::STATUS::CHALLENGE as u8 {
        error!("/send-assert/{workflow_id}: workflow {workflow_id} not currently at challenge stage");
        return HttpResponse::BadRequest().body(format!("workflow {workflow_id} not currently at challenge stage"))
    };

    let _kick_off_2_txid = match user_data.kickoff_2 {
        Some(txid) => txid,
        _ => {
            error!("/send-assert/{workflow_id}: workflow {workflow_id} missing kickoff_2_txid");
            return HttpResponse::InternalServerError().body("workflow {workflow_id} missing kickoff_2_txid".to_string())
        }
    };

    async fn long_task(workflow_id: i32) -> Result<Txid, String> {
        let db = match sql::open_db() {
            Ok(v) => v,
            Err(e) => return Err(e.to_string()),
        };
        let mut user_data = match sql::get_user_data(&db, workflow_id) {
            Ok(user_data_option) => { match user_data_option {
                    Some(data) => data,
                    _ => return Err(format!("workflow {workflow_id} does not exisit")),
                }
            },
            Err(e) => return Err(e.to_string()),
        };
        let rpc = match utils::new_rpc_client().await {
            Ok(v) => v,
            Err(e) => return Err(e.to_string()),
        };
        let kick_off_2_txid = match user_data.kickoff_2 {
            Some(txid) => txid,
            _ => return Err("workflow {workflow_id} missing kickoff_2_txid".to_string()),
        };
        let corrupt_index = user_data.fake_index;
        let connector_c_addr = Some(transactions::get_precomputed_connector_c_address());
        let bitcom_lock_scripts = transactions::borrow_bitcom_lock_scripts();
        let connector_c_tapscripts = transactions::borrow_assert_tapscripts();
        let (assert_txid, _) = match transactions::assert(&rpc, kick_off_2_txid, bitcom_lock_scripts, connector_c_tapscripts, corrupt_index, connector_c_addr).await {
            Ok(v) => v,
            Err(e) => return Err(e.to_string()),
        };

        user_data.status = sql::STATUS::ASSERT as u8;
        user_data.assert = Some(assert_txid);

        if let Err(e) = sql::update_user_data(&db, workflow_id, &user_data) {
            return Err(e.to_string())
        };

        if let Err(e) = sql::unlock_workflow(&db, workflow_id) {
            return Err(e.to_string())
        };

        Ok(assert_txid)
    }

    let task = tokio::spawn(async move {
        long_task(workflow_id).await
    });

    match task.await {
        Ok(res) => match res {
            Ok(assert_txid) => {
                let body = serde_json::to_string_pretty(&ResponseStruct{assert_txid}).unwrap();
                info!("/send-assert/{workflow_id}: ok");
                HttpResponse::Ok()
                    .content_type(ContentType::json())
                    .body(body)
            },
            Err(e) => {
                error!("/send-assert/{workflow_id}: fail to prepare & send assert tx: {}",e);
                HttpResponse::InternalServerError().body(e.to_string())
            }
        },
        Err(e) => {
            error!("/send-assert/{workflow_id}: fail while run send assert task: {}",e);
            HttpResponse::InternalServerError().body(e.to_string())
        }
    }
}

#[post("/send-take2/{workflow_id}")]
async fn send_take_2(path: web::Path<i32>) -> impl Responder {
    #[derive(Serialize)]
    struct ResponseStruct {
        take_2_txid: Txid,
    }

    let workflow_id = path.into_inner();
    info!("new REQUEST: /send-take2/{workflow_id}");
    let db = match sql::open_db() {
        Ok(v) => v,
        Err(e) => { 
            error!("/send-take2/{workflow_id}: fail to connect db: {}", e);
            return HttpResponse::InternalServerError().body(e.to_string())
        }
    };

    match sql::is_workflow_locked(&db, workflow_id) {
        Ok(v) => {
            if v {
                error!("/send-take2/{workflow_id}: workflow:{workflow_id} is processing a task, please wait");
                return HttpResponse::Locked().body("workflow:{workflow_id} is processing a task, please wait".to_string())
            };
        },
        Err(e) => {
            error!("/send-take2/{workflow_id}: fail to get workflow lock: {}", e);
            return HttpResponse::InternalServerError().body(e.to_string())
        }
    };

    if let Err(e) = sql::lock_workflow(&db, workflow_id) {
        error!("/send-take2/{workflow_id}: fail to lock workflow: {}", e);
        return HttpResponse::InternalServerError().body(e.to_string());
    };

    let user_data = match sql::get_user_data(&db, workflow_id) {
        Ok(user_data_option) => { match user_data_option {
                Some(data) => data,
                _ => {
                    error!("/send-take2/{workflow_id}: workflow {workflow_id} does not exisit");
                    return HttpResponse::BadRequest().body(format!("workflow {workflow_id} does not exisit"))
                }
            }
        },
        Err(e) => { 
            error!("/send-take2/{workflow_id}: fail to get user data: {}", e);
            return HttpResponse::InternalServerError().body(e.to_string())
        }
    };

    if user_data.status != sql::STATUS::ASSERT as u8 {
        error!("/send-take2/{workflow_id}: workflow {workflow_id} not currently at assert stage");
        return HttpResponse::BadRequest().body(format!("workflow {workflow_id} not currently at assert stage"))
    };
    let _peg_in_txid = match user_data.pegin {
        Some(txid) => txid,
        _ => {
            error!("/send-take2/{workflow_id}: workflow {workflow_id} missing pegin_txid");
            return HttpResponse::InternalServerError().body("workflow {workflow_id} missing pegin_txid".to_string())
        }
    };
    let _assert_txid = match user_data.assert {
        Some(txid) => txid,
        _ => {
            error!("/send-take2/{workflow_id}: workflow {workflow_id} missing assert_txid");
            return HttpResponse::InternalServerError().body("workflow {workflow_id} missing assert_txid".to_string())
        }
    };

    async fn long_task(workflow_id: i32) -> Result<Txid, String> {
        let db = match sql::open_db() {
            Ok(v) => v,
            Err(e) => return Err(e.to_string()),
        };
        let mut user_data = match sql::get_user_data(&db, workflow_id) {
            Ok(user_data_option) => { match user_data_option {
                    Some(data) => data,
                    _ => return Err(format!("workflow {workflow_id} does not exisit")),
                }
            },
            Err(e) => return Err(e.to_string()),
        };
        let user_address = match sql::get_user_address(&db, workflow_id) {
            Ok(user_addr_option) => { match user_addr_option {
                    Some(addr) => addr,
                    _ => return Err(format!("workflow {workflow_id} does not exisit")),
                }
            },
            Err(e) => return Err(e.to_string()),
        };
        let peg_in_txid = match user_data.pegin {
            Some(txid) => txid,
            _ => return Err("workflow {workflow_id} missing pegin_txid".to_string()),
        };
        let assert_txid = match user_data.assert {
            Some(txid) => txid,
            _ => return Err("workflow {workflow_id} missing assert_txid".to_string()),
        };
    
        let rpc = match utils::new_rpc_client().await {
            Ok(v) => v,
            Err(e) => return Err(e.to_string()),
        };
        let connector_c_addr = Some(transactions::get_precomputed_connector_c_address());
        let connector_c_tapscripts = transactions::borrow_assert_tapscripts();
        let take_2_txid = match transactions::take_2(&rpc, peg_in_txid, assert_txid, connector_c_tapscripts, connector_c_addr, user_address).await {
            Ok(v) => v,
            Err(e) => return Err(e.to_string()),
        };
    
        user_data.status = sql::STATUS::TAKE2 as u8;
        user_data.take_2 = Some(take_2_txid);
    
        if let Err(e) = sql::update_user_data(&db, workflow_id, &user_data) {
            return Err(e.to_string())
        };

        if let Err(e) = sql::unlock_workflow(&db, workflow_id) {
            return Err(e.to_string())
        };

        Ok(take_2_txid)
    }

    let task = tokio::spawn(async move {
        long_task(workflow_id).await
    });

    match task.await {
        Ok(res) => match res {
            Ok(take_2_txid) => {
                let body = serde_json::to_string_pretty(&ResponseStruct{take_2_txid}).unwrap();
                info!("/send-take2/{workflow_id}: ok");
                HttpResponse::Ok()
                    .content_type(ContentType::json())
                    .body(body)
            },
            Err(e) => {
                error!("/send-take2/{workflow_id}: fail to prepare & send take2: {}",e);
                HttpResponse::InternalServerError().body(e.to_string())
            }
        },
        Err(e) => {
            error!("/send-take2/{workflow_id}: fail while run send take2 task: {}",e);
            HttpResponse::InternalServerError().body(e.to_string())
        }
    }
}

#[post("/send-disprove/{workflow_id}")]
async fn send_disprove(path: web::Path<i32>) -> impl Responder {
    #[derive(Serialize)]
    struct ResponseStruct {
        disprove_txid: Txid,
    }

    let workflow_id = path.into_inner();
    info!("new REQUEST: /send-disprove/{workflow_id}");
    let db = match sql::open_db() {
        Ok(v) => v,
        Err(e) => { 
            error!("/send-disprove/{workflow_id}: fail to connect db: {}", e);
            return HttpResponse::InternalServerError().body(e.to_string())
        }
    };

    match sql::is_workflow_locked(&db, workflow_id) {
        Ok(v) => {
            if v {
                error!("/send-disprove/{workflow_id}: workflow:{workflow_id} is processing a task, please wait");
                return HttpResponse::Locked().body("workflow:{workflow_id} is processing a task, please wait".to_string())
            };
        },
        Err(e) => {
            error!("/send-disprove/{workflow_id}: fail to get workflow lock: {}", e);
            return HttpResponse::InternalServerError().body(e.to_string())
        }
    };

    if let Err(e) = sql::lock_workflow(&db, workflow_id) {
        error!("/send-disprove/{workflow_id}: fail to lock workflow: {}", e);
        return HttpResponse::InternalServerError().body(e.to_string());
    };

    let user_data = match sql::get_user_data(&db, workflow_id) {
        Ok(user_data_option) => { match user_data_option {
                Some(data) => data,
                _ => {
                    error!("/send-disprove/{workflow_id}: workflow {workflow_id} does not exisit");
                    return HttpResponse::BadRequest().body(format!("workflow {workflow_id} does not exisit"))
                }
            }
        },
        Err(e) => { 
            error!("/send-disprove/{workflow_id}: fail to get user data: {}", e);
            return HttpResponse::InternalServerError().body(e.to_string())
        }
    };

    if user_data.status != sql::STATUS::ASSERT as u8 {
        error!("/send-disprove/{workflow_id}: workflow {workflow_id} not currently at assert stage");
        return HttpResponse::BadRequest().body(format!("workflow {workflow_id} not currently at assert stage"))
    };
    let _assert_txid = match user_data.assert {
        Some(txid) => txid,
        _ => {
            error!("/send-disprove/{workflow_id}: workflow {workflow_id} missing assert_txid");
            return HttpResponse::InternalServerError().body("workflow {workflow_id} missing assert_txid".to_string())
        }
    };

    async fn long_task(workflow_id: i32) -> Result<Txid, String> {
        let db = match sql::open_db() {
            Ok(v) => v,
            Err(e) => return Err(e.to_string())
        };
        let mut user_data = match sql::get_user_data(&db, workflow_id) {
            Ok(user_data_option) => { match user_data_option {
                    Some(data) => data,
                    _ => return Err(format!("workflow {workflow_id} does not exisit")),
                }
            },
            Err(e) => return Err(e.to_string())
        };
        let assert_txid = match user_data.assert {
            Some(txid) => txid,
            _ => return Err("workflow {workflow_id} missing assert_txid".to_string()),
        };
        let rpc = match utils::new_rpc_client().await {
            Ok(v) => v,
            Err(e) => return Err(e.to_string())
        };
        let connector_c_addr = Some(transactions::get_precomputed_connector_c_address());
        let connector_c_tapscripts = transactions::borrow_assert_tapscripts();
        let fake_index = match user_data.fake_index {
            Some(i) => i as usize,
            _ => return Err("can not disprove valid assert".to_string()),
        };
        let disprove_txid = match transactions::disprove(&rpc, assert_txid, connector_c_tapscripts, connector_c_addr, Some(fake_index)).await {
            Ok(v) => v,
            Err(e) => return Err(e.to_string())
        };
    
        user_data.status = sql::STATUS::DISPROVE as u8;
        user_data.disprove = Some(disprove_txid);
    
        
        if let Err(e) = sql::update_user_data(&db, workflow_id, &user_data) {
            return Err(e.to_string())
        };

        if let Err(e) = sql::unlock_workflow(&db, workflow_id) {
            return Err(e.to_string())
        };

        Ok(disprove_txid)
    }

    let task = tokio::spawn(async move {
        long_task(workflow_id).await
    });

    match task.await {
        Ok(res) => match res {
            Ok(disprove_txid) => {
                let body = serde_json::to_string_pretty(&ResponseStruct{disprove_txid}).unwrap();
                info!("/send-disprove/{workflow_id}: ok");
                HttpResponse::Ok()
                    .content_type(ContentType::json())
                    .body(body)
            },
            Err(e) => {
                error!("/send-disprove/{workflow_id}: fail to prepare & send disprove: {}",e);
                HttpResponse::InternalServerError().body(e.to_string())
            }
        },
        Err(e) => {
            error!("/send-disprove/{workflow_id}: fail while run send disprove task: {}",e);
            HttpResponse::InternalServerError().body(e.to_string())
        }
    }
}
