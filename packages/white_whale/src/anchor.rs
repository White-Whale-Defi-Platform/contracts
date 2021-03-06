use std::fmt;

use cosmwasm_std::{
    to_binary, Addr, Coin, CosmosMsg, Deps, ReplyOn, Response, StdError, StdResult, SubMsg,
    Uint128, WasmMsg,
};
use cw20::Cw20ExecuteMsg;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::denom::UST_DENOM;
use crate::tax::deduct_tax;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum AnchorMsg {
    DepositStable {},
    RedeemStable {},
    Unbond {},
    WithdrawUnbonded {},
}

pub fn try_deposit_to_anchor<T: Clone + fmt::Debug + PartialEq + JsonSchema>(
    anchor_money_market_address: String,
    amount: Coin,
) -> StdResult<Response<T>> {
    if amount.denom != UST_DENOM {
        return Err(StdError::generic_err(
            "Wrong currency. Only UST (denom: uusd) is supported.",
        ));
    }

    let msg = CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: anchor_money_market_address,
        msg: to_binary(&AnchorMsg::DepositStable {})?,
        funds: vec![amount],
    });

    Ok(Response::new().add_message(msg))
}

pub fn try_deposit_to_anchor_as_submsg<T: Clone + fmt::Debug + PartialEq + JsonSchema>(
    anchor_money_market_address: String,
    amount: Coin,
    id: u64,
) -> StdResult<Response<T>> {
    if amount.denom != UST_DENOM {
        return Err(StdError::generic_err(
            "Wrong currency. Only UST (denom: uusd) is supported.",
        ));
    }

    let msg = CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: anchor_money_market_address,
        msg: to_binary(&AnchorMsg::DepositStable {})?,
        funds: vec![amount],
    });

    Ok(Response::new().add_submessage(SubMsg {
        msg,
        gas_limit: None,
        id,
        reply_on: ReplyOn::Success,
    }))
}

pub fn anchor_deposit_msg<T: Clone + fmt::Debug + PartialEq + JsonSchema>(
    deps: Deps,
    anchor_money_market_address: Addr,
    amount: Coin,
) -> StdResult<CosmosMsg<T>> {
    if amount.denom != UST_DENOM {
        return Err(StdError::generic_err(
            "Wrong currency. Only UST (denom: uusd) is supported.",
        ));
    }

    Ok(CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: anchor_money_market_address.to_string(),
        msg: to_binary(&AnchorMsg::DepositStable {})?,
        funds: vec![deduct_tax(deps, amount)?],
    }))
}

pub fn anchor_withdraw_msg<T: Clone + fmt::Debug + PartialEq + JsonSchema>(
    aust_address: Addr,
    anchor_money_market_address: Addr,
    amount: Uint128,
) -> StdResult<CosmosMsg<T>> {
    Ok(CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: aust_address.to_string(),
        msg: to_binary(&Cw20ExecuteMsg::Send {
            contract: anchor_money_market_address.to_string(),
            amount,
            msg: to_binary(&AnchorMsg::RedeemStable {})?,
        })?,
        funds: vec![],
    }))
}

pub fn anchor_bluna_unbond_msg<T: Clone + fmt::Debug + PartialEq + JsonSchema>(
    bluna_address: Addr,
    bluna_hub_address: Addr,
    amount: Uint128,
) -> StdResult<CosmosMsg<T>> {
    Ok(CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: bluna_address.to_string(),
        msg: to_binary(&Cw20ExecuteMsg::Send {
            contract: bluna_hub_address.to_string(),
            amount,
            msg: to_binary(&AnchorMsg::Unbond {})?,
        })?,
        funds: vec![],
    }))
}

pub fn anchor_withdraw_unbonded_msg<T: Clone + fmt::Debug + PartialEq + JsonSchema>(
    bluna_hub_address: Addr,
) -> StdResult<CosmosMsg<T>> {
    Ok(CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: bluna_hub_address.to_string(),
        msg: to_binary(&AnchorMsg::WithdrawUnbonded {})?,
        funds: vec![],
    }))
}
