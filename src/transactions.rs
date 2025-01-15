use bitcoin::{
    Address, Amount, OutPoint, Transaction, Txid, 
    XOnlyPublicKey, absolute, TxIn, TxOut, ScriptBuf, Witness, Sequence
};
use bitcoincore_rpc::Client;
use bitvm::bridge::connectors::connector_0;
use bitvm::bridge::contexts::operator;
use bitvm::bridge::transactions::pre_signed::PreSignedTransaction;
use std::str::FromStr;
use bitvm::treepp::*;
use bitvm::bridge::{
    connectors::{
        revealer::Revealer, connector_c::ConnectorC, connector_0::Connector0, connector::TaprootConnector,
        connector_1::Connector1, connector_2::Connector2, connector_a::ConnectorA,
    }, 
    graphs::base::{DUST_AMOUNT, FEE_AMOUNT},
    scripts::{generate_pay_to_pubkey_script_address, generate_pay_to_pubkey_script}, 
    groth16::{
        load_proof_from_file,
        generate_wots_keys_from_secrets, assert_bitcom_lock,
        load_all_assert_tapscripts_from_file,
        load_all_signed_assertions_from_file,
        corrupt_signed_assertions, validate_assertions,
        assert_unlock_scripts_from_file,
        extract_signed_assertions_from_assert_tx,
        WotsSignatures, WotsPublicKeys, WotsSecretKeys, VerifyingKey,
    },
    transactions::{
        base::{BaseTransaction, Input, InputWithScript}, 
        kick_off_1::KickOff1Transaction, 
        kick_off_2::KickOff2Transaction, 
        peg_in_confirm::PegInConfirmTransaction, 
        peg_in_deposit::PegInDepositTransaction,
        challenge::ChallengeTransaction,
        take_1::Take1Transaction,
        take_2::Take2Transaction,
        assert::AssertTransaction,
        disprove::DisproveTransaction,
    }
};
use crate::utils::{wait, wait_tx};
use crate::{config::{self, network}, utils};
use once_cell::sync::Lazy;
use log::info;

pub static CONNECTOR_C_TAPSCRIPTS: Lazy<Vec<Script>> = Lazy::new(|| {
    info!("load connector_c_tapscripts");
    get_assert_tapscripts()
});
pub static BITCOM_LOCK_SCRIPTS: Lazy<Vec<Script>> = Lazy::new(|| {
    info!("load bitcom_lock_scripts");
    get_bitcom_lock_scripts()
});
pub static BITCOM_UNLOCK_SCRIPTS: Lazy<Vec<Script>> = Lazy::new(|| {
    info!("load bitcom_unlock_scripts");
    get_bitcom_unlock_scripts()
});
pub static REVEALERS_ADDRESS: Lazy<Vec<Address>> = Lazy::new(|| {
    info!("load revealers' address");
    get_revealers_address()
});

pub fn faucet(rpc: &Client, user_addr: &Address) -> Result<(OutPoint, OutPoint), String> {
    let faucet_1_amount = Amount::from_sat(config::PEGIN_AMOUNT);
    let faucet_2_amount = Amount::from_sat(config::KICKOFF_AMOUNT);

    let (faucet_1_outpoint, faucet_2_outpoint) = match utils::generate_stub_outpoint_batch(rpc, &vec![user_addr.clone(); 2], &vec![faucet_1_amount, faucet_2_amount]) {
        Ok(v) => (v[0], v[1]),
        Err(e) => return Err(format!("fail to send the faucet_tx: {}",e))
    };

    Ok((faucet_1_outpoint, faucet_2_outpoint))
}

pub fn peg_in_prepare(rpc: &Client, faucet_1_txid: Txid, faucet_1_vout: u32) -> Result<Transaction, String> {
    let verifier_contexts = config::get_verifier_contexts();
    let input_amount = match utils::get_utxo_value(rpc, faucet_1_txid, faucet_1_vout) {
        Ok(v) => v,
        Err(e) => return Err(format!("fail to get faucet_1_outpoint value: {}", e))
    };
    let input_0 = TxIn {
        previous_output: OutPoint{
            txid: faucet_1_txid,
            vout: faucet_1_vout,
        },
        script_sig: ScriptBuf::new(),
        sequence: Sequence::MAX,
        witness: Witness::default(),
    };
    
    let total_output_amount = input_amount - Amount::from_sat(FEE_AMOUNT);

    let connector_0 = Connector0::new(network(), &verifier_contexts[0].n_of_n_taproot_public_key);

    let output_0 = TxOut {
        value: total_output_amount,
        script_pubkey: connector_0.generate_taproot_address().script_pubkey(),
    };

    Ok(Transaction {
        version: bitcoin::transaction::Version(2),
        lock_time: absolute::LockTime::ZERO,
        input: vec![input_0],
        output: vec![output_0],
    })
}

pub fn kickoff_1_prepare(rpc: &Client, faucet_2_txid: Txid, faucet_2_vout: u32) -> Result<Transaction, String> {
    let operator_context = config::get_operator_context();
    let input_amount = match utils::get_utxo_value(rpc, faucet_2_txid, faucet_2_vout) {
        Ok(v) => v,
        Err(e) => return Err(format!("fail to get faucet_2_outpoint value: {}", e))
    };
    let input_0 = TxIn {
        previous_output: OutPoint{
            txid: faucet_2_txid,
            vout: faucet_2_vout,
        },
        script_sig: ScriptBuf::new(),
        sequence: Sequence::MAX,
        witness: Witness::default(),
    };

    let total_output_amount = input_amount - Amount::from_sat(FEE_AMOUNT);

    let connector_1 = Connector1::new(
        network(),
        &operator_context.operator_taproot_public_key,
        &operator_context.n_of_n_taproot_public_key,
    );
    let connector_a = ConnectorA::new(
        network(),
        &operator_context.operator_taproot_public_key,
        &operator_context.n_of_n_taproot_public_key,
    );
    let connector_2 = Connector2::new(
        network(),
        &operator_context.operator_taproot_public_key,
        &operator_context.n_of_n_taproot_public_key,
    );

    let output_0 = TxOut {
        value: Amount::from_sat(DUST_AMOUNT),
        script_pubkey: connector_a.generate_taproot_address().script_pubkey(),
    };

    let output_1 = TxOut {
        value: total_output_amount - Amount::from_sat(DUST_AMOUNT) * 2,
        script_pubkey: connector_1.generate_taproot_address().script_pubkey(),
    };

    let output_2 = TxOut {
        value: Amount::from_sat(DUST_AMOUNT),
        script_pubkey: connector_2.generate_taproot_address().script_pubkey(),
    };

    Ok(Transaction {
        version: bitcoin::transaction::Version(2),
        lock_time: absolute::LockTime::ZERO,
        input: vec![input_0],
        output: vec![output_0, output_1, output_2],
    })
}

pub fn kick_off_2(rpc: &Client, kick_off_1_txid: Txid, bitcom_lock_scripts: &Vec<Script>) -> Result<Txid, String> {
    let operator_context = config::get_operator_context();
    let connector_1_vout = 1;
    let connector_1_amount = match utils::get_utxo_value(rpc, kick_off_1_txid, connector_1_vout) {
        Ok(v) => v,
        Err(e) => return Err(format!("fail to get connector_a value: {}", e))
    };
    let revealers = get_revealers(&operator_context.n_of_n_taproot_public_key, bitcom_lock_scripts);
    let kick_off_2_tx = KickOff2Transaction::new(
        &operator_context,
        Input {
            outpoint: OutPoint{
                txid: kick_off_1_txid,
                vout: connector_1_vout,
            },
            amount: connector_1_amount,
        },
        revealers,
    );
    let tx = kick_off_2_tx.finalize();
    let kick_off_2_txid = tx.compute_txid();
    
    if let Err(e) = utils::broadcast_tx(rpc, &tx) {
        return Err(format!("fail to broadcast kickoff_2 tx: {}", e))
    };
    if let Err(e) = utils::mint_block(rpc, 1) {
        return Err(format!("fail to mint block: {}", e))
    };
    match utils::validate_tx(rpc, kick_off_1_txid) {
        Ok(valid) => {
            if !valid { 
                return Err(format!("kickoff_2 tx is gone?!"))
            }
        },
        Err(e) => return Err(format!("fail to validate tx: {}", e))
    };

    Ok(kick_off_2_txid)
}

// return: peg_in_txid
pub fn peg_in(rpc: &Client) -> Txid {
    let deposit_input_amount = Amount::from_sat(config::PEGIN_AMOUNT);

    let depositor_context = config::get_depositor_context();
    let verifier_contexts = config::get_verifier_contexts();
    let deposit_funding_utxo_address = generate_pay_to_pubkey_script_address(
        depositor_context.network,
        &depositor_context.depositor_public_key,
    );
    let deposit_funding_outpoint =
        utils::generate_stub_outpoint(rpc, &deposit_funding_utxo_address, deposit_input_amount).unwrap();
    let deposit_input = Input {
        outpoint: deposit_funding_outpoint,
        amount: deposit_input_amount,
    };

    // peg-in deposit
    let peg_in_deposit =
        PegInDepositTransaction::new(&depositor_context, config::DEPOSITOR_EVM_ADDRESS, deposit_input);
    let peg_in_deposit_tx = peg_in_deposit.finalize();
    let deposit_txid = peg_in_deposit_tx.compute_txid();
    utils::broadcast_tx(rpc, &peg_in_deposit_tx).unwrap();
    utils::mint_block(rpc, 1).unwrap();
    utils::validate_tx(rpc, deposit_txid).unwrap();

    // peg-in confirm
    let output_index = 0;
    let confirm_funding_outpoint = OutPoint {
        txid: deposit_txid,
        vout: output_index,
    };
    let confirm_input = Input {
        outpoint: confirm_funding_outpoint,
        amount: peg_in_deposit_tx.output[output_index as usize].value,
    };
    let mut peg_in_confirm =
        PegInConfirmTransaction::new(&depositor_context, config::DEPOSITOR_EVM_ADDRESS, confirm_input);

    let secret_nonces_0 = peg_in_confirm.push_nonces(&verifier_contexts[0]);
    let secret_nonces_1 = peg_in_confirm.push_nonces(&verifier_contexts[1]);

    peg_in_confirm.pre_sign(&verifier_contexts[0], &secret_nonces_0);
    peg_in_confirm.pre_sign(&verifier_contexts[1], &secret_nonces_1);

    let peg_in_confirm_tx = peg_in_confirm.finalize();
    let confirm_txid = peg_in_confirm_tx.compute_txid();
    utils::broadcast_tx(rpc, &peg_in_confirm_tx).unwrap();
    utils::mint_block(rpc, 1).unwrap();
    utils::validate_tx(rpc, confirm_txid).unwrap();
    confirm_txid
}

// return: kickoff_1_txid
pub fn kick_off_1(rpc: &Client) -> Txid {
    let operator_context = config::get_operator_context();
    let kick_off_1_input_amount = Amount::from_sat(config::KICKOFF_AMOUNT);
    let funding_address = generate_pay_to_pubkey_script_address(
        operator_context.network,
        &operator_context.operator_public_key,
    );
    let funding_outpoint = utils::generate_stub_outpoint(rpc, &funding_address, kick_off_1_input_amount).unwrap();
    let input = Input {
        outpoint: funding_outpoint,
        amount: kick_off_1_input_amount,
    };
    let kick_off_1_tx = KickOff1Transaction::new(&operator_context, input);
    let tx = kick_off_1_tx.finalize();
    let kick_off_1_txid = tx.compute_txid();
    utils::broadcast_tx(rpc, &tx).unwrap();
    utils::mint_block(rpc, 1).unwrap();
    utils::validate_tx(rpc, kick_off_1_txid).unwrap();
    kick_off_1_txid
}

// return: (take_1_txid, take_1_tx_weight)
pub fn take_1(rpc: &Client, peg_in_txid: Txid, kick_off_1_txid: Txid, kick_off_2_txid: Txid, receive_address: Address) -> Result<Txid, String> {
    let operator_context = config::get_operator_context();
    let verifier_contexts = config::get_verifier_contexts();

    let connector_0_vout = 0; 
    let connector_0_amount = match utils::get_utxo_value(rpc, peg_in_txid, connector_0_vout) {
        Ok(v) => v,
        Err(e) => return Err(format!("fail to get connector_0 value: {}", e))
    };
    let take_1_input_0 = Input {
        outpoint: OutPoint {
            txid: peg_in_txid,
            vout: connector_0_vout,
        },
        amount: connector_0_amount,
    };
    let connector_a_vout = 0; 
    let connector_a_amount = match utils::get_utxo_value(rpc, kick_off_1_txid, connector_a_vout) {
        Ok(v) => v,
        Err(e) => return Err(format!("fail to get connector_a value: {}", e))
    };
    let take_1_input_1 = Input {
        outpoint: OutPoint {
            txid: kick_off_1_txid,
            vout: connector_a_vout,
        },
        amount: connector_a_amount,
    };
    let connector_3_vout = 0; 
    let connector_3_amount = match utils::get_utxo_value(rpc, kick_off_2_txid, connector_3_vout) {
        Ok(v) => v,
        Err(e) => return Err(format!("fail to get connector_3 value: {}", e))
    };
    let take_1_input_2 = Input {
        outpoint: OutPoint {
            txid: kick_off_2_txid,
            vout: connector_3_vout,
        },
        amount: connector_3_amount,
    };
    let connector_b_vout = 1; 
    let connector_b_amount = match utils::get_utxo_value(rpc, kick_off_2_txid, connector_b_vout) {
        Ok(v) => v,
        Err(e) => return Err(format!("fail to get connector_b value: {}", e))
    };
    let take_1_input_3 = Input {
        outpoint: OutPoint {
            txid: kick_off_2_txid,
            vout: connector_b_vout,
        },
        amount: connector_b_amount,
    };
    let mut take_1_tx = Take1Transaction::new_for_designated_receiver(
        &operator_context,
        take_1_input_0,
        take_1_input_1,
        take_1_input_2,
        take_1_input_3,
        receive_address,
    );

    let secret_nonces_0 = take_1_tx.push_nonces(&verifier_contexts[0]);
    let secret_nonces_1 = take_1_tx.push_nonces(&verifier_contexts[1]);

    take_1_tx.pre_sign(&verifier_contexts[0], &secret_nonces_0);
    take_1_tx.pre_sign(&verifier_contexts[1], &secret_nonces_1);

    let tx = take_1_tx.finalize();
    let take_1_txid = tx.compute_txid();

    if let Err(e) = utils::broadcast_tx(rpc, &tx) {
        return Err(format!("fail to broadcast take_1 tx: {}", e))
    };
    if let Err(e) = utils::mint_block(rpc, 1) {
        return Err(format!("fail to mint block: {}", e))
    };
    match utils::validate_tx(rpc, take_1_txid) {
        Ok(valid) => {
            if !valid { 
                return Err(format!("take_1 tx is gone?!"))
            }
        },
        Err(e) => return Err(format!("fail to validate tx: {}", e))
    };
    Ok(take_1_txid)
}

// return: (challenge_txid, challenge_tx_weight)
pub fn challenge(rpc: &Client, kick_off_1_txid: Txid) -> Result<Txid, String> {
    let depositor_context = config::get_depositor_context();
    let operator_context = config::get_operator_context();
    let connector_a_vout = 0;
    let connector_a_amount = match utils::get_utxo_value(rpc, kick_off_1_txid, connector_a_vout) {
        Ok(v) => v,
        Err(e) => return Err(format!("fail to get connector_a value: {}", e))
    };
    // re-use the depositor private key to imitate a third-party
    let crowdfunding_keypair = &depositor_context.depositor_keypair;
    let crowdfunding_public_key = &depositor_context.depositor_public_key;
    let challenge_amount = Amount::from_sat(config::CHALLENGE_AMOUNT);
    let challenger_address = generate_pay_to_pubkey_script_address(config::network(), crowdfunding_public_key);
    let funding_outpoint = match utils::generate_stub_outpoint(rpc, &challenger_address, challenge_amount) {
        Ok(v) => v,
        Err(e) => return Err(format!("fail to generate Challenger input: {}", e))
    };
    let refund_address = generate_pay_to_pubkey_script_address(network(), crowdfunding_public_key);
    let mut challenge_tx = ChallengeTransaction::new(
        &operator_context,
        Input {
            outpoint: OutPoint{
                txid: kick_off_1_txid,
                vout: connector_a_vout
            },
            amount: connector_a_amount,
        },
        challenge_amount,
    );
    challenge_tx.add_inputs_and_output(
        &depositor_context,
        &vec![
            InputWithScript {
                outpoint: funding_outpoint,
                amount: challenge_amount,
                script: &generate_pay_to_pubkey_script(crowdfunding_public_key),
            },
        ],
        crowdfunding_keypair,
        refund_address.script_pubkey(),
    );
    let tx = challenge_tx.finalize();
    let challenge_txid = tx.compute_txid();
    if let Err(e) = utils::broadcast_tx(rpc, &tx) {
        return Err(format!("fail to broadcast challenge tx: {}", e))
    };
    if let Err(e) = utils::mint_block(rpc, 1) {
        return Err(format!("fail to mint block: {}", e))
    };
    match utils::validate_tx(rpc, challenge_txid) {
        Ok(valid) => {
            if !valid { 
                return Err(format!("challenge tx is gone?!"))
            }
        },
        Err(e) => return Err(format!("fail to validate tx: {}", e))
    };
    Ok(challenge_txid)
}

// return: ((assert_txid, assert_tx_weight), connector_c_address)
pub async fn assert(
    rpc: &Client, 
    kick_off_2_txid: Txid, 
    bitcom_lock_scripts: &Vec<Script>,
    connector_c_tapscripts: &Vec<Script>,
    corrupt_index: Option<u32>,
    connector_c_addr: Option<Address>
) -> Result<(Txid, Address), String> {
    let operator_context = config::get_operator_context();
    let connector_b_vout = 1; 
    let connector_b_amount = get_connector_b_amount();
    // let connector_b_amount = match utils::get_utxo_value(rpc, kick_off_2_txid, connector_b_vout) {
    //     Ok(v) => v,
    //     Err(e) => return Err(format!("fail to get connector_b value: {}", e))
    // };
    let assert_input_0 = Input {
        outpoint: OutPoint {
            txid: kick_off_2_txid,
            vout: connector_b_vout,
        },
        amount: connector_b_amount,
    };
    let mut connector_c = ConnectorC::new(network(), &operator_context.operator_taproot_public_key, &connector_c_tapscripts);
    let connector_c_address = match connector_c_addr{
        Some(addr) => {
            connector_c.import_taproot_address(addr.clone());
            addr
        },
        _ => connector_c.gen_taproot_address()
    };
    
    let revealers = get_revealers(&operator_context.n_of_n_taproot_public_key, bitcom_lock_scripts);
    let bitcom_unlock_scripts = match corrupt_index {
        Some(index) => get_corrupt_bitcom_unlock_scripts(index as usize),
        _ => borrow_bitcom_unlock_scripts().clone(),
    };
    let bitcom_inputs = (0..bitcom_unlock_scripts.len())
        .map(|i| Input{
            outpoint: OutPoint {
                txid: kick_off_2_txid,
                vout: (i+2) as u32,
            },
            amount: Amount::from_sat(DUST_AMOUNT),
        })
        .collect();

    let mut assert_tx = AssertTransaction::new(
        &operator_context, 
        assert_input_0, 
        bitcom_inputs,
        connector_c, 
        revealers
    );
    assert_tx.push_bitcommitments_witness(bitcom_unlock_scripts);
    let tx = assert_tx.finalize();
    let assert_txid = tx.compute_txid();
    let _ = utils::broadcast_tx(rpc, &tx);
    wait_tx().await;
    let _ = utils::mint_block(rpc, 1);
    wait_tx().await;
    let _ = utils::mint_block(rpc, 1);
    match utils::validate_tx(rpc, assert_txid) {
        Ok(valid) => {
            if !valid { 
                return Err(format!("assert tx is gone?!"))
            }
        },
        Err(e) => return Err(format!("fail to validate tx: {}", e))
    };
    Ok((assert_txid, connector_c_address))
}   

// return: take_2_txid
pub fn take_2(
    rpc: &Client, 
    peg_in_txid: Txid, 
    assert_txid: Txid, 
    connector_c_tapscripts: &Vec<Script>,
    connector_c_address: Option<Address>,
    receive_address: Address,
) -> Result<Txid, String> {
    let operator_context = config::get_operator_context();
    let verifier_contexts = config::get_verifier_contexts();

    let connector_0_vout = 0; 
    let connector_0_amount = get_connector_0_amount();
    // let connector_0_amount = match utils::get_utxo_value(rpc, peg_in_txid, connector_0_vout) {
    //     Ok(v) => v,
    //     Err(e) => return Err(format!("fail to get connector_0 value: {}", e))
    // };
    let take_2_input_0 = Input {
        outpoint: OutPoint {
            txid: peg_in_txid,
            vout: connector_0_vout,
        },
        amount: connector_0_amount,
    };

    let connector_4_vout  = 0;
    let connector_4_amount = get_connector_4_amount();
    // let connector_4_amount = match utils::get_utxo_value(rpc, assert_txid, connector_4_vout) {
    //     Ok(v) => v,
    //     Err(e) => return Err(format!("fail to get connector_4 value: {}", e))
    // };
    let take_2_input_1 = Input {
        outpoint: OutPoint {
            txid: assert_txid,
            vout: connector_4_vout,
        },
        amount: connector_4_amount,
    };

    let connector_5_vout  = 1;
    let connector_5_amount = get_connector_5_amount();
    // let connector_5_amount = match utils::get_utxo_value(rpc, assert_txid, connector_5_vout) {
    //     Ok(v) => v,
    //     Err(e) => return Err(format!("fail to get connector_5 value: {}", e))
    // };
    let take_2_input_2 = Input {
        outpoint: OutPoint {
            txid: assert_txid,
            vout: connector_5_vout,
        },
        amount: connector_5_amount,
    };

    let connector_c_vout  = 2;
    let connector_c_amount = get_connector_c_amount();
    // let connector_c_amount = match utils::get_utxo_value(rpc, assert_txid, connector_c_vout) {
    //     Ok(v) => v,
    //     Err(e) => return Err(format!("fail to get connector_c value: {}", e))
    // };
    let take_2_input_3 = Input {
        outpoint: OutPoint {
            txid: assert_txid,
            vout: connector_c_vout,
        },
        amount: connector_c_amount,
    };

    let mut connector_c = ConnectorC::new(network(), &operator_context.operator_taproot_public_key, &connector_c_tapscripts);
    match connector_c_address {
        Some(addr) => connector_c.import_taproot_address(addr),
        _ => { connector_c.gen_taproot_address(); },
    };

    let mut take_2_tx = Take2Transaction::new_for_designated_receiver(
        &operator_context,
        connector_c,
        take_2_input_0,
        take_2_input_1,
        take_2_input_2,
        take_2_input_3,
        receive_address,
    );

    let secret_nonces_0 = take_2_tx.push_nonces(&verifier_contexts[0]);
    let secret_nonces_1 = take_2_tx.push_nonces(&verifier_contexts[1]);

    take_2_tx.pre_sign(&verifier_contexts[0], &secret_nonces_0);
    take_2_tx.pre_sign(&verifier_contexts[1], &secret_nonces_1);

    let tx = take_2_tx.finalize();
    let take_2_txid = tx.compute_txid();
    if let Err(e) = utils::broadcast_tx(rpc, &tx) {
        return Err(format!("fail to broadcast take_2 tx: {}", e))
    };
    if let Err(e) = utils::mint_block(rpc, 1) {
        return Err(format!("fail to mint block: {}", e))
    };
    match utils::validate_tx(rpc, take_2_txid) {
        Ok(valid) => {
            if !valid { 
                return Err(format!("take_2 tx is gone?!"))
            }
        },
        Err(e) => return Err(format!("fail to validate tx: {}", e))
    };
    Ok(take_2_txid)
}

// return (disprove_txid, disprove_tx_weight)
pub fn disprove(
    rpc: &Client, 
    assert_txid: Txid, 
    connector_c_tapscripts: &Vec<Script>,
    connector_c_address: Option<Address>,
    fake_index: Option<usize>,
) -> Result<Txid, String> {
    let operator_context = config::get_operator_context();
    let verifier_contexts = config::get_verifier_contexts();

    let connector_5_vout  = 1;
    let connector_5_amount = get_connector_5_amount();
    // let connector_5_amount = match utils::get_utxo_value(rpc, assert_txid, connector_5_vout) {
    //     Ok(v) => v,
    //     Err(e) => return Err(format!("fail to get connector_5 value: {}", e))
    // };
    let disprove_input_0 = Input {
        outpoint: OutPoint {
            txid: assert_txid,
            vout: connector_5_vout,
        },
        amount: connector_5_amount,
    };

    let connector_c_vout  = 2;
    let connector_c_amount = get_connector_c_amount();
    // let connector_c_amount = match utils::get_utxo_value(rpc, assert_txid, connector_c_vout) {
    //     Ok(v) => v,
    //     Err(e) => return Err(format!("fail to get connector_c value: {}", e))
    // };
    let disprove_input_1 = Input {
        outpoint: OutPoint {
            txid: assert_txid,
            vout: connector_c_vout,
        },
        amount: connector_c_amount,
    };

    let mut connector_c = ConnectorC::new(network(), &operator_context.operator_taproot_public_key, &connector_c_tapscripts);
    match connector_c_address {
        Some(addr) => connector_c.import_taproot_address(addr),
        _ => { connector_c.gen_taproot_address(); },
    };

    let (leaf_index, hint_script) = match validate_assert_bitcom(rpc, assert_txid, fake_index) {
        Ok(res) => match res {
            Some(v) => v,
            _ => return Err(format!("bitcommitments in given assert_tx is completely valid, cannot disprove valid assertions")),
        },
        Err(e) => return Err(format!("fail to evaluate assertions: {}", e))
    };

    let mut disprove_tx = DisproveTransaction::new(
        &operator_context,
        connector_c,
        disprove_input_0,
        disprove_input_1,
        leaf_index as u32,
    );

    let secret_nonces_0 = disprove_tx.push_nonces(&verifier_contexts[0]);
    let secret_nonces_1 = disprove_tx.push_nonces(&verifier_contexts[1]);

    disprove_tx.pre_sign(&verifier_contexts[0], &secret_nonces_0);
    disprove_tx.pre_sign(&verifier_contexts[1], &secret_nonces_1);

    // re-use verifier_0 as challenger
    let challenger_reward_address = generate_pay_to_pubkey_script_address(
        verifier_contexts[0].network,
        &verifier_contexts[0].verifier_public_key,
    );
    let challenger_reward_script = challenger_reward_address.script_pubkey(); 
    disprove_tx.add_input_output(leaf_index as u32, challenger_reward_script, hint_script);

    let tx = disprove_tx.finalize();
    let disprove_txid = tx.compute_txid();
    let _ = utils::broadcast_tx(rpc, &tx);
    let _ = utils::mint_block(rpc, 1);
    match utils::validate_tx(rpc, disprove_txid) {
        Ok(valid) => {
            if !valid { 
                return Err(format!("disprove tx is gone?!"))
            }
        },
        Err(e) => return Err(format!("fail to validate tx: {}", e))
    };
    Ok(disprove_txid)
}


pub fn get_connector_0_amount() -> Amount {
    Amount::from_sat(99980000)
}
pub fn get_connector_4_amount() -> Amount {
    Amount::from_sat(20000)

}
pub fn get_connector_5_amount() -> Amount {
    Amount::from_sat(17690000)
    
}
pub fn get_connector_b_amount() -> Amount {
    Amount::from_sat(18550000)
}
pub fn get_connector_c_amount() -> Amount {
    Amount::from_sat(20000)
}
pub fn get_precomputed_connector_0_address() -> Address {
    Address::from_str(config::CONNECTOR_0_ADDRESS).unwrap().assume_checked()
}
pub fn get_precomputed_connector_4_address() -> Address {
    Address::from_str(config::CONNECTOR_4_ADDRESS).unwrap().assume_checked()
}
pub fn get_precomputed_connector_5_address() -> Address {
    Address::from_str(config::CONNECTOR_5_ADDRESS).unwrap().assume_checked()
}
pub fn get_precomputed_connector_b_address() -> Address {
    Address::from_str(config::CONNECTOR_B_ADDRESS).unwrap().assume_checked()
}
pub fn get_precomputed_connector_c_address() -> Address {
    Address::from_str(config::CONNECTOR_C_ADDRESS).unwrap().assume_checked()
}

pub fn borrow_bitcom_lock_scripts() -> &'static Vec<Script> {
    &BITCOM_LOCK_SCRIPTS
}

pub fn get_bitcom_lock_scripts() -> Vec<Script> {
    let (wots_pk, _) = get_wots_keys();
    assert_bitcom_lock(&wots_pk)
}

pub fn borrow_bitcom_unlock_scripts() -> &'static Vec<Script> {
    &BITCOM_UNLOCK_SCRIPTS
}

pub fn get_bitcom_unlock_scripts() -> Vec<Script> {
    assert_unlock_scripts_from_file(config::WOTS_SIGNATURE_PATH, None, None)
}

pub fn get_corrupt_bitcom_unlock_scripts(corrupt_index: usize) -> Vec<Script> {
    let (_, wots_sk) = get_wots_keys();
    assert_unlock_scripts_from_file(config::WOTS_SIGNATURE_PATH, Some(corrupt_index), Some(wots_sk))
}

pub fn borrow_assert_tapscripts() -> &'static Vec<Script> {
    &CONNECTOR_C_TAPSCRIPTS
}

pub fn get_assert_tapscripts() -> Vec<Script> {
    load_all_assert_tapscripts_from_file(config::TAPSCRIPT_PATH)
}

pub fn get_signed_assertions() -> WotsSignatures {
    load_all_signed_assertions_from_file(config::WOTS_SIGNATURE_PATH)
}   

pub fn corrupt_assertions(signed_assertions: &mut WotsSignatures, index: usize) {
    let (_, wots_sk) = get_wots_keys();
    corrupt_signed_assertions(&wots_sk, signed_assertions, index);
}   

pub fn validate_assert_bitcom(rpc: &Client, assert_txid: Txid, fake_index: Option<usize>) -> Result<Option<(usize, Script)>, String> {
    fn validate(res: &mut Option<(usize, Script)>, vk: &VerifyingKey, signed_asserts: WotsSignatures, inpubkeys: WotsPublicKeys) {
        *res = validate_assertions(&vk, signed_asserts, inpubkeys);
    }

    let signed_assertions = match fake_index {
        Some(index) => {
            let mut ss = get_signed_assertions();
            corrupt_assertions(&mut ss, index);
            ss
        },
        _ => match extract_signed_assertions(rpc, assert_txid) {
            Ok(v) => v,
            Err(e) => return Err(format!("fail to extract_signed_assertions: {}", e))
        },
    };
    let (vk, _, _) = load_proof_from_file(config::PROOF_PATH);
    let (wots_pk, _) = get_wots_keys();
    let mut res = None;
    utils::suppress_output(||{
        validate(&mut res, &vk, signed_assertions, wots_pk);
    });
    Ok(res)
}

pub fn extract_signed_assertions(rpc: &Client, assert_txid: Txid) -> Result<WotsSignatures, String> {
    match utils::get_raw_tx(&rpc, assert_txid) {
        Ok(raw_assert_tx) => Ok(extract_signed_assertions_from_assert_tx(raw_assert_tx)),
        Err(e) => Err(format!("fail to get raw assert tx: {}", e))
    }
}

fn get_wots_keys() -> (WotsPublicKeys, WotsSecretKeys) {
    generate_wots_keys_from_secrets(config::WOTS_SECRET)
}

fn get_revealers<'a>(n_of_n_taproot_public_key: &XOnlyPublicKey, bitcom_lock_scripts: &'a Vec<Script>) -> Vec<Revealer<'a>> {
    let mut revealers = Vec::new();
    for i in 0..bitcom_lock_scripts.len() {
        let revealer = Revealer::new(network(), &n_of_n_taproot_public_key, &bitcom_lock_scripts[i]);
        revealers.push(revealer);
    }
    revealers
}

fn get_revealers_address() -> Vec<Address> {
    let operator_context = config::get_operator_context();
    let revealers = get_revealers(&operator_context.n_of_n_taproot_public_key, borrow_bitcom_lock_scripts());
    revealers.into_iter()
        .map(|r| r.generate_taproot_address())
        .collect()
}
