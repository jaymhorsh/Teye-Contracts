#![cfg(test)]

use crate::{
    types::{Gender, ObservationStatus},
    FhirContract, FhirContractClient,
};
use soroban_sdk::{Address, Bytes, Env, String};

#[test]
fn test_initialize() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(FhirContract, ());
    let client = FhirContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);

    client.initialize(&admin);
}

#[test]
fn test_register_and_get_resource() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(FhirContract, ());
    let client = FhirContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);

    client.initialize(&admin);

    let id = String::from_str(&env, "patient-1");
    let payload = Bytes::from_slice(&env, b"{\"resourceType\":\"Patient\",\"id\":\"patient-1\"}");

    client.register_resource(&admin, &id, &payload);
    assert_eq!(client.get_resource(&id), payload);
}

#[test]
fn test_update_resource() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(FhirContract, ());
    let client = FhirContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);

    client.initialize(&admin);

    let id = String::from_str(&env, "patient-1");
    let payload1 = Bytes::from_slice(&env, b"v1");
    let payload2 = Bytes::from_slice(&env, b"v2");

    client.register_resource(&admin, &id, &payload1);
    client.update_resource(&admin, &id, &payload2);
    assert_eq!(client.get_resource(&id), payload2);
}

#[test]
fn test_delete_resource() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(FhirContract, ());
    let client = FhirContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);

    client.initialize(&admin);

    let id = String::from_str(&env, "patient-1");
    let payload = Bytes::from_slice(&env, b"v1");

    client.register_resource(&admin, &id, &payload);
    client.delete_resource(&admin, &id);
    
    let result = client.try_get_resource(&id);
    assert!(result.is_err());
}

#[test]
fn test_patient_validation() {
    let env = Env::default();
    let contract_id = env.register(FhirContract, ());
    let client = FhirContractClient::new(&env, &contract_id);

    let id = String::from_str(&env, "p1");
    let identifier = String::from_str(&env, "id1");
    let name = String::from_str(&env, "John Doe");
    let gender = Gender::Male;
    let birth_date = 123456789;

    let patient = client.create_patient(&id, &identifier, &name, &gender, &birth_date);
    assert!(client.validate_patient(&patient));
}
