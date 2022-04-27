use crate::treasury::dapp_base::msg::{BaseExecuteMsg, BaseQueryMsg};
use cosmwasm_std::Uint128;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    Base(BaseExecuteMsg),
    // Add dapp-specific messages here
    DepositStable { deposit_amount: Uint128 },
    RedeemStable { withdraw_amount: Uint128 },
    Unbond { bluna_amount: Uint128 },
    WithdrawUnbonded {},
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    Base(BaseQueryMsg),
    // Add dapp-specific queries here
}
