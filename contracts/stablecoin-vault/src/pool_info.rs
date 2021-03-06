use cosmwasm_std::{Addr, Deps, StdResult, Uint128};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use terraswap::asset::{Asset, AssetInfo, AssetInfoRaw};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct PoolInfo {
    pub asset_infos: [AssetInfo; 2],
    pub contract_addr: Addr,
    pub liquidity_token: Addr,
    pub stable_cap: Uint128,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct PoolInfoRaw {
    pub asset_infos: [AssetInfoRaw; 2],
    pub contract_addr: Addr,
    pub liquidity_token: Addr,
    pub stable_cap: Uint128,
}

impl PoolInfoRaw {
    pub fn to_normal(&self, deps: Deps) -> StdResult<PoolInfo> {
        Ok(PoolInfo {
            liquidity_token: self.liquidity_token.clone(),
            stable_cap: self.stable_cap,
            contract_addr: self.contract_addr.clone(),
            asset_infos: [
                self.asset_infos[0].to_normal(deps.api)?,
                self.asset_infos[1].to_normal(deps.api)?,
            ],
        })
    }

    pub fn query_pools(&self, deps: Deps, contract_addr: Addr) -> StdResult<[Asset; 2]> {
        let info_0: AssetInfo = self.asset_infos[0].to_normal(deps.api)?;
        let info_1: AssetInfo = self.asset_infos[1].to_normal(deps.api)?;
        Ok([
            Asset {
                amount: info_0.query_pool(&deps.querier, deps.api, contract_addr.clone())?,
                info: info_0,
            },
            Asset {
                amount: info_1.query_pool(&deps.querier, deps.api, contract_addr)?,
                info: info_1,
            },
        ])
    }
}
