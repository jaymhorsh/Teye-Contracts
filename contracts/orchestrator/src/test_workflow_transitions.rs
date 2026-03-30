use common::transaction::{TransactionError, TransactionLog, TransactionPhase, TransactionStatus, TransactionOperation, ContractType, get_transaction_log, set_transaction_log};
use soroban_sdk::{testutils::Address as _, vec, Address, Env, String, Vec};
use crate::OrchestratorContract;

fn register_orchestrator(env: &Env) -> Address {
    env.register(OrchestratorContract, ())
}

#[test]
fn test_invalid_state_transition_committed_to_rolled_back() {
    let env = Env::default();
    let admin = Address::generate(&env);
    let contract_id = register_orchestrator(&env);

    env.as_contract(&contract_id, || {
        OrchestratorContract::initialize(env.clone(), admin.clone(), None).unwrap();

        let log = TransactionLog {
            transaction_id: 1,
            initiator: Address::generate(&env),
            phase: TransactionPhase::Committed,
            status: TransactionStatus::Completed,
            operations: Vec::new(&env),
            created_at: env.ledger().timestamp(),
            updated_at: env.ledger().timestamp(),
            timeout_seconds: 300,
            error: None,
            metadata: Vec::new(&env),
        };
        set_transaction_log(&env, &log);

        let result = OrchestratorContract::rollback_transaction(env.clone(), admin.clone(), 1);
        assert_eq!(result, Err(TransactionError::InvalidPhase));
    });
}

#[test]
fn test_recovery_from_failed_state() {
    let env = Env::default();
    let admin = Address::generate(&env);
    let contract_id = register_orchestrator(&env);

    env.as_contract(&contract_id, || {
        OrchestratorContract::initialize(env.clone(), admin.clone(), None).unwrap();

        let log = TransactionLog {
            transaction_id: 2,
            initiator: Address::generate(&env),
            phase: TransactionPhase::RolledBack,
            status: TransactionStatus::Failed,
            operations: Vec::new(&env),
            created_at: env.ledger().timestamp(),
            updated_at: env.ledger().timestamp(),
            timeout_seconds: 300,
            error: Some(String::from_str(&env, "Initial failure")),
            metadata: Vec::new(&env),
        };
        set_transaction_log(&env, &log);

        let result = OrchestratorContract::rollback_transaction(env.clone(), admin.clone(), 2);
        assert_eq!(result, Ok(()));

        let updated_log = get_transaction_log(&env, 2).unwrap();
        assert_eq!(updated_log.phase, TransactionPhase::RolledBack);
        assert_eq!(updated_log.status, TransactionStatus::Cancelled);
    });
}

#[test]
#[should_panic]
fn test_pharmaceutical_supply_chain_workflow_panics_on_missing_contract() {
    let env = Env::default();
    let admin = Address::generate(&env);
    let initiator = Address::generate(&env);
    let contract_id = register_orchestrator(&env);

    env.as_contract(&contract_id, || {
        OrchestratorContract::initialize(env.clone(), admin.clone(), None).unwrap();

        let mut operations = Vec::new(&env);
        operations.push_back(TransactionOperation {
            operation_id: 101,
            contract_type: ContractType::VisionRecords,
            contract_address: Address::generate(&env),
            function_name: String::from_str(&env, "record_batch"),
            parameters: Vec::new(&env),
            locked_resources: vec![&env, String::from_str(&env, "batch_vial_001")],
            prepared: false,
            committed: false,
            error: None,
        });

        // This will panic inside invoke_contract because contract_address is not registered
        let _ = OrchestratorContract::start_transaction(
            env.clone(),
            initiator.clone(),
            operations,
            Some(600),
            vec![&env, String::from_str(&env, "pharma_supply_chain")],
        );
    });
}
