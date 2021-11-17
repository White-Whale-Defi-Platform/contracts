use crate::fee::{Fee, VaultFee};
use cosmwasm_std::{
    to_binary, Addr, Binary, Coin, CosmosMsg, Decimal, StdResult, Uint128, WasmMsg,
};
use cw20::Cw20ReceiveMsg;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::fmt;
use terraswap::asset::{Asset, AssetInfo};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InstantiateMsg {
    pub anchor_money_market_address: String,
    pub aust_address: String,
    pub profit_check_address: String,
    pub warchest_addr: String,
    pub asset_info: AssetInfo,
    pub token_code_id: u64,
    pub warchest_fee: Decimal,
    pub flash_loan_fee: Decimal,
    pub stable_cap: Uint128,
    pub vault_lp_token_name: Option<String>,
    pub vault_lp_token_symbol: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    Receive(Cw20ReceiveMsg),
    ProvideLiquidity {
        asset: Asset,
    },
    SetStableCap {
        stable_cap: Uint128,
    },
    SetFee {
        flash_loan_fee: Option<Fee>,
        warchest_fee: Option<Fee>,
    },
    SetAdmin {
        admin: String,
    },
    AddToWhitelist {
        contract_addr: String,
    },
    RemoveFromWhitelist {
        contract_addr: String,
    },
    UpdateState {
        anchor_money_market_address: Option<String>,
        aust_address: Option<String>,
        profit_check_address: Option<String>,
        allow_non_whitelisted: Option<bool>,
    },
    FlashLoan {
        payload: FlashLoanPayload,
    },
    Callback(CallbackMsg),
}

// Modified from
// https://github.com/CosmWasm/cosmwasm-plus/blob/v0.2.3/packages/cw20/src/receiver.rs#L15
impl CallbackMsg {
    pub fn to_cosmos_msg<T: Clone + fmt::Debug + PartialEq + JsonSchema>(
        &self,
        contract_addr: &Addr,
    ) -> StdResult<CosmosMsg<T>> {
        Ok(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: String::from(contract_addr),
            msg: to_binary(&ExecuteMsg::Callback(self.clone()))?,
            funds: vec![],
        }))
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum CallbackMsg {
    AfterSuccessfulLoanCallback {},
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct PoolResponse {
    pub assets: [Asset; 2],
    pub total_value_in_ust: Uint128,
    pub total_share: Uint128,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct FlashLoanPayload {
    pub requested_asset: Asset,
    pub callback: Binary,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum VaultQueryMsg {
    Config {},
    State {},
    Pool {},
    Fees {},
    EstimateWithdrawFee { amount: Uint128 },
    VaultValue {},
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct DepositResponse {
    pub deposit: Uint128,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct ValueResponse {
    pub total_ust_value: Uint128,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct FeeResponse {
    pub fees: VaultFee,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct EstimateDepositFeeResponse {
    pub fee: Vec<Coin>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct EstimateWithdrawFeeResponse {
    pub fee: Vec<Coin>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct StateResponse {
    pub anchor_money_market_address: String,
    pub aust_address: String,   
    pub profit_check_address: String,
    pub allow_non_whitelisted: bool,
}
