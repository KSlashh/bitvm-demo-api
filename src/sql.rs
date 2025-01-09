use rusqlite::{params, Connection, Result, OptionalExtension};
use bitcoin::{Txid, Address, address::NetworkUnchecked};
use serde::{Deserialize, Serialize};
use serde_json;
use crate::config;

pub enum STATUS {
    EMPTY = 0,
    FAUCET = 1,
    PEGIN = 2,
    KICKOFF1 = 3,
    KICKOFF2 = 4,
    CHALLENGE = 5,
    TAKE1 = 6,
    ASSERT = 7,
    TAKE2 = 8,
    DISPROVE = 9,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct UserData {
    pub status: u8,
    pub fake_index: Option<u32>,
    pub faucet_1: Option<(Txid, u32)>,
    pub faucet_2: Option<(Txid, u32)>,
    pub pegin: Option<Txid>,
    pub kickoff_1: Option<Txid>,
    pub kickoff_2: Option<Txid>,
    pub challenge: Option<Txid>,
    pub assert: Option<Txid>,
    pub disprove: Option<Txid>,
    pub take_1: Option<Txid>,
    pub take_2: Option<Txid>,
}

pub fn open_db() -> Result<Connection, String> {
    let open_res = Connection::open(config::DB_PATH);
    let db = match open_res {
        Ok(v) => v,
        Err(e) => return Err(format!("fail to open db: {}", e))
    };
    let create_res = db.execute(
        "CREATE TABLE IF NOT EXISTS workflow (
            id      INTEGER PRIMARY KEY,
            addr    TEXT NOT NULL,
            data    TEXT
        )", 
        []);
    match create_res {
        Ok(_) => Ok(db),
        Err(e) => Err(format!("fail to try create table: {}", e))
    }
}

pub fn get_user_id(db: &Connection, addr: &Address) -> Result<Option<i32>, String> {
    let mut stmt = match db.prepare("SELECT MAX(id) FROM workflow WHERE addr = ?1") {
        Ok(v) => v,
        Err(e) => return Err(format!("fail to prepare select: {}", e))
    };
    match stmt.query_row(params![serde_json::to_string(&addr).unwrap()], |row| row.get(0)) {
        Ok(v) => Ok(v),
        Err(e) => return Err(format!("fail to query db: {}", e))
    }
}

pub fn get_user_data(db: &Connection, id: i32) -> Result<Option<UserData>, String> {
    let mut stmt = match db.prepare("SELECT data FROM workflow WHERE id = ?1") {
        Ok(v) => v,
        Err(e) => return Err(format!("fail to prepare select: {}", e))
    };
    let user_data_str: String = match stmt.query_row(params![id], |row| row.get(0)).optional() {
        Ok(v) => match v {
            Some(s) => s,
            _ => return Ok(None)
        },
        Err(e) => return Err(format!("fail to query db: {}", e))
    };
    Ok(Some(serde_json::from_str(&user_data_str).unwrap()))
}

pub fn get_user_address(db: &Connection, id: i32) -> Result<Option<Address>, String> {
    let mut stmt = match db.prepare("SELECT addr FROM workflow WHERE id = ?1") {
        Ok(v) => v,
        Err(e) => return Err(format!("fail to prepare select: {}", e))
    };
    let user_addr_str: String = match stmt.query_row(params![id], |row| row.get(0)).optional() {
        Ok(v) => match v {
            Some(s) => s,
            _ => return Ok(None)
        },
        Err(e) => return Err(format!("fail to query db: {}", e))
    };
    Ok(Some(serde_json::from_str::<Address<NetworkUnchecked>>(&user_addr_str).unwrap().assume_checked()))
}

pub fn new_user(db: &Connection, addr: &Address) -> Result<i32, String> {
    let user_data = UserData {
        status: STATUS::EMPTY as u8,
        fake_index: None,
        faucet_1: None,
        faucet_2: None,
        pegin: None,
        kickoff_1: None,
        kickoff_2: None,
        challenge: None,
        assert: None,
        disprove: None,
        take_1: None,
        take_2: None,
    };
    let insert_res = db.execute(
        "INSERT INTO workflow (addr, data) values (?1, ?2)", 
        [
            serde_json::to_string(&addr).unwrap(),
            serde_json::to_string(&user_data).unwrap()
        ]);
    match insert_res {
        Ok(_) => {
            match get_user_id(db, addr) {
                Ok(v) => Ok(v.unwrap()),
                Err(e) => Err(e),
            }
        },
        Err(e) => Err(e.to_string()),
    }
}

pub fn update_user_data(db: &Connection, id: i32, data: &UserData) -> Result<bool, String> {
    match db.execute("UPDATE workflow SET data = ?1 WHERE id = ?2", params![serde_json::to_string(&data).unwrap(), id]) {
        Ok(_) => Ok(true),
        Err(e) => Err(format!("fail to update data: {}", e))
    }
}