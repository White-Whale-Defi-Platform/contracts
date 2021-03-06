use cosmwasm_std::testing::{mock_env, mock_info};
use cosmwasm_std::Api;
use cosmwasm_std::DepsMut;

use crate::contract::{execute, instantiate};
use crate::state::{State, ARB_BASE_ASSET, STATE};

use terraswap::asset::AssetInfo;

use white_whale::deposit_info::ArbBaseAsset;

use crate::tests::common::{TEST_CREATOR, VAULT_CONTRACT};
use crate::tests::mock_querier::mock_dependencies;
use white_whale::peg_arb::msg::*;

use super::common::POOL_NAME;

pub(crate) fn instantiate_msg() -> InstantiateMsg {
    InstantiateMsg {
        vault_address: VAULT_CONTRACT.to_string(),
        seignorage_address: "seignorage".to_string(),
        asset_info: AssetInfo::NativeToken {
            denom: "uusd".to_string(),
        },
    }
}

/**
 * Mocks instantiation.
 */
pub fn mock_instantiate(mut deps: DepsMut) {
    let msg = InstantiateMsg {
        vault_address: VAULT_CONTRACT.to_string(),
        seignorage_address: "seignorage".to_string(),
        asset_info: AssetInfo::NativeToken {
            denom: "uusd".to_string(),
        },
    };

    let info = mock_info(TEST_CREATOR, &[]);
    let _res = instantiate(deps.branch(), mock_env(), info.clone(), msg)
        .expect("contract successfully handles InstantiateMsg");

    let add_pool_msg = ExecuteMsg::UpdatePools {
        to_add: Some(vec![(POOL_NAME.to_string(), "terraswap_pool".to_string())]),
        to_remove: None,
    };

    let _res = execute(deps, mock_env(), info, add_pool_msg).unwrap();
}

/**
 * Tests successful instantiation of the contract.
 */
#[test]
fn successful_initialization() {
    let mut deps = mock_dependencies(&[]);

    let msg = instantiate_msg();
    let info = mock_info(TEST_CREATOR, &[]);
    let res = instantiate(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();
    assert_eq!(0, res.messages.len());

    let state: State = STATE.load(&deps.storage).unwrap();
    assert_eq!(
        state,
        State {
            vault_address: deps.api.addr_validate(&VAULT_CONTRACT).unwrap(),
            seignorage_address: deps.api.addr_validate(&"seignorage").unwrap(),
        }
    );

    let base_asset: ArbBaseAsset = ARB_BASE_ASSET.load(&deps.storage).unwrap();
    assert_eq!(
        base_asset.asset_info,
        AssetInfo::NativeToken {
            denom: "uusd".to_string(),
        },
    );
}

#[test]
fn successful_set_admin() {
    let mut deps = mock_dependencies(&[]);
    mock_instantiate(deps.as_mut());

    // update admin
    let info = mock_info(TEST_CREATOR, &[]);
    let msg = ExecuteMsg::SetAdmin {
        admin: "new_admin".to_string(),
    };

    let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();
    assert_eq!(0, res.messages.len());
}

#[test]
fn successful_set_vault() {
    let mut deps = mock_dependencies(&[]);
    mock_instantiate(deps.as_mut());

    // update admin
    let info = mock_info(TEST_CREATOR, &[]);
    let msg = ExecuteMsg::SetVault {
        vault: "new_vault".to_string(),
    };

    let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();
    assert_eq!(0, res.messages.len());
}

#[test]
fn unsuccessful_set_vault() {
    let mut deps = mock_dependencies(&[]);
    mock_instantiate(deps.as_mut());

    // update admin
    let info = mock_info("someone", &[]);
    let msg = ExecuteMsg::SetVault {
        vault: "new_vault".to_string(),
    };

    let res = execute(deps.as_mut(), mock_env(), info, msg);
    match res {
        Ok(_) => panic!("caller is not admin, should error"),
        Err(_) => (),
    }
}
