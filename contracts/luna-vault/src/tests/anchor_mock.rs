use cosmwasm_bignumber::{Decimal256, Uint256};
use cosmwasm_std::{
    attr, from_binary, to_binary, Addr, BankMsg, Binary, Coin, CosmosMsg, Empty, Response, Uint128,
    WasmMsg,
};
use cw20::Cw20ExecuteMsg;
use cw20::Cw20ReceiveMsg;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use terra_multi_test::{Contract, ContractWrapper};
use terraswap::asset::{Asset, AssetInfo};

use white_whale::query::anchor::{
    AnchorQuery, EpochStateResponse, UnbondRequestsResponse, WithdrawableUnbondedResponse,
};

use crate::contract::VaultResult;
use crate::error::LunaVaultError;

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct MockInstantiateMsg {}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct PingMsg {
    pub payload: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum MockExecuteMsg {
    Receive(Cw20ReceiveMsg),
    DepositStable {},
    RedeemStable { burn_amount: Uint128 },
}

// We define a custom struct for each query response
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct PoolResponse {
    pub assets: [Asset; 2],
    pub total_share: Uint128,
}

// We define a custom struct for each query response
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct PairResponse {
    pub asset_infos: [AssetInfo; 2],
    pub contract_addr: String,
    pub liquidity_token: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Cw20HookMsg {
    /// Return stable coins to a user
    /// according to exchange rate
    RedeemStable {},
}

#[allow(dead_code)]
pub fn contract_anchor_mock() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new(
        |deps, _, info, msg: MockExecuteMsg| -> VaultResult<Response> {
            match msg {
                MockExecuteMsg::Receive(Cw20ReceiveMsg {
                    sender: _,
                    amount,
                    msg,
                }) => match from_binary(&msg) {
                    Ok(Cw20HookMsg::RedeemStable {}) => {
                        let redeem_amount = Uint256::from(amount) * Decimal256::percent(120);
                        Ok(Response::new()
                            .add_messages(vec![
                                CosmosMsg::Wasm(WasmMsg::Execute {
                                    contract_addr: deps
                                        .api
                                        .addr_humanize(
                                            &deps
                                                .api
                                                .addr_canonicalize(&String::from("Contract #2"))?,
                                        )?
                                        .to_string(),
                                    funds: vec![],
                                    msg: to_binary(&Cw20ExecuteMsg::Burn { amount })?,
                                }),
                                CosmosMsg::Bank(BankMsg::Send {
                                    to_address: info.sender.to_string(),
                                    amount: vec![Coin {
                                        denom: "uusd".to_string(),
                                        amount: redeem_amount.into(),
                                    }],
                                }),
                            ])
                            .add_attributes(vec![
                                attr("action", "redeem_stable"),
                                attr("burn_amount", amount),
                                attr("redeem_amount", redeem_amount),
                            ]))
                    }
                    _ => Err(LunaVaultError::generic_err("Unauthorized")),
                },
                MockExecuteMsg::DepositStable {} => {
                    // Check base denom deposit
                    let deposit_amount: Uint256 = info
                        .funds
                        .iter()
                        .find(|c| c.denom == *"uusd")
                        .map(|c| Uint256::from(c.amount))
                        .unwrap_or_else(Uint256::zero);
                    // Get Mint amount
                    let mint_amount = deposit_amount / Decimal256::percent(120);
                    // Perform a mint from the contract
                    Ok(Response::new()
                        .add_message(CosmosMsg::Wasm(WasmMsg::Execute {
                            contract_addr: deps
                                .api
                                .addr_humanize(
                                    &deps.api.addr_canonicalize(&String::from("Contract #2"))?,
                                )?
                                .to_string(),
                            funds: vec![],
                            msg: to_binary(&Cw20ExecuteMsg::Mint {
                                recipient: info.sender.to_string(),
                                amount: mint_amount.into(),
                            })?,
                        }))
                        .add_attributes(vec![
                            attr("action", "deposit_stable"),
                            attr("depositor", info.sender),
                            attr("mint_amount", mint_amount),
                            attr("deposit_amount", deposit_amount),
                        ]))
                }
                MockExecuteMsg::RedeemStable { burn_amount } => {
                    let redeem_amount = Uint256::from(burn_amount) * Decimal256::percent(120);
                    Ok(Response::new()
                        .add_messages(vec![
                            CosmosMsg::Wasm(WasmMsg::Execute {
                                contract_addr: deps
                                    .api
                                    .addr_humanize(
                                        &deps
                                            .api
                                            .addr_canonicalize(&String::from("Contract #2"))?,
                                    )?
                                    .to_string(),
                                funds: vec![],
                                msg: to_binary(&Cw20ExecuteMsg::Burn {
                                    amount: burn_amount,
                                })?,
                            }),
                            CosmosMsg::Bank(BankMsg::Send {
                                to_address: info.sender.to_string(),
                                amount: vec![Coin {
                                    denom: "uusd".to_string(),
                                    amount: redeem_amount.into(),
                                }],
                            }),
                        ])
                        .add_attributes(vec![
                            attr("action", "redeem_stable"),
                            attr("burn_amount", burn_amount),
                            attr("redeem_amount", redeem_amount),
                        ]))
                }
            }
        },
        |_, _, _, _: MockInstantiateMsg| -> VaultResult<Response> { Ok(Response::default()) },
        |deps, _, msg: AnchorQuery| -> VaultResult<Binary> {
            match msg {
                AnchorQuery::EpochState {
                    distributed_interest: _,
                    block_height: _,
                } => Ok(to_binary(&mock_epoch_state())?),
                AnchorQuery::UnbondRequests { address: addr } => Ok(to_binary(
                    &mock_unbond_requests(deps.api.addr_validate(&*addr)?),
                )?),
                AnchorQuery::WithdrawableUnbonded { .. } => {
                    Ok(to_binary(&mock_withdrawable_unbonded())?)
                }
            }
        },
    );
    Box::new(contract)
}

pub fn mock_epoch_state() -> EpochStateResponse {
    let epoch_state: EpochStateResponse = EpochStateResponse {
        exchange_rate: Decimal256::percent(120),
        aterra_supply: Uint256::from(1000000u64),
    };
    epoch_state
}

pub fn mock_unbond_requests(address: Addr) -> UnbondRequestsResponse {
    let unbond_requests: UnbondRequestsResponse = UnbondRequestsResponse {
        address: address.to_string(),
        requests: vec![],
    };
    unbond_requests
}

pub fn mock_withdrawable_unbonded() -> WithdrawableUnbondedResponse {
    let withdrawable_unbonded: WithdrawableUnbondedResponse = WithdrawableUnbondedResponse {
        withdrawable: Uint128::zero(),
    };
    withdrawable_unbonded
}
