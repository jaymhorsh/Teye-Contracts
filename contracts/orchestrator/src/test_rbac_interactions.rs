#![allow(clippy::unwrap_used, clippy::expect_used)]
#![cfg(test)]

use crate::{OrchestratorContract};
use common::transaction::{
    ContractType, TransactionError, TransactionLog, TransactionOperation, 
    TransactionPhase, TransactionStatus, TransactionTimeoutConfig,
};
use soroban_sdk::{testutils::Address as _, vec, Address, Env, String, Vec};

fn setup_orchestrator(env: &Env) -> (Address, Address, Address, Address) {
    env.mock_all_auths();
    
    let admin = Address::generate(env);
    let regular_user = Address::generate(env);
    let _privileged_user = Address::generate(env);
    
    let contract_id = env.register(OrchestratorContract, ());
    
    env.as_contract(&contract_id, || {
        OrchestratorContract::initialize(env.clone(), admin.clone(), None).unwrap();
    });
    
    (contract_id, admin, regular_user, _privileged_user)
}

#[test]
fn test_admin_has_all_permissions() {
    let env = Env::default();
    let (contract_id, admin, _, _) = setup_orchestrator(&env);
    
    let config = TransactionTimeoutConfig {
        default_timeout: 600,
        max_timeout: 3600,
        contract_timeouts: Vec::new(&env),
    };
    
    env.as_contract(&contract_id, || {
        let result = OrchestratorContract::update_timeout_config(
            env.clone(), 
            admin.clone(), 
            config
        );
        assert!(result.is_ok());
    });
}

#[test]
fn test_regular_user_cannot_call_admin_functions() {
    let env = Env::default();
    let (contract_id, _, regular_user, _) = setup_orchestrator(&env);
    
    let config = TransactionTimeoutConfig {
        default_timeout: 600,
        max_timeout: 3600,
        contract_timeouts: Vec::new(&env),
    };
    
    env.as_contract(&contract_id, || {
        let result = OrchestratorContract::update_timeout_config(
            env.clone(),
            regular_user.clone(),
            config,
        );
        assert_eq!(result, Err(TransactionError::Unauthorized));
    });
}

#[test]
fn test_regular_user_can_start_transaction() {
    let env = Env::default();
    let (contract_id, _, regular_user, _) = setup_orchestrator(&env);
    
    let operations = Vec::new(&env);
    let metadata = Vec::new(&env);
    
    env.as_contract(&contract_id, || {
        let result = OrchestratorContract::start_transaction(
            env.clone(),
            regular_user.clone(),
            operations,
            Some(300),
            metadata,
        );
        assert!(result.is_ok());
    });
}
