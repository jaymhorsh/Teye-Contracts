use crate::{
    types::{DataFormat, EmrSystem, ExchangeDirection, ProviderStatus, SyncStatus},
    EmrBridgeContract, EmrBridgeContractClient,
};
use soroban_sdk::{testutils::Address as _, Address, Env, String, Vec};

fn setup_env() -> (Env, EmrBridgeContractClient<'static>, Address) {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(EmrBridgeContract, ());
    let client = EmrBridgeContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    client.initialize(&admin);
    (env, client, admin)
}

// ── Initialization Tests ─────────────────────────────────────────────────────

#[test]
fn test_initialize() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(EmrBridgeContract, ());
    let client = EmrBridgeContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);

    client.initialize(&admin);

    let stored_admin = client.get_admin();
    assert_eq!(stored_admin, admin);
}

#[test]
#[should_panic(expected = "Error(Contract, #2)")]
fn test_initialize_twice_fails() {
    let (env, client, _admin) = setup_env();
    let another_admin = Address::generate(&env);
    client.initialize(&another_admin);
}

// ── Provider Onboarding Tests ────────────────────────────────────────────────

#[test]
fn test_register_provider() {
    let (env, client, admin) = setup_env();

    let provider_id = String::from_str(&env, "epic-001");
    let name = String::from_str(&env, "City Hospital Epic");
    let endpoint = String::from_str(&env, "https://epic.cityhospital.org/fhir");

    let provider = client.register_provider(
        &admin,
        &provider_id,
        &name,
        &EmrSystem::EpicFhir,
        &endpoint,
        &DataFormat::FhirR4,
    );

    assert_eq!(provider.provider_id, provider_id);
    assert_eq!(provider.name, name);
    assert_eq!(provider.emr_system, EmrSystem::EpicFhir);
    assert_eq!(provider.data_format, DataFormat::FhirR4);
    assert_eq!(provider.status, ProviderStatus::Pending);
    assert_eq!(provider.registered_by, admin);
}

#[test]
#[should_panic(expected = "Error(Contract, #5)")]
fn test_register_duplicate_provider_fails() {
    let (env, client, admin) = setup_env();

    let provider_id = String::from_str(&env, "epic-001");
    let name = String::from_str(&env, "City Hospital Epic");
    let endpoint = String::from_str(&env, "https://epic.cityhospital.org/fhir");

    client.register_provider(
        &admin,
        &provider_id,
        &name,
        &EmrSystem::EpicFhir,
        &endpoint,
        &DataFormat::FhirR4,
    );

    // Registering again with same ID should fail
    client.register_provider(
        &admin,
        &provider_id,
        &name,
        &EmrSystem::EpicFhir,
        &endpoint,
        &DataFormat::FhirR4,
    );
}

#[test]
fn test_activate_provider() {
    let (env, client, admin) = setup_env();

    let provider_id = String::from_str(&env, "epic-001");
    let name = String::from_str(&env, "City Hospital Epic");
    let endpoint = String::from_str(&env, "https://epic.cityhospital.org/fhir");

    client.register_provider(
        &admin,
        &provider_id,
        &name,
        &EmrSystem::EpicFhir,
        &endpoint,
        &DataFormat::FhirR4,
    );

    client.activate_provider(&admin, &provider_id);

    let provider = client.get_provider(&provider_id);
    assert_eq!(provider.status, ProviderStatus::Active);
}

#[test]
fn test_suspend_provider() {
    let (env, client, admin) = setup_env();

    let provider_id = String::from_str(&env, "epic-001");
    let name = String::from_str(&env, "City Hospital Epic");
    let endpoint = String::from_str(&env, "https://epic.cityhospital.org/fhir");

    client.register_provider(
        &admin,
        &provider_id,
        &name,
        &EmrSystem::EpicFhir,
        &endpoint,
        &DataFormat::FhirR4,
    );

    client.activate_provider(&admin, &provider_id);
    client.suspend_provider(&admin, &provider_id);

    let provider = client.get_provider(&provider_id);
    assert_eq!(provider.status, ProviderStatus::Suspended);
}

#[test]
fn test_list_providers() {
    let (env, client, admin) = setup_env();

    let id1 = String::from_str(&env, "epic-001");
    let id2 = String::from_str(&env, "cerner-001");
    let name1 = String::from_str(&env, "City Hospital");
    let name2 = String::from_str(&env, "County Clinic");
    let endpoint1 = String::from_str(&env, "https://epic.city.org/fhir");
    let endpoint2 = String::from_str(&env, "https://cerner.county.org/api");

    client.register_provider(
        &admin,
        &id1,
        &name1,
        &EmrSystem::EpicFhir,
        &endpoint1,
        &DataFormat::FhirR4,
    );
    client.register_provider(
        &admin,
        &id2,
        &name2,
        &EmrSystem::CernerMillennium,
        &endpoint2,
        &DataFormat::Hl7V2,
    );

    let providers = client.list_providers();
    assert_eq!(providers.len(), 2);
}

#[test]
#[should_panic(expected = "Error(Contract, #3)")]
fn test_non_admin_cannot_register_provider() {
    let (env, client, _admin) = setup_env();

    let non_admin = Address::generate(&env);
    let provider_id = String::from_str(&env, "epic-001");
    let name = String::from_str(&env, "City Hospital");
    let endpoint = String::from_str(&env, "https://epic.city.org/fhir");

    client.register_provider(
        &non_admin,
        &provider_id,
        &name,
        &EmrSystem::EpicFhir,
        &endpoint,
        &DataFormat::FhirR4,
    );
}

// ── Data Exchange Tests ──────────────────────────────────────────────────────

#[test]
fn test_record_data_exchange() {
    let (env, client, admin) = setup_env();

    // Register and activate provider first
    let provider_id = String::from_str(&env, "epic-001");
    let name = String::from_str(&env, "City Hospital");
    let endpoint = String::from_str(&env, "https://epic.city.org/fhir");

    client.register_provider(
        &admin,
        &provider_id,
        &name,
        &EmrSystem::EpicFhir,
        &endpoint,
        &DataFormat::FhirR4,
    );
    client.activate_provider(&admin, &provider_id);

    let exchange_id = String::from_str(&env, "ex-001");
    let patient_id = String::from_str(&env, "pat-123");
    let resource_type = String::from_str(&env, "Patient");
    let record_hash = String::from_str(&env, "abc123hash");

    let record = client.record_data_exchange(
        &admin,
        &exchange_id,
        &provider_id,
        &patient_id,
        &ExchangeDirection::Import,
        &DataFormat::FhirR4,
        &resource_type,
        &record_hash,
    );

    assert_eq!(record.exchange_id, exchange_id);
    assert_eq!(record.provider_id, provider_id);
    assert_eq!(record.patient_id, patient_id);
    assert_eq!(record.direction, ExchangeDirection::Import);
    assert_eq!(record.status, SyncStatus::Pending);
}

#[test]
#[should_panic(expected = "Error(Contract, #6)")]
fn test_exchange_with_inactive_provider_fails() {
    let (env, client, admin) = setup_env();

    // Register provider but do NOT activate
    let provider_id = String::from_str(&env, "epic-001");
    let name = String::from_str(&env, "City Hospital");
    let endpoint = String::from_str(&env, "https://epic.city.org/fhir");

    client.register_provider(
        &admin,
        &provider_id,
        &name,
        &EmrSystem::EpicFhir,
        &endpoint,
        &DataFormat::FhirR4,
    );

    let exchange_id = String::from_str(&env, "ex-001");
    let patient_id = String::from_str(&env, "pat-123");
    let resource_type = String::from_str(&env, "Patient");
    let record_hash = String::from_str(&env, "abc123hash");

    client.record_data_exchange(
        &admin,
        &exchange_id,
        &provider_id,
        &patient_id,
        &ExchangeDirection::Import,
        &DataFormat::FhirR4,
        &resource_type,
        &record_hash,
    );
}

#[test]
fn test_update_exchange_status() {
    let (env, client, admin) = setup_env();

    let provider_id = String::from_str(&env, "epic-001");
    let name = String::from_str(&env, "City Hospital");
    let endpoint = String::from_str(&env, "https://epic.city.org/fhir");

    client.register_provider(
        &admin,
        &provider_id,
        &name,
        &EmrSystem::EpicFhir,
        &endpoint,
        &DataFormat::FhirR4,
    );
    client.activate_provider(&admin, &provider_id);

    let exchange_id = String::from_str(&env, "ex-001");
    let patient_id = String::from_str(&env, "pat-123");
    let resource_type = String::from_str(&env, "Patient");
    let record_hash = String::from_str(&env, "abc123hash");

    client.record_data_exchange(
        &admin,
        &exchange_id,
        &provider_id,
        &patient_id,
        &ExchangeDirection::Import,
        &DataFormat::FhirR4,
        &resource_type,
        &record_hash,
    );

    client.update_exchange_status(&admin, &exchange_id, &SyncStatus::InProgress);

    let record = client.get_exchange(&exchange_id);
    assert_eq!(record.status, SyncStatus::InProgress);
}

#[test]
fn test_get_patient_exchanges() {
    let (env, client, admin) = setup_env();

    let provider_id = String::from_str(&env, "epic-001");
    let name = String::from_str(&env, "City Hospital");
    let endpoint = String::from_str(&env, "https://epic.city.org/fhir");

    client.register_provider(
        &admin,
        &provider_id,
        &name,
        &EmrSystem::EpicFhir,
        &endpoint,
        &DataFormat::FhirR4,
    );
    client.activate_provider(&admin, &provider_id);

    let patient_id = String::from_str(&env, "pat-123");
    let resource_type = String::from_str(&env, "Patient");
    let hash = String::from_str(&env, "hash1");

    let ex1 = String::from_str(&env, "ex-001");
    let ex2 = String::from_str(&env, "ex-002");

    client.record_data_exchange(
        &admin,
        &ex1,
        &provider_id,
        &patient_id,
        &ExchangeDirection::Import,
        &DataFormat::FhirR4,
        &resource_type,
        &hash,
    );
    client.record_data_exchange(
        &admin,
        &ex2,
        &provider_id,
        &patient_id,
        &ExchangeDirection::Export,
        &DataFormat::FhirR4,
        &resource_type,
        &hash,
    );

    let exchanges = client.get_patient_exchanges(&patient_id);
    assert_eq!(exchanges.len(), 2);
}

// ── Data Mapping Tests ───────────────────────────────────────────────────────

#[test]
fn test_create_field_mapping() {
    let (env, client, admin) = setup_env();

    let provider_id = String::from_str(&env, "epic-001");
    let name = String::from_str(&env, "City Hospital");
    let endpoint = String::from_str(&env, "https://epic.city.org/fhir");

    client.register_provider(
        &admin,
        &provider_id,
        &name,
        &EmrSystem::EpicFhir,
        &endpoint,
        &DataFormat::FhirR4,
    );

    let mapping_id = String::from_str(&env, "map-001");
    let source_field = String::from_str(&env, "patient.name.given");
    let target_field = String::from_str(&env, "first_name");
    let transform_rule = String::from_str(&env, "direct_copy");

    let mapping = client.create_field_mapping(
        &admin,
        &mapping_id,
        &provider_id,
        &source_field,
        &target_field,
        &transform_rule,
    );

    assert_eq!(mapping.mapping_id, mapping_id);
    assert_eq!(mapping.provider_id, provider_id);
    assert_eq!(mapping.source_field, source_field);
    assert_eq!(mapping.target_field, target_field);
    assert_eq!(mapping.transform_rule, transform_rule);
}

#[test]
#[should_panic(expected = "Error(Contract, #7)")]
fn test_create_mapping_with_empty_fields_fails() {
    let (env, client, admin) = setup_env();

    let provider_id = String::from_str(&env, "epic-001");
    let name = String::from_str(&env, "City Hospital");
    let endpoint = String::from_str(&env, "https://epic.city.org/fhir");

    client.register_provider(
        &admin,
        &provider_id,
        &name,
        &EmrSystem::EpicFhir,
        &endpoint,
        &DataFormat::FhirR4,
    );

    let mapping_id = String::from_str(&env, "map-001");
    let empty = String::from_str(&env, "");
    let target_field = String::from_str(&env, "first_name");
    let transform_rule = String::from_str(&env, "direct_copy");

    client.create_field_mapping(
        &admin,
        &mapping_id,
        &provider_id,
        &empty,
        &target_field,
        &transform_rule,
    );
}

#[test]
fn test_get_provider_mappings() {
    let (env, client, admin) = setup_env();

    let provider_id = String::from_str(&env, "epic-001");
    let name = String::from_str(&env, "City Hospital");
    let endpoint = String::from_str(&env, "https://epic.city.org/fhir");

    client.register_provider(
        &admin,
        &provider_id,
        &name,
        &EmrSystem::EpicFhir,
        &endpoint,
        &DataFormat::FhirR4,
    );

    let map1 = String::from_str(&env, "map-001");
    let map2 = String::from_str(&env, "map-002");
    let src1 = String::from_str(&env, "patient.name.given");
    let tgt1 = String::from_str(&env, "first_name");
    let src2 = String::from_str(&env, "patient.name.family");
    let tgt2 = String::from_str(&env, "last_name");
    let rule = String::from_str(&env, "direct_copy");

    client.create_field_mapping(&admin, &map1, &provider_id, &src1, &tgt1, &rule);
    client.create_field_mapping(&admin, &map2, &provider_id, &src2, &tgt2, &rule);

    let mappings = client.get_provider_mappings(&provider_id);
    assert_eq!(mappings.len(), 2);
}

// ── Sync Verification Tests ──────────────────────────────────────────────────

#[test]
fn test_verify_sync_consistent() {
    let (env, client, admin) = setup_env();

    // Setup provider and exchange
    let provider_id = String::from_str(&env, "epic-001");
    let name = String::from_str(&env, "City Hospital");
    let endpoint = String::from_str(&env, "https://epic.city.org/fhir");

    client.register_provider(
        &admin,
        &provider_id,
        &name,
        &EmrSystem::EpicFhir,
        &endpoint,
        &DataFormat::FhirR4,
    );
    client.activate_provider(&admin, &provider_id);

    let exchange_id = String::from_str(&env, "ex-001");
    let patient_id = String::from_str(&env, "pat-123");
    let resource_type = String::from_str(&env, "Patient");
    let record_hash = String::from_str(&env, "abc123hash");

    client.record_data_exchange(
        &admin,
        &exchange_id,
        &provider_id,
        &patient_id,
        &ExchangeDirection::Import,
        &DataFormat::FhirR4,
        &resource_type,
        &record_hash,
    );

    let verification_id = String::from_str(&env, "ver-001");
    let source_hash = String::from_str(&env, "hash_abc");
    let target_hash = String::from_str(&env, "hash_abc");
    let discrepancies: Vec<String> = Vec::new(&env);

    let verification = client.verify_sync(
        &admin,
        &verification_id,
        &exchange_id,
        &source_hash,
        &target_hash,
        &discrepancies,
    );

    assert!(verification.is_consistent);
    assert_eq!(verification.verification_id, verification_id);

    // Exchange status should be updated to Completed
    let exchange = client.get_exchange(&exchange_id);
    assert_eq!(exchange.status, SyncStatus::Completed);
}

#[test]
fn test_verify_sync_inconsistent() {
    let (env, client, admin) = setup_env();

    let provider_id = String::from_str(&env, "epic-001");
    let name = String::from_str(&env, "City Hospital");
    let endpoint = String::from_str(&env, "https://epic.city.org/fhir");

    client.register_provider(
        &admin,
        &provider_id,
        &name,
        &EmrSystem::EpicFhir,
        &endpoint,
        &DataFormat::FhirR4,
    );
    client.activate_provider(&admin, &provider_id);

    let exchange_id = String::from_str(&env, "ex-001");
    let patient_id = String::from_str(&env, "pat-123");
    let resource_type = String::from_str(&env, "Patient");
    let record_hash = String::from_str(&env, "abc123hash");

    client.record_data_exchange(
        &admin,
        &exchange_id,
        &provider_id,
        &patient_id,
        &ExchangeDirection::Import,
        &DataFormat::FhirR4,
        &resource_type,
        &record_hash,
    );

    let verification_id = String::from_str(&env, "ver-001");
    let source_hash = String::from_str(&env, "hash_abc");
    let target_hash = String::from_str(&env, "hash_xyz");
    let mut discrepancies: Vec<String> = Vec::new(&env);
    discrepancies.push_back(String::from_str(&env, "name field mismatch"));

    let verification = client.verify_sync(
        &admin,
        &verification_id,
        &exchange_id,
        &source_hash,
        &target_hash,
        &discrepancies,
    );

    assert!(!verification.is_consistent);

    // Exchange status should be PartialSuccess
    let exchange = client.get_exchange(&exchange_id);
    assert_eq!(exchange.status, SyncStatus::PartialSuccess);
}

#[test]
#[should_panic(expected = "Error(Contract, #8)")]
fn test_verify_sync_nonexistent_exchange_fails() {
    let (env, client, admin) = setup_env();

    let verification_id = String::from_str(&env, "ver-001");
    let exchange_id = String::from_str(&env, "nonexistent");
    let source_hash = String::from_str(&env, "hash_abc");
    let target_hash = String::from_str(&env, "hash_abc");
    let discrepancies: Vec<String> = Vec::new(&env);

    client.verify_sync(
        &admin,
        &verification_id,
        &exchange_id,
        &source_hash,
        &target_hash,
        &discrepancies,
    );
}

// ── Invalid Input / Fuzz Tests ───────────────────────────────────────────────
// These tests pass malformed, empty, boundary, and unexpected arguments to
// every contract entry point and verify the contract rejects them gracefully
// with the correct descriptive error codes.

// --- initialize ---

#[test]
#[should_panic(expected = "Error(Contract, #2)")]
fn fuzz_initialize_already_initialized() {
    // Calling initialize a second time must return AlreadyInitialized (#2)
    let (env, client, _) = setup_env();
    let attacker = Address::generate(&env);
    client.initialize(&attacker);
}

// --- get_admin before init ---

#[test]
#[should_panic(expected = "Error(Contract, #1)")]
fn fuzz_get_admin_before_init() {
    // Calling get_admin on a fresh, uninitialized contract must return NotInitialized (#1)
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(EmrBridgeContract, ());
    let client = EmrBridgeContractClient::new(&env, &contract_id);
    client.get_admin();
}

// --- register_provider: unauthorized caller ---

#[test]
#[should_panic(expected = "Error(Contract, #3)")]
fn fuzz_register_provider_random_caller_rejected() {
    let (env, client, _) = setup_env();
    let rando = Address::generate(&env);
    client.register_provider(
        &rando,
        &String::from_str(&env, "p-fuzz"),
        &String::from_str(&env, "Fuzz Hospital"),
        &EmrSystem::Custom,
        &String::from_str(&env, "https://fuzz.example.com"),
        &DataFormat::Custom,
    );
}

// --- register_provider: empty provider_id ---

#[test]
#[should_panic(expected = "Error(Contract, #5)")]
fn fuzz_register_provider_empty_id_then_duplicate() {
    // Two registrations with the same empty string ID must trigger ProviderAlreadyExists (#5)
    let (env, client, admin) = setup_env();
    let empty = String::from_str(&env, "");
    client.register_provider(
        &admin,
        &empty,
        &String::from_str(&env, "A"),
        &EmrSystem::Custom,
        &String::from_str(&env, "https://a.example.com"),
        &DataFormat::Custom,
    );
    client.register_provider(
        &admin,
        &empty,
        &String::from_str(&env, "B"),
        &EmrSystem::Custom,
        &String::from_str(&env, "https://b.example.com"),
        &DataFormat::Custom,
    );
}

// --- activate_provider: nonexistent provider ---

#[test]
#[should_panic(expected = "Error(Contract, #4)")]
fn fuzz_activate_nonexistent_provider() {
    let (env, client, admin) = setup_env();
    client.activate_provider(&admin, &String::from_str(&env, "ghost-provider"));
}

// --- activate_provider: unauthorized ---

#[test]
#[should_panic(expected = "Error(Contract, #3)")]
fn fuzz_activate_provider_unauthorized() {
    let (env, client, admin) = setup_env();
    let provider_id = String::from_str(&env, "p-001");
    client.register_provider(
        &admin,
        &provider_id,
        &String::from_str(&env, "Hospital"),
        &EmrSystem::EpicFhir,
        &String::from_str(&env, "https://epic.example.com"),
        &DataFormat::FhirR4,
    );
    let rando = Address::generate(&env);
    client.activate_provider(&rando, &provider_id);
}

// --- suspend_provider: nonexistent provider ---

#[test]
#[should_panic(expected = "Error(Contract, #4)")]
fn fuzz_suspend_nonexistent_provider() {
    let (env, client, admin) = setup_env();
    client.suspend_provider(&admin, &String::from_str(&env, "ghost-provider"));
}

// --- suspend_provider: unauthorized ---

#[test]
#[should_panic(expected = "Error(Contract, #3)")]
fn fuzz_suspend_provider_unauthorized() {
    let (env, client, admin) = setup_env();
    let provider_id = String::from_str(&env, "p-001");
    client.register_provider(
        &admin,
        &provider_id,
        &String::from_str(&env, "Hospital"),
        &EmrSystem::EpicFhir,
        &String::from_str(&env, "https://epic.example.com"),
        &DataFormat::FhirR4,
    );
    client.activate_provider(&admin, &provider_id);
    let rando = Address::generate(&env);
    client.suspend_provider(&rando, &provider_id);
}

// --- get_provider: nonexistent ---

#[test]
#[should_panic(expected = "Error(Contract, #4)")]
fn fuzz_get_provider_nonexistent() {
    let (env, client, _) = setup_env();
    client.get_provider(&String::from_str(&env, "does-not-exist"));
}

// --- get_provider: empty id ---

#[test]
#[should_panic(expected = "Error(Contract, #4)")]
fn fuzz_get_provider_empty_id() {
    let (env, client, _) = setup_env();
    client.get_provider(&String::from_str(&env, ""));
}

// --- record_data_exchange: provider not found ---

#[test]
#[should_panic(expected = "Error(Contract, #4)")]
fn fuzz_record_exchange_unknown_provider() {
    let (env, client, admin) = setup_env();
    client.record_data_exchange(
        &admin,
        &String::from_str(&env, "ex-fuzz"),
        &String::from_str(&env, "ghost-provider"),
        &String::from_str(&env, "pat-000"),
        &ExchangeDirection::Import,
        &DataFormat::FhirR4,
        &String::from_str(&env, "Patient"),
        &String::from_str(&env, "hash"),
    );
}

// --- record_data_exchange: provider exists but is suspended (not active) ---

#[test]
#[should_panic(expected = "Error(Contract, #6)")]
fn fuzz_record_exchange_suspended_provider() {
    let (env, client, admin) = setup_env();
    let provider_id = String::from_str(&env, "p-001");
    client.register_provider(
        &admin,
        &provider_id,
        &String::from_str(&env, "Hospital"),
        &EmrSystem::EpicFhir,
        &String::from_str(&env, "https://epic.example.com"),
        &DataFormat::FhirR4,
    );
    client.activate_provider(&admin, &provider_id);
    client.suspend_provider(&admin, &provider_id);
    client.record_data_exchange(
        &admin,
        &String::from_str(&env, "ex-001"),
        &provider_id,
        &String::from_str(&env, "pat-001"),
        &ExchangeDirection::Import,
        &DataFormat::FhirR4,
        &String::from_str(&env, "Patient"),
        &String::from_str(&env, "hash"),
    );
}

// --- record_data_exchange: duplicate exchange id ---

#[test]
#[should_panic(expected = "Error(Contract, #9)")]
fn fuzz_record_exchange_duplicate_id() {
    let (env, client, admin) = setup_env();
    let provider_id = String::from_str(&env, "p-001");
    client.register_provider(
        &admin,
        &provider_id,
        &String::from_str(&env, "Hospital"),
        &EmrSystem::EpicFhir,
        &String::from_str(&env, "https://epic.example.com"),
        &DataFormat::FhirR4,
    );
    client.activate_provider(&admin, &provider_id);
    let exchange_id = String::from_str(&env, "ex-dup");
    let patient_id = String::from_str(&env, "pat-001");
    let resource_type = String::from_str(&env, "Patient");
    let hash = String::from_str(&env, "hash");
    client.record_data_exchange(
        &admin, &exchange_id, &provider_id, &patient_id,
        &ExchangeDirection::Import, &DataFormat::FhirR4, &resource_type, &hash,
    );
    client.record_data_exchange(
        &admin, &exchange_id, &provider_id, &patient_id,
        &ExchangeDirection::Export, &DataFormat::FhirR4, &resource_type, &hash,
    );
}

// --- record_data_exchange: unauthorized caller ---

#[test]
#[should_panic(expected = "Error(Contract, #3)")]
fn fuzz_record_exchange_unauthorized() {
    let (env, client, admin) = setup_env();
    let provider_id = String::from_str(&env, "p-001");
    client.register_provider(
        &admin,
        &provider_id,
        &String::from_str(&env, "Hospital"),
        &EmrSystem::EpicFhir,
        &String::from_str(&env, "https://epic.example.com"),
        &DataFormat::FhirR4,
    );
    client.activate_provider(&admin, &provider_id);
    let rando = Address::generate(&env);
    client.record_data_exchange(
        &rando,
        &String::from_str(&env, "ex-001"),
        &provider_id,
        &String::from_str(&env, "pat-001"),
        &ExchangeDirection::Import,
        &DataFormat::FhirR4,
        &String::from_str(&env, "Patient"),
        &String::from_str(&env, "hash"),
    );
}

// --- update_exchange_status: nonexistent exchange ---

#[test]
#[should_panic(expected = "Error(Contract, #8)")]
fn fuzz_update_exchange_status_nonexistent() {
    let (env, client, admin) = setup_env();
    client.update_exchange_status(
        &admin,
        &String::from_str(&env, "ghost-exchange"),
        &SyncStatus::Failed,
    );
}

// --- update_exchange_status: unauthorized ---

#[test]
#[should_panic(expected = "Error(Contract, #3)")]
fn fuzz_update_exchange_status_unauthorized() {
    let (env, client, admin) = setup_env();
    let provider_id = String::from_str(&env, "p-001");
    client.register_provider(
        &admin, &provider_id,
        &String::from_str(&env, "Hospital"),
        &EmrSystem::EpicFhir,
        &String::from_str(&env, "https://epic.example.com"),
        &DataFormat::FhirR4,
    );
    client.activate_provider(&admin, &provider_id);
    let exchange_id = String::from_str(&env, "ex-001");
    client.record_data_exchange(
        &admin, &exchange_id, &provider_id,
        &String::from_str(&env, "pat-001"),
        &ExchangeDirection::Import, &DataFormat::FhirR4,
        &String::from_str(&env, "Patient"),
        &String::from_str(&env, "hash"),
    );
    let rando = Address::generate(&env);
    client.update_exchange_status(&rando, &exchange_id, &SyncStatus::Failed);
}

// --- get_exchange: nonexistent ---

#[test]
#[should_panic(expected = "Error(Contract, #8)")]
fn fuzz_get_exchange_nonexistent() {
    let (env, client, _) = setup_env();
    client.get_exchange(&String::from_str(&env, "ghost-exchange"));
}

// --- get_exchange: empty id ---

#[test]
#[should_panic(expected = "Error(Contract, #8)")]
fn fuzz_get_exchange_empty_id() {
    let (env, client, _) = setup_env();
    client.get_exchange(&String::from_str(&env, ""));
}

// --- create_field_mapping: provider not found ---

#[test]
#[should_panic(expected = "Error(Contract, #4)")]
fn fuzz_create_mapping_unknown_provider() {
    let (env, client, admin) = setup_env();
    client.create_field_mapping(
        &admin,
        &String::from_str(&env, "map-001"),
        &String::from_str(&env, "ghost-provider"),
        &String::from_str(&env, "src"),
        &String::from_str(&env, "tgt"),
        &String::from_str(&env, "direct"),
    );
}

// --- create_field_mapping: empty target field ---

#[test]
#[should_panic(expected = "Error(Contract, #7)")]
fn fuzz_create_mapping_empty_target_field() {
    let (env, client, admin) = setup_env();
    let provider_id = String::from_str(&env, "p-001");
    client.register_provider(
        &admin, &provider_id,
        &String::from_str(&env, "Hospital"),
        &EmrSystem::EpicFhir,
        &String::from_str(&env, "https://epic.example.com"),
        &DataFormat::FhirR4,
    );
    client.create_field_mapping(
        &admin,
        &String::from_str(&env, "map-001"),
        &provider_id,
        &String::from_str(&env, "source.field"),
        &String::from_str(&env, ""),  // empty target
        &String::from_str(&env, "direct"),
    );
}

// --- create_field_mapping: duplicate mapping id ---

#[test]
#[should_panic(expected = "Error(Contract, #12)")]
fn fuzz_create_mapping_duplicate_id() {
    let (env, client, admin) = setup_env();
    let provider_id = String::from_str(&env, "p-001");
    client.register_provider(
        &admin, &provider_id,
        &String::from_str(&env, "Hospital"),
        &EmrSystem::EpicFhir,
        &String::from_str(&env, "https://epic.example.com"),
        &DataFormat::FhirR4,
    );
    let mapping_id = String::from_str(&env, "map-dup");
    let src = String::from_str(&env, "src");
    let tgt = String::from_str(&env, "tgt");
    let rule = String::from_str(&env, "direct");
    client.create_field_mapping(&admin, &mapping_id, &provider_id, &src, &tgt, &rule);
    client.create_field_mapping(&admin, &mapping_id, &provider_id, &src, &tgt, &rule);
}

// --- create_field_mapping: unauthorized ---

#[test]
#[should_panic(expected = "Error(Contract, #3)")]
fn fuzz_create_mapping_unauthorized() {
    let (env, client, admin) = setup_env();
    let provider_id = String::from_str(&env, "p-001");
    client.register_provider(
        &admin, &provider_id,
        &String::from_str(&env, "Hospital"),
        &EmrSystem::EpicFhir,
        &String::from_str(&env, "https://epic.example.com"),
        &DataFormat::FhirR4,
    );
    let rando = Address::generate(&env);
    client.create_field_mapping(
        &rando,
        &String::from_str(&env, "map-001"),
        &provider_id,
        &String::from_str(&env, "src"),
        &String::from_str(&env, "tgt"),
        &String::from_str(&env, "direct"),
    );
}

// --- get_field_mapping: nonexistent ---

#[test]
#[should_panic(expected = "Error(Contract, #7)")]
fn fuzz_get_field_mapping_nonexistent() {
    let (env, client, _) = setup_env();
    client.get_field_mapping(&String::from_str(&env, "ghost-mapping"));
}

// --- get_field_mapping: empty id ---

#[test]
#[should_panic(expected = "Error(Contract, #7)")]
fn fuzz_get_field_mapping_empty_id() {
    let (env, client, _) = setup_env();
    client.get_field_mapping(&String::from_str(&env, ""));
}

// --- verify_sync: duplicate verification id ---

#[test]
#[should_panic(expected = "Error(Contract, #14)")]
fn fuzz_verify_sync_duplicate_verification_id() {
    let (env, client, admin) = setup_env();
    let provider_id = String::from_str(&env, "p-001");
    client.register_provider(
        &admin, &provider_id,
        &String::from_str(&env, "Hospital"),
        &EmrSystem::EpicFhir,
        &String::from_str(&env, "https://epic.example.com"),
        &DataFormat::FhirR4,
    );
    client.activate_provider(&admin, &provider_id);
    let exchange_id = String::from_str(&env, "ex-001");
    client.record_data_exchange(
        &admin, &exchange_id, &provider_id,
        &String::from_str(&env, "pat-001"),
        &ExchangeDirection::Import, &DataFormat::FhirR4,
        &String::from_str(&env, "Patient"),
        &String::from_str(&env, "hash"),
    );
    let ver_id = String::from_str(&env, "ver-dup");
    let h = String::from_str(&env, "hash_abc");
    let disc: Vec<String> = Vec::new(&env);
    client.verify_sync(&admin, &ver_id, &exchange_id, &h, &h, &disc);
    client.verify_sync(&admin, &ver_id, &exchange_id, &h, &h, &disc);
}

// --- verify_sync: unauthorized ---

#[test]
#[should_panic(expected = "Error(Contract, #3)")]
fn fuzz_verify_sync_unauthorized() {
    let (env, client, admin) = setup_env();
    let provider_id = String::from_str(&env, "p-001");
    client.register_provider(
        &admin, &provider_id,
        &String::from_str(&env, "Hospital"),
        &EmrSystem::EpicFhir,
        &String::from_str(&env, "https://epic.example.com"),
        &DataFormat::FhirR4,
    );
    client.activate_provider(&admin, &provider_id);
    let exchange_id = String::from_str(&env, "ex-001");
    client.record_data_exchange(
        &admin, &exchange_id, &provider_id,
        &String::from_str(&env, "pat-001"),
        &ExchangeDirection::Import, &DataFormat::FhirR4,
        &String::from_str(&env, "Patient"),
        &String::from_str(&env, "hash"),
    );
    let rando = Address::generate(&env);
    let h = String::from_str(&env, "hash_abc");
    let disc: Vec<String> = Vec::new(&env);
    client.verify_sync(&rando, &String::from_str(&env, "ver-001"), &exchange_id, &h, &h, &disc);
}

// --- get_verification: nonexistent ---

#[test]
#[should_panic(expected = "Error(Contract, #13)")]
fn fuzz_get_verification_nonexistent() {
    let (env, client, _) = setup_env();
    client.get_verification(&String::from_str(&env, "ghost-ver"));
}

// --- get_verification: empty id ---

#[test]
#[should_panic(expected = "Error(Contract, #13)")]
fn fuzz_get_verification_empty_id() {
    let (env, client, _) = setup_env();
    client.get_verification(&String::from_str(&env, ""));
}

// --- Boundary / edge-case fuzz scenarios ---

// Whitespace-only strings should be treated as valid (non-empty) by the contract
// but should not collide with normal IDs
#[test]
fn fuzz_whitespace_provider_id_is_distinct_from_normal() {
    let (env, client, admin) = setup_env();
    let ws_id = String::from_str(&env, "   ");
    let normal_id = String::from_str(&env, "p-001");
    client.register_provider(
        &admin, &ws_id,
        &String::from_str(&env, "WS Hospital"),
        &EmrSystem::Custom,
        &String::from_str(&env, "https://ws.example.com"),
        &DataFormat::Custom,
    );
    client.register_provider(
        &admin, &normal_id,
        &String::from_str(&env, "Normal Hospital"),
        &EmrSystem::EpicFhir,
        &String::from_str(&env, "https://normal.example.com"),
        &DataFormat::FhirR4,
    );
    let providers = client.list_providers();
    assert_eq!(providers.len(), 2);
}

// Verify that a pending (never activated) provider blocks data exchange
#[test]
#[should_panic(expected = "Error(Contract, #6)")]
fn fuzz_exchange_with_pending_provider_rejected() {
    let (env, client, admin) = setup_env();
    let provider_id = String::from_str(&env, "p-pending");
    client.register_provider(
        &admin, &provider_id,
        &String::from_str(&env, "Pending Hospital"),
        &EmrSystem::Allscripts,
        &String::from_str(&env, "https://allscripts.example.com"),
        &DataFormat::CcdA,
    );
    // Do NOT activate — status remains Pending
    client.record_data_exchange(
        &admin,
        &String::from_str(&env, "ex-001"),
        &provider_id,
        &String::from_str(&env, "pat-001"),
        &ExchangeDirection::Export,
        &DataFormat::CcdA,
        &String::from_str(&env, "Observation"),
        &String::from_str(&env, "hash"),
    );
}

// Verify all EMR system variants and data format variants are accepted
#[test]
fn fuzz_all_emr_system_and_format_variants_accepted() {
    let (env, client, admin) = setup_env();
    let systems = [
        (EmrSystem::EpicFhir, DataFormat::FhirR4, "epic"),
        (EmrSystem::CernerMillennium, DataFormat::Hl7V2, "cerner"),
        (EmrSystem::Allscripts, DataFormat::CcdA, "allscripts"),
        (EmrSystem::Athenahealth, DataFormat::Custom, "athena"),
        (EmrSystem::Custom, DataFormat::Custom, "custom"),
    ];
    for (system, format, id_suffix) in systems {
        let pid = String::from_str(&env, id_suffix);
        client.register_provider(
            &admin, &pid,
            &String::from_str(&env, "Hospital"),
            &system,
            &String::from_str(&env, "https://example.com"),
            &format,
        );
    }
    let providers = client.list_providers();
    assert_eq!(providers.len(), 5);
}

// Verify all SyncStatus variants can be set via update_exchange_status
#[test]
fn fuzz_all_sync_status_variants_accepted() {
    let (env, client, admin) = setup_env();
    let provider_id = String::from_str(&env, "p-001");
    client.register_provider(
        &admin, &provider_id,
        &String::from_str(&env, "Hospital"),
        &EmrSystem::EpicFhir,
        &String::from_str(&env, "https://epic.example.com"),
        &DataFormat::FhirR4,
    );
    client.activate_provider(&admin, &provider_id);

    let cases: &[(SyncStatus, &str)] = &[
        (SyncStatus::InProgress, "ex-0"),
        (SyncStatus::Failed, "ex-1"),
        (SyncStatus::PartialSuccess, "ex-2"),
        (SyncStatus::Completed, "ex-3"),
        (SyncStatus::Pending, "ex-4"),
    ];
    for (status, id_str) in cases {
        let ex_id = String::from_str(&env, id_str);
        client.record_data_exchange(
            &admin, &ex_id, &provider_id,
            &String::from_str(&env, "pat-001"),
            &ExchangeDirection::Import, &DataFormat::FhirR4,
            &String::from_str(&env, "Patient"),
            &String::from_str(&env, "hash"),
        );
        client.update_exchange_status(&admin, &ex_id, status);
        let record = client.get_exchange(&ex_id);
        assert_eq!(record.status, *status);
    }
}

// Verify that matching hashes but non-empty discrepancies yields inconsistent
#[test]
fn fuzz_verify_sync_matching_hashes_with_discrepancies_is_inconsistent() {
    let (env, client, admin) = setup_env();
    let provider_id = String::from_str(&env, "p-001");
    client.register_provider(
        &admin, &provider_id,
        &String::from_str(&env, "Hospital"),
        &EmrSystem::EpicFhir,
        &String::from_str(&env, "https://epic.example.com"),
        &DataFormat::FhirR4,
    );
    client.activate_provider(&admin, &provider_id);
    let exchange_id = String::from_str(&env, "ex-001");
    client.record_data_exchange(
        &admin, &exchange_id, &provider_id,
        &String::from_str(&env, "pat-001"),
        &ExchangeDirection::Import, &DataFormat::FhirR4,
        &String::from_str(&env, "Patient"),
        &String::from_str(&env, "hash"),
    );
    let same_hash = String::from_str(&env, "same_hash");
    let mut disc: Vec<String> = Vec::new(&env);
    disc.push_back(String::from_str(&env, "unexpected field diff"));
    let ver = client.verify_sync(
        &admin,
        &String::from_str(&env, "ver-001"),
        &exchange_id,
        &same_hash,
        &same_hash,
        &disc,
    );
    // Even though hashes match, discrepancies make it inconsistent
    assert!(!ver.is_consistent);
    let record = client.get_exchange(&exchange_id);
    assert_eq!(record.status, SyncStatus::PartialSuccess);
}

// Verify that differing hashes with empty discrepancies yields inconsistent
#[test]
fn fuzz_verify_sync_differing_hashes_no_discrepancies_is_inconsistent() {
    let (env, client, admin) = setup_env();
    let provider_id = String::from_str(&env, "p-001");
    client.register_provider(
        &admin, &provider_id,
        &String::from_str(&env, "Hospital"),
        &EmrSystem::EpicFhir,
        &String::from_str(&env, "https://epic.example.com"),
        &DataFormat::FhirR4,
    );
    client.activate_provider(&admin, &provider_id);
    let exchange_id = String::from_str(&env, "ex-001");
    client.record_data_exchange(
        &admin, &exchange_id, &provider_id,
        &String::from_str(&env, "pat-001"),
        &ExchangeDirection::Import, &DataFormat::FhirR4,
        &String::from_str(&env, "Patient"),
        &String::from_str(&env, "hash"),
    );
    let disc: Vec<String> = Vec::new(&env);
    let ver = client.verify_sync(
        &admin,
        &String::from_str(&env, "ver-001"),
        &exchange_id,
        &String::from_str(&env, "hash_A"),
        &String::from_str(&env, "hash_B"),
        &disc,
    );
    assert!(!ver.is_consistent);
    let record = client.get_exchange(&exchange_id);
    assert_eq!(record.status, SyncStatus::PartialSuccess);
}

// get_patient_exchanges returns empty vec for unknown patient (no panic)
#[test]
fn fuzz_get_patient_exchanges_unknown_patient_returns_empty() {
    let (env, client, _) = setup_env();
    let exchanges = client.get_patient_exchanges(&String::from_str(&env, "unknown-patient"));
    assert_eq!(exchanges.len(), 0);
}

// get_provider_mappings returns empty vec for provider with no mappings (no panic)
#[test]
fn fuzz_get_provider_mappings_no_mappings_returns_empty() {
    let (env, client, admin) = setup_env();
    let provider_id = String::from_str(&env, "p-001");
    client.register_provider(
        &admin, &provider_id,
        &String::from_str(&env, "Hospital"),
        &EmrSystem::EpicFhir,
        &String::from_str(&env, "https://epic.example.com"),
        &DataFormat::FhirR4,
    );
    let mappings = client.get_provider_mappings(&provider_id);
    assert_eq!(mappings.len(), 0);
}
