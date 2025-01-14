use bitcoin::{Network, PublicKey};
use bitvm::{bridge::graphs::base, groth16::g16};
use bitvm::bridge::contexts::{
    base::generate_keys_from_secret,
    depositor::DepositorContext,
    verifier::VerifierContext,
    operator::OperatorContext,
};

pub const DB_PATH: &str = "data-dir/userdata.db";
pub const BIND_IP: &str = "0.0.0.0";
pub const BIND_PORT: u16 = 7080;

pub const RPCUSER: &str = "test";
pub const RPCPASSWORD: &str = "test";
pub const RPC_URL: &str = "http://127.0.0.1:18443/wallet/public-test";

pub const PROOF_PATH: &str = "data-dir/dummy_proof.json";
pub const COMPILE_PATH: &str = "data-dir/compile";
pub const TAPSCRIPT_PATH: &str = "data-dir/tapscripts";
pub const TAPSCRIPT_FILE_PATH: &str = "data-dir/tapscripts.json";
pub const WOTS_SIGNATURE_PATH: &str = "data-dir/signed_assertions";
pub const WOTS_SIGNATURE_FILE_PATH: &str = "data-dir/signed_assertions.json";
pub const WOTS_SECRET: &str = "a138982ce17ac813d505a5b40b665d404e9528e7"; // just for test
pub const N_TAPLEAVES: usize = g16::N_TAPLEAVES;
pub const N_ASSERTIONS: usize = g16::N_VERIFIER_PUBLIC_INPUTS + g16::N_VERIFIER_FQS + g16::N_VERIFIER_HASHES;

pub const TX_WAIT_TIME: u64 = 5; // in seconds
pub const PEGIN_AMOUNT: u64 = 100_000_000;
pub const KICKOFF_AMOUNT: u64 = 20_000_000;
pub const CHALLENGE_AMOUNT: u64 = 10_000_000;
pub const OPERATOR_SECRET: &str = base::OPERATOR_SECRET;
pub const VERIFIER_0_SECRET: &str = base::VERIFIER_0_SECRET;
pub const VERIFIER_1_SECRET: &str = base::VERIFIER_1_SECRET;
pub const DEPOSITOR_SECRET: &str = base::DEPOSITOR_SECRET;
pub const WITHDRAWER_SECRET: &str = base::WITHDRAWER_SECRET;
pub const DEPOSITOR_EVM_ADDRESS: &str = base::DEPOSITOR_EVM_ADDRESS;
pub const WITHDRAWER_EVM_ADDRESS: &str = base::WITHDRAWER_EVM_ADDRESS;


pub const CONNECTOR_0_ADDRESS: &str = "bcrt1pevdd404wz9tn9q9mv2m2qqwkr7ggk9gr42aslxl8khs0fv8nh98qj9lqh5";
pub const CONNECTOR_4_ADDRESS: &str = "bcrt1qn0fq79zuqhgysahj29u7kghhmj6fukwmf5mgcxt5x86se03s297q8r3lmf";
pub const CONNECTOR_5_ADDRESS: &str = "bcrt1pevdd404wz9tn9q9mv2m2qqwkr7ggk9gr42aslxl8khs0fv8nh98qj9lqh5";
pub const CONNECTOR_B_ADDRESS: &str = "bcrt1pscls7gz4p7f78yq2h3pxv03v7tnqj585szy3gcenzu7gfexn6q0sjndqe7";
pub const CONNECTOR_C_ADDRESS: &str = "bcrt1p2ftjy8xzyy49lp7c4qdrqmzu0tn6scn5h2x7tw3309uf2lmquzqsrcnhrn";

pub fn network() -> Network {
    Network::Regtest
}

pub fn get_depositor_context() -> DepositorContext {
    let (_, _, verifier_0_public_key) =
        generate_keys_from_secret(network(), VERIFIER_0_SECRET);
    let (_, _, verifier_1_public_key) =
        generate_keys_from_secret(network(), VERIFIER_1_SECRET);
    let mut n_of_n_public_keys: Vec<PublicKey> = Vec::new();
    n_of_n_public_keys.push(verifier_0_public_key);
    n_of_n_public_keys.push(verifier_1_public_key);
    DepositorContext::new(network(), DEPOSITOR_SECRET, &n_of_n_public_keys)
}

pub fn get_verifier_contexts() -> [VerifierContext; 2] {
    let (_, _, verifier_0_public_key) =
        generate_keys_from_secret(network(), VERIFIER_0_SECRET);
    let (_, _, verifier_1_public_key) =
        generate_keys_from_secret(network(), VERIFIER_1_SECRET);
    let mut n_of_n_public_keys: Vec<PublicKey> = Vec::new();
    n_of_n_public_keys.push(verifier_0_public_key);
    n_of_n_public_keys.push(verifier_1_public_key);

    let verifier_0_context =
        VerifierContext::new(network(), VERIFIER_0_SECRET, &n_of_n_public_keys);
    let verifier_1_context =
        VerifierContext::new(network(), VERIFIER_1_SECRET, &n_of_n_public_keys);

    [verifier_0_context, verifier_1_context]
}

pub fn get_operator_context() -> OperatorContext {
    let (_, _, verifier_0_public_key) =
        generate_keys_from_secret(network(), VERIFIER_0_SECRET);
    let (_, _, verifier_1_public_key) =
        generate_keys_from_secret(network(), VERIFIER_1_SECRET);
    let mut n_of_n_public_keys: Vec<PublicKey> = Vec::new();
    n_of_n_public_keys.push(verifier_0_public_key);
    n_of_n_public_keys.push(verifier_1_public_key);
    OperatorContext::new(network(), OPERATOR_SECRET, &n_of_n_public_keys)
}


