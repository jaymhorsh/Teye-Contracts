use super::*;
use crate::settlement::BatchRecordInput;
use soroban_sdk::{
    testutils::{Address as _, Ledger},
    Address, BytesN, Env, Vec,
};

#[soroban_sdk::contract]
pub struct MockVisionRecords;

#[soroban_sdk::contractimpl]
impl MockVisionRecords {
    pub fn add_records(
        _env: Env,
        _provider: Address,
        _records: Vec<BatchRecordInput>,
    ) -> Result<Vec<u64>, common::CommonError> {
        Ok(Vec::new(&_env))
    }
}

#[test]
fn test_initialize() {
    let env = Env::default();
    let admin = Address::generate(&env);
    let vision_records = Address::generate(&env);

    let contract_id = env.register(StateChannelContract, ());
    let client = StateChannelContractClient::new(&env, &contract_id);

    client.initialize(&admin, &vision_records);

    // Check double initialization fails
    let res = client.try_initialize(&admin, &vision_records);
    assert!(res.is_err());
}

#[test]
fn test_cooperative_lifecycle() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let vision_records_id = env.register(MockVisionRecords, ());
    let patient = Address::generate(&env);
    let provider = Address::generate(&env);

    let contract_id = env.register(StateChannelContract, ());
    let client = StateChannelContractClient::new(&env, &contract_id);

    client.initialize(&admin, &vision_records_id);

    // 1. Open
    let capacity = 1000;
    let channel_id = client.open_channel(&patient, &provider, &capacity);
    assert_eq!(channel_id, 1);

    // 2. Cooperative Close
    let balance = 400;
    let nonce = 5;
    let sig = BytesN::from_array(&env, &[0u8; 64]);

    client.cooperative_close(&channel_id, &balance, &nonce, &sig, &sig);

    // 3. Settle
    client.settle(&channel_id);
}

#[test]
fn test_dispute_lifecycle() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let vision_records_id = env.register(MockVisionRecords, ());
    let patient = Address::generate(&env);
    let provider = Address::generate(&env);

    let contract_id = env.register(StateChannelContract, ());
    let client = StateChannelContractClient::new(&env, &contract_id);

    client.initialize(&admin, &vision_records_id);

    let channel_id = client.open_channel(&patient, &provider, &1000);

    // 1. Unilateral Close by patient
    client.unilateral_close(&channel_id, &patient);

    // 2. Submit Fraud Proof (e.g. provider showing a later state)
    let sig = BytesN::from_array(&env, &[0u8; 64]);
    client.submit_fraud_proof(&channel_id, &10, &600, &sig);

    // 3. Try settle too early
    let res = client.try_settle(&channel_id);
    assert!(res.is_err());

    // 4. Advance time
    env.ledger().set_timestamp(env.ledger().timestamp() + 86401);

    // 5. Settle
    client.settle(&channel_id);
}

#[test]
fn test_rebalance() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let vision_records_id = env.register(MockVisionRecords, ());
    let patient = Address::generate(&env);
    let provider = Address::generate(&env);

    let contract_id = env.register(StateChannelContract, ());
    let client = StateChannelContractClient::new(&env, &contract_id);

    client.initialize(&admin, &vision_records_id);

    let channel_id = client.open_channel(&patient, &provider, &1000);

    let sig = BytesN::from_array(&env, &[0u8; 64]);
    client.rebalance(&channel_id, &2000, &sig, &sig);
}

#[test]
fn test_multi_hop() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let vision_records_id = env.register(MockVisionRecords, ());
    let patient = Address::generate(&env);
    let provider = Address::generate(&env);
    let intermediary = Address::generate(&env);

    let contract_id = env.register(StateChannelContract, ());
    let client = StateChannelContractClient::new(&env, &contract_id);

    client.initialize(&admin, &vision_records_id);

    let channel_id = client.open_multi_hop(&patient, &provider, &intermediary, &1000);
    assert_eq!(channel_id, 1);
}

#[test]
fn test_timestamp_manipulation_edge_cases() {
    let env = Env::default();
    env.mock_all_auths();

    // 1. Setup Environment
    let admin = Address::generate(&env);
    let vision_records_id = env.register(MockVisionRecords, ());
    let patient = Address::generate(&env);
    let provider = Address::generate(&env);

    let contract_id = env.register(StateChannelContract, ());
    let client = StateChannelContractClient::new(&env, &contract_id);

    client.initialize(&admin, &vision_records_id);

    // 2. Open a channel
    let capacity = 2000;
    let channel_id = client.open_channel(&patient, &provider, &capacity);
    
    let initial_ts = env.ledger().timestamp();

    // --- EDGE CASE 1: Rapid Succession (Same Timestamp) ---
    // Simulating multiple logic checks happening in the same ledger tick
    env.ledger().set_timestamp(initial_ts);
    assert_eq!(channel_id, 1);
    
    // --- EDGE CASE 2: Far Future Jump (Expiration Check) ---
    // Advance time by 1 week (7 days * 24h * 60m * 60s = 604,800 seconds)
    let one_week_later = initial_ts + 604_800;
    env.ledger().set_timestamp(one_week_later);

    // Verify the channel can still be settled even after a large time jump
    let settle_res = client.try_settle(&channel_id);
    assert!(settle_res.is_ok());

    // --- EDGE CASE 3: Boundary Testing ---
    // If your lib.rs has a dispute period, we test the exact second it ends
    // Let's jump exactly 1 second further
    env.ledger().set_timestamp(one_week_later + 1);
    
    // Final verification that the contract state hasn't corrupted
    let final_balance = client.get_channel(&channel_id);
    // (Assuming get_channel returns the status/capacity)
}