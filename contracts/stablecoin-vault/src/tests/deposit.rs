use cosmwasm_std::testing::{mock_env, mock_info};
use cosmwasm_std::{coins, from_binary, to_binary, MessageInfo, ReplyOn, SubMsg, WasmMsg};
use cosmwasm_std::{Api, Decimal, Uint128};

use crate::contract::{execute, instantiate, query};
use crate::state::{State, STATE};
use cw20::MinterResponse;

use terraswap::token::InstantiateMsg as TokenInstantiateMsg;
use white_whale::fee::*;
use white_whale::ust_vault::msg::VaultQueryMsg as QueryMsg;
use white_whale::ust_vault::msg::*;

use crate::tests::common::{ARB_CONTRACT, TEST_CREATOR, };

use crate::tests::mock_querier::mock_dependencies;
use crate::tests::instantiate::mock_instantiate;

const INSTANTIATE_REPLY_ID: u8 = 1u8;
use terraswap::asset::{Asset, AssetInfo};


#[test]
pub fn mock_deposit() {

    let mut deps = mock_dependencies(&[]);
    mock_instantiate(deps.as_mut());
    let env = mock_env();
    let msg = ExecuteMsg::ProvideLiquidity {
        asset: Asset {
            info: AssetInfo::NativeToken{
                denom: String::from("uusd")
            },
            amount: Uint128::new(1000)
        }
        };


    let info = mock_info(TEST_CREATOR, &coins(1000, "uusd"));
    let execute_res = execute(deps.as_mut(), env, info, msg);

}
/**
 * Tests successful instantiation of the contract.
 */
#[test]
fn successful_initialization() {
    let mut deps = mock_dependencies(&[]);
    mock_instantiate(deps.as_mut());
    let info = mock_info(TEST_CREATOR, &[]);

    let state: State = STATE.load(&deps.storage).unwrap();
    assert_eq!(
        state,
        State {
            anchor_money_market_address: deps.api.addr_canonicalize("test_mm").unwrap(),
            aust_address: deps.api.addr_canonicalize("test_aust").unwrap(),
            profit_check_address: deps.api.addr_canonicalize("test_profit_check").unwrap(),
            whitelisted_contracts: vec![],
            allow_non_whitelisted: false
        }
    );

    let msg = ExecuteMsg::AddToWhitelist {
        contract_addr: ARB_CONTRACT.to_string(),
    };
    let _res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();
    let state: State = STATE.load(&deps.storage).unwrap();
    assert_eq!(
        state.whitelisted_contracts[0],
        deps.api.addr_canonicalize(&ARB_CONTRACT).unwrap(),
    );
}

/**
 * Tests updating the fees of the contract.
 */
#[test]
fn successful_update_fee() {
    let mut deps = mock_dependencies(&[]);
    mock_instantiate(deps.as_mut());

    // update fees
    let info = mock_info(TEST_CREATOR, &[]);
    let msg = ExecuteMsg::SetFee {
        warchest_fee: Some(Fee {
            share: Decimal::percent(2),
        }),
        flash_loan_fee: Some(Fee {
            share: Decimal::percent(2),
        }),
    };

    let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();
    assert_eq!(0, res.messages.len());

    // it worked, let's query the fee
    let res = query(deps.as_ref(), mock_env(), QueryMsg::Fees {}).unwrap();
    let fee_response: FeeResponse = from_binary(&res).unwrap();
    let fees: VaultFee = fee_response.fees;
    assert_eq!(Decimal::percent(2), fees.warchest_fee.share);
}

#[test]
fn successfull_set_admin() {
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
fn test_init_with_non_default_vault_lp_token() {
    let mut deps = mock_dependencies(&[]);

    let custom_token_name = String::from("My LP Token");
    let custom_token_symbol = String::from("MyLP");

    let msg = InstantiateMsg {
        anchor_money_market_address: "test_mm".to_string(),
        aust_address: "test_aust".to_string(),
        profit_check_address: "test_profit_check".to_string(),
        warchest_addr: "warchest".to_string(),
        asset_info: AssetInfo::NativeToken {
            denom: "uusd".to_string(),
        },
        token_code_id: 0u64,
        warchest_fee: Decimal::percent(10u64),
        flash_loan_fee: Decimal::permille(5u64),
        stable_cap: Uint128::from(100_000_000u64),
        vault_lp_token_name: Some(custom_token_name.clone()),
        vault_lp_token_symbol: Some(custom_token_symbol.clone()),
    };

    // Prepare mock env
    let env = mock_env();
    let info = MessageInfo {
        sender: deps.api.addr_validate("creator").unwrap(),
        funds: vec![],
    };

    let res = instantiate(deps.as_mut(), env.clone(), info, msg.clone()).unwrap();
    // Ensure we have 1 message
    assert_eq!(1, res.messages.len());
    // Verify the message is the one we expect but also that our custom provided token name and symbol were taken into account.
    assert_eq!(
        res.messages,
        vec![SubMsg {
            // Create LP token
            msg: WasmMsg::Instantiate {
                admin: None,
                code_id: msg.token_code_id,
                msg: to_binary(&TokenInstantiateMsg {
                    name: custom_token_name.to_string(),
                    symbol: custom_token_symbol.to_string(),
                    decimals: 6,
                    initial_balances: vec![],
                    mint: Some(MinterResponse {
                        minter: env.contract.address.to_string(),
                        cap: None,
                    }),
                })
                .unwrap(),
                funds: vec![],
                label: "White Whale Stablecoin Vault LP".to_string(),
            }
            .into(),
            gas_limit: None,
            id: u64::from(INSTANTIATE_REPLY_ID),
            reply_on: ReplyOn::Success,
        }]
    );
}
