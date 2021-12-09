#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_binary, Binary, CanonicalAddr, CosmosMsg, Deps, DepsMut, Empty, Env, MessageInfo, Order,
    Response, StdResult, Uint128,
};

use crate::error::TreasuryError;
use terraswap::asset::AssetInfo;
use white_whale::query::terraswap::query_asset_balance;
use white_whale::treasury::msg::{ConfigResponse, ExecuteMsg, InstantiateMsg, QueryMsg, MigrateMsg};
use white_whale::treasury::state::{State, ADMIN, STATE, VAULT_ASSETS};
use white_whale::treasury::vault_assets::{get_identifier, VaultAsset};
use cw2::{set_contract_version, get_contract_version};
use semver::Version;
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
pub fn execute(deps: DepsMut, env: Env, info: MessageInfo, msg: ExecuteMsg) -> TreasuryResult {
    match msg {
        ExecuteMsg::DAppAction { msgs } => execute_action(deps, info, msgs),
        ExecuteMsg::SendAsset {
            id,
            amount,
            recipient,
        } => send_asset(deps.as_ref(), env, info, id, amount, recipient),
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
    // let data = deps
    //     .storage
    //     .get(CONFIG_KEY)
    //     .ok_or_else(|| StdError::not_found("State"))?;
    // // We can start a new State object from the old one
    // let mut config: State = from_slice(&data)?;
    // // And use something provided in MigrateMsg to update the state of the migrated contract
    // config.verifier = deps.api.addr_validate(&msg.verifier)?;
    // // Then store our modified State 
    // deps.storage.set(CONFIG_KEY, &to_vec(&config)?);
    // If we have no need to update the State of the contract then just Response::default() should suffice
    // in this case, the code is still updated, the migration does not change the contract addr or funds 
    // if this is the case you desire, consider making the new Addr part of the MigrateMsg and then doing
    // a payout

    let version: Version = CONTRACT_VERSION.parse()?;
    let storage_version: Version = get_contract_version(deps.storage)?.version.parse()?;

    if storage_version < version {
        set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

        // If state structure changed in any contract version in the way migration is needed, it
        // should occur here
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
        .contains(&deps.api.addr_canonicalize(msg_info.sender.as_str())?)
    {
        return Err(TreasuryError::SenderNotWhitelisted {});
    }

    Ok(Response::new().add_messages(msgs))
}

pub fn send_asset(deps: Deps,
    env: Env,
    msg_info: MessageInfo,
    id: String,
    amount: Uint128,
    recipient: String,
) -> TreasuryResult {
    // Only admin can send funds
    ADMIN.assert_admin(deps, &msg_info.sender)?;
    let mut vault_asset = VAULT_ASSETS.load(deps.storage, &id)?.asset;
    // Get balance and check against requested
    let balance = query_asset_balance(deps, &vault_asset.info, env.contract.address)?;
    if balance < amount {
        return Err(TreasuryError::Broke{ requested: amount, balance})
    }
    vault_asset.amount = amount;
    Ok(Response::new().add_message(vault_asset.into_msg(&deps.querier, deps.api.addr_validate(&recipient)?)?))
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
    if state
        .dapps
        .contains(&deps.api.addr_canonicalize(&dapp)?)
    {
        return Err(TreasuryError::AlreadyInList {});
    }

    // Add contract to whitelist.
    state.dapps.push(deps.api.addr_canonicalize(&dapp)?);
    STATE.save(deps.storage, &state)?;

    // Respond and note the change
    Ok(Response::new().add_attribute("Added contract to whitelist: ", dapp))
}

/// Remove a contract from the whitelist
pub fn remove_dapp(deps: DepsMut, msg_info: MessageInfo, dapp: String) -> TreasuryResult {
    ADMIN.assert_admin(deps.as_ref(), &msg_info.sender)?;

    let mut state = STATE.load(deps.storage)?;
    if !state
        .dapps
        .contains(&deps.api.addr_canonicalize(&dapp)?)
    {
        return Err(TreasuryError::NotInList {});
    }

    // Remove contract from whitelist.
    let canonical_addr = deps.api.addr_canonicalize(&dapp)?;
    state.dapps.retain(|addr| *addr != canonical_addr);
    STATE.save(deps.storage, &state)?;

    // Respond and note the change
    Ok(Response::new().add_attribute("Removed contract from whitelist: ", dapp))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Config {} => to_binary(&query_config(deps)?),
        QueryMsg::TotalValue {} => to_binary(&compute_total_value(deps, env)?),
        QueryMsg::HoldingAmount { identifier } => {
            let vault_asset: VaultAsset = VAULT_ASSETS.load(deps.storage, identifier.as_str())?;
            to_binary(&query_asset_balance(
                deps,
                &vault_asset.asset.info,
                env.contract.address,
            )?)
        }
        QueryMsg::HoldingValue { identifier } => {
            to_binary(&compute_holding_value(deps, &env, identifier)?)
        }
        QueryMsg::VaultAssetConfig { identifier } => {
            to_binary(&VAULT_ASSETS.load(deps.storage, identifier.as_str())?)
        }
    }
}

/// Returns the whitelisted dapps
pub fn query_config(deps: Deps) -> StdResult<ConfigResponse> {
    let state = STATE.load(deps.storage)?;
    let dapps: Vec<CanonicalAddr> = state.dapps;
    let resp = ConfigResponse {
        dapps: dapps
            .iter()
            .map(|dapp| -> String { deps.api.addr_humanize(dapp).unwrap().to_string() })
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
