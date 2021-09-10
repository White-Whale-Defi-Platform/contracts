use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use cosmwasm_std::{Decimal, Uint128};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InstantiateMsg {
    pub whale_token_addr: String,
    pub whale_pair_addr: String,
    pub anchor_money_market_addr: String,
    pub aust_addr: String,
    pub anchor_deposit_threshold: Uint128,
    pub anchor_withdraw_threshold: Uint128,
    pub anchor_deposit_ratio: Decimal
}
