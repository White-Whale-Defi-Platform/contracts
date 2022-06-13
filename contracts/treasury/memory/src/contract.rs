use cosmwasm_std::{entry_point, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult};

use crate::commands::*;
use white_whale::memory::error::MemoryError;
use crate::queries;
use crate::state::ADMIN;
use white_whale::memory::msg::{ExecuteMsg, InstantiateMsg, QueryMsg};

pub type MemoryResult = Result<Response, MemoryError>;

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    _msg: InstantiateMsg,
) -> MemoryResult {
    // Setup the admin as the creator of the contract
    ADMIN.set(deps, Some(info.sender))?;

    Ok(Response::default())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(deps: DepsMut, _env: Env, info: MessageInfo, msg: ExecuteMsg) -> MemoryResult {
    handle_message(deps, info, msg)
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::QueryAssets { names } => queries::query_assets(deps, env, names),
        QueryMsg::QueryContracts { names } => queries::query_contract(deps, env, names),
    }
}
