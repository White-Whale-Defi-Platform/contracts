use crate::treasury::dapp_base::msg::{BaseExecuteMsg, BaseQueryMsg};
use cosmwasm_std::{Decimal, Uint128};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use terra_rust_script_derive::contract;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema, contract)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    /// dApp base messages that handle updating the config and addressbook
    Base(BaseExecuteMsg),
    /// Constructs a provide liquidity msg and forwards it to the treasury
    /// Calculates the required asset amount for the second asset in the pool.
    ProvideLiquidity {
        pool_id: String,
        main_asset_id: String,
        amount: Uint128,
    },
    /// Constructs a provide liquidity msg and forwards it to the treasury.
    DetailedProvideLiquidity {
        assets: Vec<(String, Uint128)>,
        pool_id: String,
        slippage_tolerance: Option<Decimal>,
    },
    /// Constructs a withdraw liquidity msg and forwards it to the treasury
    WithdrawLiquidity {
        lp_token_id: String,
        amount: Uint128,
    },
    /// Constructs a swap msg and forwards it to the treasury
    SwapAsset {
        offer_id: String,
        pool_id: String,
        amount: Uint128,
        max_spread: Option<Decimal>,
        belief_price: Option<Decimal>,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema, contract)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    /// Handles all the base query msgs
    Base(BaseQueryMsg),
}
