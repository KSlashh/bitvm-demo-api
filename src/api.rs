use actix_web::{get, post, web,  http::header::ContentType, HttpResponse, HttpServer, Responder};
use bitcoin::{OutPoint, Txid, Address, Amount};
use bitvm::bridge::transactions::{kick_off_1, peg_in_refund};
use rusqlite::Connection;
use serde::Serialize;
use crate::{sql::{self, UserData}, transactions, utils};

#[derive(Serialize)]
struct TxInput {
    txid: Txid,
    vout: u32,
    value: Amount,
}
#[derive(Serialize)]
struct TxOutput {
    testnet_address: Address,
    regtest_address: Address,
    value: Amount,
}

#[post("/get-user-workflow/{user_address}")]
async fn get_user_workflow(path: web::Path<String>) -> impl Responder {
    #[derive(Serialize)]
    struct ResponseStruct {
        workflow_id: i32,
        workflow: sql::UserData,
    }

    let user_addr = path.into_inner();
    let user_addr = match utils::address_from_str(&user_addr) {
        Ok(v) => v,
        Err(e) => return HttpResponse::BadRequest().body(e.to_string())
    };
    let db = match sql::open_db() {
        Ok(v) => v,
        Err(e) => return HttpResponse::InternalServerError().body(e.to_string()),
    };
    let workflow_id = match sql::get_user_id(&db, &user_addr) {
        Ok(id_option) => match id_option {
            Some(id) => id,
            _ => {
                if let Err(e) = sql::new_user(&db, &user_addr) {
                    return HttpResponse::InternalServerError().body(e.to_string())
                };
                match sql::get_user_id(&db, &user_addr) {
                    Ok(id) => id.unwrap(),
                    Err(e) => return HttpResponse::InternalServerError().body(e.to_string()),
                }
            }
        },
        Err(e) => return HttpResponse::InternalServerError().body(e.to_string()),
    };
    let workflow = match sql::get_user_data(&db, workflow_id) {
        Ok(user_data_option) => { match user_data_option {
                Some(data) => data,
                _ => return HttpResponse::BadRequest().body(format!("workflow {workflow_id} does not exisit")),
            }
        },
        Err(e) => return HttpResponse::InternalServerError().body(e.to_string()),
    };

    let body = serde_json::to_string_pretty(&ResponseStruct{workflow_id,workflow}).unwrap();
    HttpResponse::Ok()
        .content_type(ContentType::json())
        .body(body)
}

#[get("/get-workflow-info/{workflow_id}")]
async fn get_workflow_info(path: web::Path<i32>) -> impl Responder {
    // type ResponseStruct = sql::UserData;

    let workflow_id = path.into_inner();
    let db = match sql::open_db() {
        Ok(v) => v,
        Err(e) => return HttpResponse::InternalServerError().body(e.to_string()),
    };

    let user_data = match sql::get_user_data(&db, workflow_id) {
        Ok(user_data_option) => { match user_data_option {
                Some(data) => data,
                _ => return HttpResponse::BadRequest().body(format!("workflow {workflow_id} does not exisit")),
            }
        },
        Err(e) => return HttpResponse::InternalServerError().body(e.to_string()),
    };

    let body = serde_json::to_string_pretty(&user_data).unwrap();
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
    let user_addr = match utils::address_from_str(&user_addr) {
        Ok(v) => v,
        Err(e) => return HttpResponse::BadRequest().body(e.to_string())
    };

    let db = match sql::open_db() {
        Ok(v) => v,
        Err(e) => return HttpResponse::InternalServerError().body(e.to_string()),
    };

    let (workflow_id, mut user_data) = match sql::get_user_id(&db, &user_addr) {
        Ok(id_option) => {
            match id_option {
                Some(workflow_id) => {
                    let user_data = match sql::get_user_data(&db, workflow_id) {
                        Ok(user_data_option) => { match user_data_option {
                                Some(data) => data,
                                _ => return HttpResponse::BadRequest().body(format!("workflow {workflow_id} does not exisit")),
                            }
                        },
                        Err(e) => return HttpResponse::InternalServerError().body(e.to_string()),
                    };
                    if user_data.status != sql::STATUS::EMPTY as u8 {
                        match create_new_user(&db, &user_addr) {
                            Ok(v) => v,
                            Err(e) => return HttpResponse::InternalServerError().body(e.to_string()),
                        }
                    } else {
                        (workflow_id, user_data)
                    }
                },
                _ =>  {
                    match create_new_user(&db, &user_addr) {
                        Ok(v) => v,
                        Err(e) => return HttpResponse::InternalServerError().body(e.to_string()),
                    }
                },          
            }
        },
        Err(e) => return HttpResponse::InternalServerError().body(e.to_string()),
    };

    let rpc = match utils::new_rpc_client().await {
        Ok(v) => v,
        Err(e) => return HttpResponse::InternalServerError().body(e.to_string()),
    };
    let (faucet_outpoint_1, faucet_outpoint_2) = match transactions::faucet(&rpc, &user_addr) {
        Ok(v) => v,
        Err(e) => return HttpResponse::InternalServerError().body(e.to_string()),
    };

    user_data.status = sql::STATUS::FAUCET as u8;
    user_data.faucet_1 = Some((faucet_outpoint_1.txid, faucet_outpoint_1.vout));
    user_data.faucet_2 = Some((faucet_outpoint_2.txid, faucet_outpoint_2.vout));

    if let Err(e) = sql::update_user_data(&db, workflow_id, &user_data) {
        return HttpResponse::InternalServerError().body(e.to_string())
    }

    let faucet_txid = faucet_outpoint_1.txid;
    let body = serde_json::to_string_pretty(&ResponseStruct{workflow_id, faucet_txid}).unwrap();
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

    let db = match sql::open_db() {
        Ok(v) => v,
        Err(e) => return HttpResponse::InternalServerError().body(e.to_string()),
    };

    let user_data = match sql::get_user_data(&db, workflow_id) {
        Ok(user_data_option) => { match user_data_option {
                Some(data) => data,
                _ => return HttpResponse::BadRequest().body(format!("workflow {workflow_id} does not exisit")),
            }
        },
        Err(e) => return HttpResponse::InternalServerError().body(e.to_string()),
    };

    if user_data.status != sql::STATUS::FAUCET as u8 {
        return HttpResponse::BadRequest().body(format!("workflow {workflow_id} not currently at faucet stage"))
    };

    let (faucet_1_txid, faucet_1_vout) = match user_data.faucet_1 {
        Some(v) => v,
        _ => return HttpResponse::InternalServerError().body("workflow {workflow_id} missing faucet_1_txid".to_string()),
    };

    let rpc = match utils::new_rpc_client().await {
        Ok(v) => v,
        Err(e) => return HttpResponse::InternalServerError().body(e.to_string()),
    };

    let pegin_tx = match transactions::peg_in_prepare(&rpc, faucet_1_txid, faucet_1_vout) {
        Ok(v) => v,
        Err(e) => return HttpResponse::InternalServerError().body(e.to_string()),
    };

    let txid = pegin_tx.input[0].previous_output.txid;
    let vout = pegin_tx.input[0].previous_output.vout;
    let value = match utils::get_utxo_value(&rpc, txid, vout) {
        Ok(v) => v,
        Err(e) => return HttpResponse::InternalServerError().body(e.to_string()),
    };
    let input = TxInput {
        txid,
        vout,
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
    let pegin_txid = match utils::txid_from_str(&pegin_txid) {
        Ok(v) => v,
        Err(e) => return HttpResponse::BadRequest().body(e.to_string()),
    };

    let db = match sql::open_db() {
        Ok(v) => v,
        Err(e) => return HttpResponse::InternalServerError().body(e.to_string()),
    };

    let mut user_data = match sql::get_user_data(&db, workflow_id) {
        Ok(user_data_option) => { match user_data_option {
                Some(data) => data,
                _ => return HttpResponse::BadRequest().body(format!("workflow {workflow_id} does not exisit")),
            }
        },
        Err(e) => return HttpResponse::InternalServerError().body(e.to_string()),
    };

    if user_data.status != sql::STATUS::FAUCET as u8 {
        return HttpResponse::BadRequest().body(format!("workflow {workflow_id} not currently at faucet stage"))
    };

    user_data.status = sql::STATUS::PEGIN as u8;
    user_data.pegin = Some(pegin_txid);

    if let Err(e) = sql::update_user_data(&db, workflow_id, &user_data) {
        return HttpResponse::InternalServerError().body(e.to_string())
    }

    let body = serde_json::to_string_pretty(&ResponseStruct{success: true}).unwrap();
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
    let db = match sql::open_db() {
        Ok(v) => v,
        Err(e) => return HttpResponse::InternalServerError().body(e.to_string()),
    };

    let mut user_data = match sql::get_user_data(&db, workflow_id) {
        Ok(user_data_option) => { match user_data_option {
                Some(data) => data,
                _ => return HttpResponse::BadRequest().body(format!("workflow {workflow_id} does not exisit")),
            }
        },
        Err(e) => return HttpResponse::InternalServerError().body(e.to_string()),
    };

    if user_data.status != sql::STATUS::PEGIN as u8 {
        return HttpResponse::BadRequest().body(format!("workflow {workflow_id} not currently at pegin stage"))
    };

    let fake_index = if fake_index > 614 { 1 } else {fake_index };
    user_data.fake_index = Some(fake_index);

    if let Err(e) = sql::update_user_data(&db, workflow_id, &user_data) {
        return HttpResponse::InternalServerError().body(e.to_string())
    }

    let body = serde_json::to_string_pretty(&ResponseStruct{success:true}).unwrap();
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

    let db = match sql::open_db() {
        Ok(v) => v,
        Err(e) => return HttpResponse::InternalServerError().body(e.to_string()),
    };

    let user_data = match sql::get_user_data(&db, workflow_id) {
        Ok(user_data_option) => { match user_data_option {
                Some(data) => data,
                _ => return HttpResponse::BadRequest().body(format!("workflow {workflow_id} does not exisit")),
            }
        },
        Err(e) => return HttpResponse::InternalServerError().body(e.to_string()),
    };

    if user_data.status != sql::STATUS::PEGIN as u8 {
        return HttpResponse::BadRequest().body(format!("workflow {workflow_id} not currently at pegin stage"))
    };

    let (faucet_2_txid, faucet_2_vout) = match user_data.faucet_2 {
        Some(v) => v,
        _ => return HttpResponse::InternalServerError().body("workflow {workflow_id} missing faucet_2_txid".to_string()),
    };

    let rpc = match utils::new_rpc_client().await {
        Ok(v) => v,
        Err(e) => return HttpResponse::InternalServerError().body(e.to_string()),
    };

    let kickoff_1_tx = match transactions::kickoff_1_prepare(&rpc, faucet_2_txid, faucet_2_vout) {
        Ok(v) => v,
        Err(e) => return HttpResponse::InternalServerError().body(e.to_string()),
    };

    let txid = kickoff_1_tx.input[0].previous_output.txid;
    let vout = kickoff_1_tx.input[0].previous_output.vout;
    let value = match utils::get_utxo_value(&rpc, txid, vout) {
        Ok(v) => v,
        Err(e) => return HttpResponse::InternalServerError().body(e.to_string()),
    };
    let input = TxInput {
        txid,
        vout,
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
    let kick_off_1_txid = match utils::txid_from_str(&kickoff_1_txid) {
        Ok(v) => v,
        Err(e) => return HttpResponse::BadRequest().body(e.to_string()),
    };

    let db = match sql::open_db() {
        Ok(v) => v,
        Err(e) => return HttpResponse::InternalServerError().body(e.to_string()),
    };

    let mut user_data = match sql::get_user_data(&db, workflow_id) {
        Ok(user_data_option) => { match user_data_option {
                Some(data) => data,
                _ => return HttpResponse::BadRequest().body(format!("workflow {workflow_id} does not exisit")),
            }
        },
        Err(e) => return HttpResponse::InternalServerError().body(e.to_string()),
    };

    if user_data.status == sql::STATUS::PEGIN as u8 {
        user_data.status = sql::STATUS::KICKOFF1 as u8;
    } else if user_data.status != sql::STATUS::KICKOFF1 as u8 {
        return HttpResponse::BadRequest().body(format!("workflow {workflow_id} not currently at pegin/kickoff1 stage"))
    };

    user_data.kickoff_1 = Some(kick_off_1_txid);

    if let Err(e) = sql::update_user_data(&db, workflow_id, &user_data) {
        return HttpResponse::InternalServerError().body(e.to_string())
    }

    let rpc = match utils::new_rpc_client().await {
        Ok(v) => v,
        Err(e) => return HttpResponse::InternalServerError().body(e.to_string()),
    };
    let bitcom_lock_scripts = transactions::get_bitcom_lock_scripts();
    let kick_off_2_txid = match transactions::kick_off_2(&rpc, kick_off_1_txid, &bitcom_lock_scripts) {
        Ok(v) => v,
        Err(e) => return HttpResponse::InternalServerError().body(e.to_string()),
    };

    user_data.status = sql::STATUS::KICKOFF2 as u8;
    user_data.kickoff_2 = Some(kick_off_2_txid);

    if let Err(e) = sql::update_user_data(&db, workflow_id, &user_data) {
        return HttpResponse::InternalServerError().body(e.to_string())
    }

    let body = serde_json::to_string_pretty(&ResponseStruct{kick_off_2_txid}).unwrap();
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
    let db = match sql::open_db() {
        Ok(v) => v,
        Err(e) => return HttpResponse::InternalServerError().body(e.to_string()),
    };

    let mut user_data = match sql::get_user_data(&db, workflow_id) {
        Ok(user_data_option) => { match user_data_option {
                Some(data) => data,
                _ => return HttpResponse::BadRequest().body(format!("workflow {workflow_id} does not exisit")),
            }
        },
        Err(e) => return HttpResponse::InternalServerError().body(e.to_string()),
    };

    if user_data.status != sql::STATUS::KICKOFF2 as u8 {
        return HttpResponse::BadRequest().body(format!("workflow {workflow_id} not currently at kickoff2 stage"))
    };

    let kick_off_1_txid = match user_data.kickoff_1 {
        Some(txid) => txid,
        _ => return HttpResponse::InternalServerError().body("workflow {workflow_id} missing kickoff_1_txid".to_string()),
    };

    let rpc = match utils::new_rpc_client().await {
        Ok(v) => v,
        Err(e) => return HttpResponse::InternalServerError().body(e.to_string()),
    };
    let challenge_txid = match transactions::challenge(&rpc, kick_off_1_txid) {
        Ok(v) => v,
        Err(e) => return HttpResponse::InternalServerError().body(e.to_string()),
    };

    user_data.status = sql::STATUS::CHALLENGE as u8;
    user_data.challenge = Some(challenge_txid);

    if let Err(e) = sql::update_user_data(&db, workflow_id, &user_data) {
        return HttpResponse::InternalServerError().body(e.to_string())
    }

    let body = serde_json::to_string_pretty(&ResponseStruct{challenge_txid}).unwrap();
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
    let db = match sql::open_db() {
        Ok(v) => v,
        Err(e) => return HttpResponse::InternalServerError().body(e.to_string()),
    };

    let mut user_data = match sql::get_user_data(&db, workflow_id) {
        Ok(user_data_option) => { match user_data_option {
                Some(data) => data,
                _ => return HttpResponse::BadRequest().body(format!("workflow {workflow_id} does not exisit")),
            }
        },
        Err(e) => return HttpResponse::InternalServerError().body(e.to_string()),
    };

    if user_data.status != sql::STATUS::KICKOFF2 as u8 {
        return HttpResponse::BadRequest().body(format!("workflow {workflow_id} not currently at kickoff2 stage"))
    };

    let peg_in_txid = match user_data.pegin {
        Some(txid) => txid,
        _ => return HttpResponse::InternalServerError().body("workflow {workflow_id} missing pegin_txid".to_string()),
    };
    let kick_off_1_txid = match user_data.kickoff_1 {
        Some(txid) => txid,
        _ => return HttpResponse::InternalServerError().body("workflow {workflow_id} missing kickoff_1_txid".to_string()),
    };
    let kick_off_2_txid = match user_data.kickoff_2 {
        Some(txid) => txid,
        _ => return HttpResponse::InternalServerError().body("workflow {workflow_id} missing kickoff_2_txid".to_string()),
    };

    let user_address = match sql::get_user_address(&db, workflow_id) {
        Ok(user_addr_option) => { match user_addr_option {
                Some(addr) => addr,
                _ => return HttpResponse::BadRequest().body(format!("workflow {workflow_id} does not exisit")),
            }
        },
        Err(e) => return HttpResponse::InternalServerError().body(e.to_string()),
    };

    let rpc = match utils::new_rpc_client().await {
        Ok(v) => v,
        Err(e) => return HttpResponse::InternalServerError().body(e.to_string()),
    };
    let take_1_txid = match transactions::take_1(&rpc, peg_in_txid, kick_off_1_txid, kick_off_2_txid, user_address) {
        Ok(v) => v,
        Err(e) => return HttpResponse::InternalServerError().body(e.to_string()),
    };

    user_data.status = sql::STATUS::TAKE1 as u8;
    user_data.take_1 = Some(take_1_txid);

    if let Err(e) = sql::update_user_data(&db, workflow_id, &user_data) {
        return HttpResponse::InternalServerError().body(e.to_string())
    }

    let body = serde_json::to_string_pretty(&ResponseStruct{take_1_txid}).unwrap();
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
    let db = match sql::open_db() {
        Ok(v) => v,
        Err(e) => return HttpResponse::InternalServerError().body(e.to_string()),
    };

    let user_data = match sql::get_user_data(&db, workflow_id) {
        Ok(user_data_option) => { match user_data_option {
                Some(data) => data,
                _ => return HttpResponse::BadRequest().body(format!("workflow {workflow_id} does not exisit")),
            }
        },
        Err(e) => return HttpResponse::InternalServerError().body(e.to_string()),
    };

    if user_data.status != sql::STATUS::CHALLENGE as u8 {
        return HttpResponse::BadRequest().body(format!("workflow {workflow_id} not currently at challenge stage"))
    };

    let _kick_off_2_txid = match user_data.kickoff_2 {
        Some(txid) => txid,
        _ => return HttpResponse::InternalServerError().body("workflow {workflow_id} missing kickoff_2_txid".to_string()),
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
        let bitcom_lock_scripts = transactions::get_bitcom_lock_scripts();
        let connector_c_tapscripts = transactions::get_assert_tapscripts();
        let (assert_txid, _) = match transactions::assert(&rpc, kick_off_2_txid, &bitcom_lock_scripts, &connector_c_tapscripts, corrupt_index, connector_c_addr) {
            Ok(v) => v,
            Err(e) => return Err(e.to_string()),
        };

        user_data.status = sql::STATUS::ASSERT as u8;
        user_data.assert = Some(assert_txid);

        match sql::update_user_data(&db, workflow_id, &user_data) {
            Ok(_) => Ok(assert_txid),
            Err(e) => return Err(e.to_string())
        }
    }


    let task = tokio::spawn(async move {
        long_task(workflow_id).await
    });

    match task.await {
        Ok(res) => match res {
            Ok(assert_txid) => {
                let body = serde_json::to_string_pretty(&ResponseStruct{assert_txid}).unwrap();
                HttpResponse::Ok()
                    .content_type(ContentType::json())
                    .body(body)
            },
            Err(e) => {
                HttpResponse::InternalServerError().body(e.to_string())
            }
        },
        Err(e) => {
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
    let db = match sql::open_db() {
        Ok(v) => v,
        Err(e) => return HttpResponse::InternalServerError().body(e.to_string()),
    };

    let user_data = match sql::get_user_data(&db, workflow_id) {
        Ok(user_data_option) => { match user_data_option {
                Some(data) => data,
                _ => return HttpResponse::BadRequest().body(format!("workflow {workflow_id} does not exisit")),
            }
        },
        Err(e) => return HttpResponse::InternalServerError().body(e.to_string()),
    };

    if user_data.status != sql::STATUS::ASSERT as u8 {
        return HttpResponse::BadRequest().body(format!("workflow {workflow_id} not currently at assert stage"))
    };
    let _peg_in_txid = match user_data.pegin {
        Some(txid) => txid,
        _ => return HttpResponse::InternalServerError().body("workflow {workflow_id} missing pegin_txid".to_string()),
    };
    let _assert_txid = match user_data.assert {
        Some(txid) => txid,
        _ => return HttpResponse::InternalServerError().body("workflow {workflow_id} missing assert_txid".to_string()),
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
        let connector_c_tapscripts = transactions::get_assert_tapscripts();
        let take_2_txid = match transactions::take_2(&rpc, peg_in_txid, assert_txid, &connector_c_tapscripts, connector_c_addr, user_address) {
            Ok(v) => v,
            Err(e) => return Err(e.to_string()),
        };
    
        user_data.status = sql::STATUS::TAKE2 as u8;
        user_data.take_2 = Some(take_2_txid);
    
        match sql::update_user_data(&db, workflow_id, &user_data) {
            Ok(_) => Ok(take_2_txid),
            Err(e) => return Err(e.to_string())
        }
    }

    let task = tokio::spawn(async move {
        long_task(workflow_id).await
    });

    match task.await {
        Ok(res) => match res {
            Ok(take_2_txid) => {
                let body = serde_json::to_string_pretty(&ResponseStruct{take_2_txid}).unwrap();
                HttpResponse::Ok()
                    .content_type(ContentType::json())
                    .body(body)
            },
            Err(e) => {
                HttpResponse::InternalServerError().body(e.to_string())
            }
        },
        Err(e) => {
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
    let db = match sql::open_db() {
        Ok(v) => v,
        Err(e) => return HttpResponse::InternalServerError().body(e.to_string()),
    };

    let user_data = match sql::get_user_data(&db, workflow_id) {
        Ok(user_data_option) => { match user_data_option {
                Some(data) => data,
                _ => return HttpResponse::BadRequest().body(format!("workflow {workflow_id} does not exisit")),
            }
        },
        Err(e) => return HttpResponse::InternalServerError().body(e.to_string()),
    };

    if user_data.status != sql::STATUS::ASSERT as u8 {
        return HttpResponse::BadRequest().body(format!("workflow {workflow_id} not currently at assert stage"))
    };
    let _assert_txid = match user_data.assert {
        Some(txid) => txid,
        _ => return HttpResponse::InternalServerError().body("workflow {workflow_id} missing assert_txid".to_string()),
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
        let connector_c_tapscripts = transactions::get_assert_tapscripts();
        let disprove_txid = match transactions::disprove(&rpc, assert_txid, &connector_c_tapscripts, connector_c_addr) {
            Ok(v) => v,
            Err(e) => return Err(e.to_string())
        };
    
        user_data.status = sql::STATUS::DISPROVE as u8;
        user_data.disprove = Some(disprove_txid);
    
        match sql::update_user_data(&db, workflow_id, &user_data) {
            Ok(_) => Ok(disprove_txid),
            Err(e) => return Err(e.to_string())
        }
    }

    let task = tokio::spawn(async move {
        long_task(workflow_id).await
    });

    match task.await {
        Ok(res) => match res {
            Ok(disprove_txid) => {
                let body = serde_json::to_string_pretty(&ResponseStruct{disprove_txid}).unwrap();
                HttpResponse::Ok()
                    .content_type(ContentType::json())
                    .body(body)
            },
            Err(e) => {
                HttpResponse::InternalServerError().body(e.to_string())
            }
        },
        Err(e) => {
            HttpResponse::InternalServerError().body(e.to_string())
        }
    }
}
