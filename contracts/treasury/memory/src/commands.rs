use cosmwasm_std::{Addr, DepsMut, MessageInfo, Response, StdResult};
use terraswap::asset::AssetInfo;

use crate::contract::MemoryResult;
use crate::state::*;
use white_whale::memory::msg::ExecuteMsg;

/// Handles the common base execute messages
pub fn handle_message(deps: DepsMut, info: MessageInfo, message: ExecuteMsg) -> MemoryResult {
    match message {
        ExecuteMsg::SetAdmin { admin } => set_admin(deps, info, admin),
        ExecuteMsg::UpdateContractAddresses { to_add, to_remove } => {
            update_contract_addresses(deps, info, to_add, to_remove)
        }
        ExecuteMsg::UpdateAssetAddresses { to_add, to_remove } => {
            update_asset_addresses(deps, info, to_add, to_remove)
        }
    }
}

//----------------------------------------------------------------------------------------
//  GOVERNANCE CONTROLLED SETTERS
//----------------------------------------------------------------------------------------

/// Adds, updates or removes provided addresses.
pub fn update_contract_addresses(
    deps: DepsMut,
    msg_info: MessageInfo,
    to_add: Vec<(String, String)>,
    to_remove: Vec<String>,
) -> MemoryResult {
    // Only Admin can call this method
    ADMIN.assert_admin(deps.as_ref(), &msg_info.sender)?;

    for (name, new_address) in to_add.into_iter() {
        // validate addr
        let addr = deps.as_ref().api.addr_validate(&new_address)?;
        // Update function for new or existing keys
        let insert = |_| -> StdResult<Addr> { Ok(addr) };
        CONTRACT_ADDRESSES.update(deps.storage, name.as_str(), insert)?;
    }

    for name in to_remove {
        CONTRACT_ADDRESSES.remove(deps.storage, name.as_str());
    }

    Ok(Response::new().add_attribute("action", "updated contract addressses"))
}

/// Adds, updates or removes provided addresses.
pub fn update_asset_addresses(
    deps: DepsMut,
    msg_info: MessageInfo,
    to_add: Vec<(String, AssetInfo)>,
    to_remove: Vec<String>,
) -> MemoryResult {
    // Only Admin can call this method
    ADMIN.assert_admin(deps.as_ref(), &msg_info.sender)?;

    for (name, new_address) in to_add.into_iter() {
        // Update function for new or existing keys
        let insert = |_| -> StdResult<AssetInfo> { Ok(new_address) };
        ASSET_ADDRESSES.update(deps.storage, name.as_str(), insert)?;
    }

    for name in to_remove {
        ASSET_ADDRESSES.remove(deps.storage, name.as_str());
    }

    Ok(Response::new().add_attribute("action", "updated asset addresses"))
}

pub fn set_admin(deps: DepsMut, info: MessageInfo, admin: String) -> MemoryResult {
    ADMIN.assert_admin(deps.as_ref(), &info.sender)?;

    let admin_addr = deps.api.addr_validate(&admin)?;
    let previous_admin = ADMIN.get(deps.as_ref())?.unwrap();
    ADMIN.execute_update_admin(deps, info, Some(admin_addr))?;
    Ok(Response::default()
        .add_attribute("previous admin", previous_admin)
        .add_attribute("admin", admin))
}
