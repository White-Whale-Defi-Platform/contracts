use cosmwasm_std::from_binary;
use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};
use cw_controllers::AdminError;

use white_whale::memory::LIST_SIZE_LIMIT;
use white_whale::treasury::msg::{ConfigResponse, ExecuteMsg, InstantiateMsg, QueryMsg};
use white_whale::treasury::state::{State, STATE};

use crate::contract::{execute, instantiate, query};
use crate::error::TreasuryError;

use super::common::TEST_CREATOR;

fn init_msg() -> InstantiateMsg {
    InstantiateMsg {}
}

#[test]
fn proper_initialization() {
    let mut deps = mock_dependencies(&[]);
    let msg = init_msg();
    let info = mock_info(TEST_CREATOR, &[]);

    let _res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

    let config: ConfigResponse =
        from_binary(&query(deps.as_ref(), mock_env(), QueryMsg::Config {}).unwrap()).unwrap();
    assert_eq!(0, config.dapps.len());
}

#[test]
fn test_update_admin() {
    let mut deps = mock_dependencies(&[]);
    let msg = init_msg();
    let info = mock_info(TEST_CREATOR, &[]);

    let _res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

    let msg = ExecuteMsg::SetAdmin {
        admin: String::from("addr0001"),
    };
    let info = mock_info("addr0001", &[]);
    // Call as non-admin, should fail
    match execute(deps.as_mut(), mock_env(), info, msg.clone()) {
        Ok(_) => panic!("Must return error"),
        Err(TreasuryError::Admin(AdminError::NotAdmin {})) => (),
        Err(_) => panic!("Unknown error"),
    }

    // Call as admin
    let info = mock_info(TEST_CREATOR, &[]);
    match execute(deps.as_mut(), mock_env(), info, msg.clone()) {
        Ok(_) => (),
        Err(_) => panic!("Should not error"),
    }
}

#[test]
fn test_add_dapp() {
    let mut deps = mock_dependencies(&[]);
    let msg = init_msg();
    let info = mock_info(TEST_CREATOR, &[]);
    let _res = instantiate(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();

    let msg = ExecuteMsg::AddDApp {
        dapp: "addr420".to_string(),
    };

    match execute(deps.as_mut(), mock_env(), info, msg) {
        Ok(_) => (),
        Err(_) => panic!("Unknown error"),
    }
    let config: ConfigResponse =
        from_binary(&query(deps.as_ref(), mock_env(), QueryMsg::Config {}).unwrap()).unwrap();
    assert_eq!(1, config.dapps.len());
    assert_eq!("addr420", config.dapps[0]);
}

#[test]
fn test_unsuccessful_add_dapp_limit_reached() {
    let mut deps = mock_dependencies(&[]);
    let msg = init_msg();
    let info = mock_info(TEST_CREATOR, &[]);
    let _res = instantiate(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();

    let state: State = STATE.load(&deps.storage).unwrap();
    assert_eq!(state, State { dapps: vec![] });

    for n in 0..LIST_SIZE_LIMIT + 1 {
        let mut dapp = "dappaddr".to_owned();
        let number = n.to_string().to_owned();
        dapp.push_str(&number);

        let msg = ExecuteMsg::AddDApp { dapp };

        match execute(deps.as_mut(), mock_env(), info.clone(), msg) {
            Ok(_) => {
                let state: State = STATE.load(&deps.storage).unwrap();
                assert!(state.dapps.len() <= LIST_SIZE_LIMIT);
            }
            Err(TreasuryError::DAppsLimitReached {}) => {
                let state: State = STATE.load(&deps.storage).unwrap();
                assert_eq!(state.dapps.len(), LIST_SIZE_LIMIT);
                ()
            } //expected at n > LIST_SIZE_LIMIT
            Err(e) => panic!("Unexpected error: {:?}", e),
        }
    }
}

#[test]
fn test_remove_dapp() {
    let mut deps = mock_dependencies(&[]);
    let msg = init_msg();
    let info = mock_info(TEST_CREATOR, &[]);
    let _res = instantiate(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();

    let msg = ExecuteMsg::AddDApp {
        dapp: "addr420".to_string(),
    };

    match execute(deps.as_mut(), mock_env(), info.clone(), msg) {
        Ok(_) => (),
        Err(_) => panic!("Unknown error"),
    }
    let config: ConfigResponse =
        from_binary(&query(deps.as_ref(), mock_env(), QueryMsg::Config {}).unwrap()).unwrap();
    assert_eq!(1, config.dapps.len());
    // now remove dapp again.
    let msg = ExecuteMsg::RemoveDApp {
        dapp: "addr420".to_string(),
    };
    match execute(deps.as_mut(), mock_env(), info, msg) {
        Ok(_) => (),
        Err(_) => panic!("Unknown error"),
    }
    // get dapp list and assert
    let config: ConfigResponse =
        from_binary(&query(deps.as_ref(), mock_env(), QueryMsg::Config {}).unwrap()).unwrap();
    assert_eq!(0, config.dapps.len());
}
