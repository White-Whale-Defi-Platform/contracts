#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::Order::Ascending;
use cosmwasm_std::{
    to_binary, Addr, Binary, CosmosMsg, Deps, DepsMut, Empty, Env, MessageInfo, Order, Response,
    StdResult, Uint128,
};
use cw2::{get_contract_version, set_contract_version};
use semver::Version;
use terraswap::asset::AssetInfo;

use white_whale::memory::LIST_SIZE_LIMIT;
use white_whale::query::terraswap::query_asset_balance;
use white_whale::treasury::msg::{
    ConfigResponse, ExecuteMsg, HoldingValueResponse, InstantiateMsg, MigrateMsg, QueryMsg,
    TotalValueResponse,
};
use white_whale::treasury::state::{State, ADMIN, STATE, VAULT_ASSETS};
use white_whale::treasury::vault_assets::{get_identifier, VaultAsset};

use crate::error::TreasuryError;

type TreasuryResult = Result<Response, TreasuryError>;

/*
    The treasury is the bank account of the protocol. It owns the liquidity and acts as a proxy contract.
    Whitelisted dApps construct messages for this contract. The dApps are controlled by Governance.
*/

// version info for migration info
const CONTRACT_NAME: &str = "crates.io:treasury";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    _msg: InstantiateMsg,
) -> TreasuryResult {
    // Use CW2 to set the contract version, this is needed for migrations
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    STATE.save(deps.storage, &State { dapps: vec![] })?;
    let admin_addr = Some(info.sender);
    ADMIN.set(deps, admin_addr)?;

    Ok(Response::default())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(deps: DepsMut, _env: Env, info: MessageInfo, msg: ExecuteMsg) -> TreasuryResult {
    match msg {
        ExecuteMsg::DAppAction { msgs } => execute_action(deps, info, msgs),
        ExecuteMsg::SetAdmin { admin } => {
            let admin_addr = deps.api.addr_validate(&admin)?;
            let previous_admin = ADMIN.get(deps.as_ref())?.unwrap();
            ADMIN.execute_update_admin(deps, info, Some(admin_addr))?;
            Ok(Response::default()
                .add_attribute("previous admin", previous_admin)
                .add_attribute("admin", admin))
        }
        ExecuteMsg::AddDApp { dapp } => add_dapp(deps, info, dapp),
        ExecuteMsg::RemoveDApp { dapp } => remove_dapp(deps, info, dapp),
        ExecuteMsg::UpdateAssets { to_add, to_remove } => {
            update_assets(deps, info, to_add, to_remove)
        }
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(deps: DepsMut, _env: Env, _msg: MigrateMsg) -> TreasuryResult {
    let version: Version = CONTRACT_VERSION.parse()?;
    let storage_version: Version = get_contract_version(deps.storage)?.version.parse()?;

    if storage_version < version {
        set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    }
    Ok(Response::default())
}

/// Executes actions forwarded by whitelisted contracts
/// This contracts acts as a proxy contract for the dApps
pub fn execute_action(
    deps: DepsMut,
    msg_info: MessageInfo,
    msgs: Vec<CosmosMsg<Empty>>,
) -> TreasuryResult {
    let state = STATE.load(deps.storage)?;
    if !state
        .dapps
        .contains(&deps.api.addr_validate(msg_info.sender.as_str())?)
    {
        return Err(TreasuryError::SenderNotWhitelisted {});
    }

    Ok(Response::new().add_messages(msgs))
}

/// Update the stored vault asset information
pub fn update_assets(
    deps: DepsMut,
    msg_info: MessageInfo,
    to_add: Vec<VaultAsset>,
    to_remove: Vec<AssetInfo>,
) -> TreasuryResult {
    // Only Admin can call this method
    ADMIN.assert_admin(deps.as_ref(), &msg_info.sender)?;

    // Check the vault size to be within the size limit to prevent running out of gas when doing lookups
    let current_vault_size = VAULT_ASSETS
        .keys(deps.storage, None, None, Ascending)
        .count();
    let delta: i128 = to_add.len() as i128 - to_remove.len() as i128;
    if current_vault_size as i128 + delta > LIST_SIZE_LIMIT as i128 {
        return Err(TreasuryError::AssetsLimitReached {});
    }

    for new_asset in to_add.into_iter() {
        let id = get_identifier(&new_asset.asset.info).as_str();
        // update function for new or existing keys
        let insert =
            |_vault_asset: Option<VaultAsset>| -> StdResult<VaultAsset> { Ok(new_asset.clone()) };
        VAULT_ASSETS.update(deps.storage, id, insert)?;
    }

    for asset_id in to_remove {
        VAULT_ASSETS.remove(deps.storage, get_identifier(&asset_id).as_str());
    }

    Ok(Response::new().add_attribute("action", "update_cw20_token_list"))
}

/// Add a contract to the whitelist
pub fn add_dapp(deps: DepsMut, msg_info: MessageInfo, dapp: String) -> TreasuryResult {
    ADMIN.assert_admin(deps.as_ref(), &msg_info.sender)?;

    let mut state = STATE.load(deps.storage)?;
    if state.dapps.contains(&deps.api.addr_validate(&dapp)?) {
        return Err(TreasuryError::AlreadyInList {});
    }

    // This is a limit to prevent potentially running out of gas when doing lookups on the dapps list
    if state.dapps.len() >= LIST_SIZE_LIMIT {
        return Err(TreasuryError::DAppsLimitReached {});
    }

    // Add contract to whitelist.
    state.dapps.push(deps.api.addr_validate(&dapp)?);
    STATE.save(deps.storage, &state)?;

    // Respond and note the change
    Ok(Response::new().add_attribute("Added contract to whitelist: ", dapp))
}

/// Remove a contract from the whitelist
pub fn remove_dapp(deps: DepsMut, msg_info: MessageInfo, dapp: String) -> TreasuryResult {
    ADMIN.assert_admin(deps.as_ref(), &msg_info.sender)?;

    let mut state = STATE.load(deps.storage)?;
    if !state.dapps.contains(&deps.api.addr_validate(&dapp)?) {
        return Err(TreasuryError::NotInList {});
    }

    // Remove contract from whitelist.
    let dapp_address = deps.api.addr_validate(&dapp)?;
    state.dapps.retain(|addr| *addr != dapp_address);
    STATE.save(deps.storage, &state)?;

    // Respond and note the change
    Ok(Response::new().add_attribute("Removed contract from whitelist: ", dapp))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Config {} => to_binary(&query_config(deps)?),
        QueryMsg::TotalValue {} => to_binary(&TotalValueResponse {
            value: compute_total_value(deps, env)?,
        }),
        QueryMsg::HoldingAmount { identifier } => {
            let vault_asset: VaultAsset = VAULT_ASSETS.load(deps.storage, identifier.as_str())?;
            to_binary(&query_asset_balance(
                deps,
                &vault_asset.asset.info,
                env.contract.address,
            )?)
        }
        QueryMsg::HoldingValue { identifier } => to_binary(&HoldingValueResponse {
            value: compute_holding_value(deps, &env, identifier)?,
        }),
        QueryMsg::VaultAssetConfig { identifier } => {
            to_binary(&VAULT_ASSETS.load(deps.storage, identifier.as_str())?)
        }
    }
}

/// Returns the whitelisted dapps
pub fn query_config(deps: Deps) -> StdResult<ConfigResponse> {
    let state = STATE.load(deps.storage)?;
    let dapps: Vec<Addr> = state.dapps;
    let resp = ConfigResponse {
        dapps: dapps
            .iter()
            .map(|dapp| -> String { dapp.to_string() })
            .collect(),
    };
    Ok(resp)
}

/// Returns the value of a specified asset.
pub fn compute_holding_value(deps: Deps, env: &Env, holding: String) -> StdResult<Uint128> {
    let mut vault_asset: VaultAsset = VAULT_ASSETS.load(deps.storage, holding.as_str())?;
    let value = vault_asset.value(deps, env, None)?;
    Ok(value)
}

/// Computes the total value locked in this contract
pub fn compute_total_value(deps: Deps, env: Env) -> StdResult<Uint128> {
    // Get all assets from storage
    let mut all_assets = VAULT_ASSETS
        .range(deps.storage, None, None, Order::Ascending)
        .collect::<StdResult<Vec<(Vec<u8>, VaultAsset)>>>()?;

    let mut total_value = Uint128::zero();
    // Calculate their value iteratively
    for vault_asset_entry in all_assets.iter_mut() {
        total_value += vault_asset_entry.1.value(deps, &env, None)?;
    }

    Ok(total_value)
}
