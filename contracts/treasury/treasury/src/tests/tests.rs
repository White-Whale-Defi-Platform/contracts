use crate::contract::{execute, instantiate, query};
use crate::error::TreasuryError;
use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};
use cosmwasm_std::{from_binary};
use cw_controllers::AdminError;
use white_whale::treasury::msg::{ConfigResponse, ExecuteMsg, InstantiateMsg, QueryMsg};

fn init_msg() -> InstantiateMsg {
    InstantiateMsg {}
}

#[test]
fn proper_initialization() {
    let mut deps = mock_dependencies(&[]);
    let msg = init_msg();
    let info = mock_info("addr0000", &[]);

    let _res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

    let config: ConfigResponse =
        from_binary(&query(deps.as_ref(), mock_env(), QueryMsg::Config {}).unwrap()).unwrap();
    assert_eq!(0, config.traders.len());
}

#[test]
fn test_update_admin() {
    let mut deps = mock_dependencies(&[]);
    let msg = init_msg();
    let info = mock_info("addr0000", &[]);

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
    let info = mock_info("addr0000", &[]);
    match execute(deps.as_mut(), mock_env(), info, msg.clone()) {
        Ok(_) => (),
        Err(_) => panic!("Should not error"),
    }
}

#[test]
fn test_add_trader() {
    let mut deps = mock_dependencies(&[]);
    let msg = init_msg();
    let info = mock_info("addr0000", &[]);
    let _res = instantiate(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();

    let msg = ExecuteMsg::AddTrader {
        trader: "addr420".to_string(),
    };

    match execute(deps.as_mut(), mock_env(), info, msg) {
        Ok(_) => (),
        Err(_) => panic!("Unknown error"),
    }
    let config: ConfigResponse =
        from_binary(&query(deps.as_ref(), mock_env(), QueryMsg::Config {}).unwrap()).unwrap();
    assert_eq!(1, config.traders.len());
}

#[test]
fn test_remove_trader() {
    let mut deps = mock_dependencies(&[]);
    let msg = init_msg();
    let info = mock_info("addr0000", &[]);
    let _res = instantiate(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();

    let msg = ExecuteMsg::AddTrader {
        trader: "addr420".to_string(),
    };

    match execute(deps.as_mut(), mock_env(), info.clone(), msg) {
        Ok(_) => (),
        Err(_) => panic!("Unknown error"),
    }
    let config: ConfigResponse =
        from_binary(&query(deps.as_ref(), mock_env(), QueryMsg::Config {}).unwrap()).unwrap();
    assert_eq!(1, config.traders.len());
    // now remove trader again.
    let msg = ExecuteMsg::RemoveTrader {
        trader: "addr420".to_string(),
    };
    match execute(deps.as_mut(), mock_env(), info, msg) {
        Ok(_) => (),
        Err(_) => panic!("Unknown error"),
    }
    // get trader list and assert
    let config: ConfigResponse =
        from_binary(&query(deps.as_ref(), mock_env(), QueryMsg::Config {}).unwrap()).unwrap();
    assert_eq!(0, config.traders.len());
}
