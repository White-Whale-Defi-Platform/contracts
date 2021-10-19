use cosmwasm_std::{to_binary, Addr, Coin, CosmosMsg, Decimal, StdResult, WasmMsg};

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::fmt;
use terraswap::asset::{AssetInfo};


#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InitMsg {
    pub trader: String,
    pub vault_address: String,
    pub seignorage_address: String,
    pub pool_address: String,
    pub asset_info: AssetInfo,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    TestMsg {},
    SendToVault {},
    AbovePeg {
        amount: Coin,
        slippage: Decimal,
        belief_price: Decimal,
    },
    BelowPeg {
        amount: Coin,
        slippage: Decimal,
        belief_price: Decimal,
    },
    SetAdmin {
        admin: String,
    },
    SetTrader {
        trader: String,
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
    AfterSuccessfulTradeCallback {},
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    Config {},
}
