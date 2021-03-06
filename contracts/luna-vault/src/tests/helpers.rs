use cosmwasm_std::testing::{mock_env, mock_info};
use cosmwasm_std::{to_binary, ReplyOn, Response, SubMsg, Uint128};
use cw20::Cw20ReceiveMsg;
use white_whale::luna_vault::msg::Cw20HookMsg;

use white_whale::ust_vault::msg::CallbackMsg;

use crate::commands::receive_cw20;
use crate::error::LunaVaultError;
use crate::flashloan::encapsulate_payload;
use crate::helpers::get_treasury_fee;
use crate::tests::common::TEST_CREATOR;
use crate::tests::instantiate::{mock_instantiate, TREASURY_FEE};
use crate::tests::mock_querier::mock_dependencies;

#[test]
fn test_encapsulate_payload() {
    let mut deps = mock_dependencies(&[]);
    mock_instantiate(deps.as_mut());

    let response = Response::new();
    let loan_fee = Uint128::new(1000);

    let res = encapsulate_payload(deps.as_ref(), mock_env(), response, loan_fee).unwrap();
    assert_eq!(res.messages.len(), 1);

    let env = mock_env();

    assert_eq!(
        res.messages,
        vec![SubMsg {
            id: 0,
            gas_limit: None,
            reply_on: ReplyOn::Never,
            msg: CallbackMsg::AfterTrade { loan_fee }
                .to_cosmos_msg(&env.contract.address)
                .unwrap()
        },]
    );
}

#[test]
fn unsuccessful_receive_cw20_unauthorized() {
    let mut deps = mock_dependencies(&[]);
    mock_instantiate(deps.as_mut());
    let info = mock_info(TEST_CREATOR, &[]);

    let cw20_msg = Cw20ReceiveMsg {
        sender: "unauthorized".to_string(),
        amount: Default::default(),
        msg: to_binary(&Cw20HookMsg::Unbond {}).unwrap(),
    };

    let res = receive_cw20(deps.as_mut(), mock_env(), info, cw20_msg);
    println!("{:?}", res);
    match res {
        Err(LunaVaultError::Unauthorized {}) => (),
        _ => panic!("Must return LunaVaultError::Unauthorized"),
    }
}

#[test]
fn test_get_treasury_fee() {
    let mut deps = mock_dependencies(&[]);
    mock_instantiate(deps.as_mut());

    let amount = Uint128::new(1000);

    let treasury_fee = get_treasury_fee(deps.as_ref(), amount).unwrap();
    assert_eq!(
        treasury_fee,
        amount / Uint128::new(u128::from(TREASURY_FEE))
    );
}
