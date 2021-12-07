use cosmwasm_std::{
    entry_point, to_binary, Addr, Binary, CosmosMsg, Decimal, Deps, DepsMut, Env, Fraction,
    MessageInfo, Response, StdResult, Uint128, WasmMsg,
};

use cw20::{Cw20ExecuteMsg, Cw20ReceiveMsg};
// use terraswap::asset::Asset;
// use terraswap::pair::{Cw20HookMsg, PoolResponse};

use white_whale::astroport_helper::*;
use crate::error::DAppError;
use crate::msg::{ExecuteMsg, InstantiateMsg, QueryMsg, StateResponse};
use crate::state::{get_asset_info, load_contract_addr, State, ADDRESS_BOOK, ADMIN, STATE};
use crate::astroport_msg::*;
use white_whale::query::astroport::{query_asset_balance, query_pool};
use white_whale::treasury::msg::send_to_treasury;

type DAppResult = Result<Response, DAppError>;
const PAIR_POSTFIX: &str = "_pair";

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(deps: DepsMut, _env: Env, info: MessageInfo, msg: InstantiateMsg) -> DAppResult {
    let state = State {
        treasury_address: deps.api.addr_canonicalize(&msg.treasury_address)?,
        trader: deps.api.addr_canonicalize(&msg.trader)?,
    };

    // Store the initial config
    STATE.save(deps.storage, &state)?;

    // Setup the admin as the creator of the contract
    ADMIN.set(deps, Some(info.sender))?;

    Ok(Response::default())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(deps: DepsMut, env: Env, info: MessageInfo, msg: ExecuteMsg) -> DAppResult {
    match msg {
        ExecuteMsg::ProvideLiquidity {
            pool_id,
            main_asset_id,
            amount,
        } => provide_liquidity(deps.as_ref(), info, main_asset_id, pool_id, amount),
        ExecuteMsg::WithdrawLiquidity {
            lp_token_id,
            amount,
        } => withdraw_liquidity(deps.as_ref(), info, lp_token_id, amount),
        ExecuteMsg::SwapAsset {
            offer_id,
            pool_id,
            amount,
            max_spread,
            belief_price,
        } => astroport_swap(
            deps.as_ref(),
            env,
            info,
            offer_id,
            pool_id,
            amount,
            max_spread,
            belief_price,
        ),
        ExecuteMsg::UpdateConfig {
            treasury_address,
            trader,
        } => update_config(deps, info, treasury_address, trader),
        ExecuteMsg::SetAdmin { admin } => {
            ADMIN.assert_admin(deps.as_ref(), &info.sender)?;

            let admin_addr = deps.api.addr_validate(&admin)?;
            let previous_admin = ADMIN.get(deps.as_ref())?.unwrap();
            ADMIN.execute_update_admin(deps, info, Some(admin_addr))?;
            Ok(Response::default()
                .add_attribute("previous admin", previous_admin)
                .add_attribute("admin", admin))
        }
        ExecuteMsg::UpdateAddressBook { to_add, to_remove } => {
            update_address_book(deps, info, to_add, to_remove)
        }
    }
}

//----------------------------------------------------------------------------------------
//  EXECUTE FUNCTION HANDLERS
//----------------------------------------------------------------------------------------

/// Constructs and forwards the terraswap provide_liquidity message
pub fn provide_liquidity(
    deps: Deps,
    msg_info: MessageInfo,
    main_asset_id: String,
    pool_id: String,
    amount: Uint128,
) -> DAppResult {
    let state = STATE.load(deps.storage)?;
    // Check if caller is trader.
    if msg_info.sender != deps.api.addr_humanize(&state.trader)? {
        return Err(DAppError::Unauthorized {});
    }

    let treasury_address = deps.api.addr_humanize(&state.treasury_address)?;

    // Get lp token address
    let pair_address = load_contract_addr(deps, &pool_id)?;

    // Get pool info
    let pool_info: PoolResponse = query_pool(deps, &pair_address)?;
    let asset_1 = &pool_info.assets[0];
    let asset_2 = &pool_info.assets[1];

    let ratio = Decimal::from_ratio(asset_1.amount, asset_2.amount);

    let main_asset_info = get_asset_info(deps, &main_asset_id)?;
    let main_asset = Asset {
        info: main_asset_info,
        amount,
    };
    let mut first_asset: Asset;
    let mut second_asset: Asset;

    // Determine second asset and required amount to do a 50/50 LP
    if asset_2.info.equal(&main_asset.info) {
        first_asset = asset_1.clone();
        first_asset.amount = ratio * amount;
        second_asset = main_asset;
    } else {
        second_asset = asset_2.clone();
        second_asset.amount = ratio.inv().unwrap_or_default() * amount;
        first_asset = main_asset;
    }

    // Does the treasury have enough of these assets?
    let first_asset_balance =
        query_asset_balance(deps, &first_asset.info, treasury_address.clone())?;
    let second_asset_balance =
        query_asset_balance(deps, &second_asset.info, treasury_address.clone())?;
    if second_asset_balance < second_asset.amount || first_asset_balance < first_asset.amount {
        return Err(DAppError::Broke {});
    }

    let msgs: Vec<CosmosMsg> = deposit_lp_msg(deps, [second_asset, first_asset], pair_address)?;

    // Deposit lp msg either returns a bank send msg or a
    // increase allowance msg for each asset.
    Ok(Response::new().add_message(send_to_treasury(msgs, &treasury_address)?))
}

/// Constructs withdraw liquidity msg and forwards it to treasury
pub fn withdraw_liquidity(
    deps: Deps,
    msg_info: MessageInfo,
    lp_token_id: String,
    amount: Uint128,
) -> DAppResult {
    let state = STATE.load(deps.storage)?;
    if msg_info.sender != deps.api.addr_humanize(&state.trader)? {
        return Err(DAppError::Unauthorized {});
    }
    let treasury_address = deps.api.addr_humanize(&state.treasury_address)?;

    // get lp token address
    let lp_token_address = load_contract_addr(deps, &lp_token_id)?;
    let pair_address = load_contract_addr(deps, &(lp_token_id.clone() + PAIR_POSTFIX))?;

    // Check if the treasury has enough lp tokens
    has_sufficient(deps, &lp_token_id, &treasury_address, amount)?;


    
    let cw20_msg = Cw20ReceiveMsg {
        sender: String::from(&treasury_address),
        amount: amount,
        msg: to_binary(&Cw20HookMsg::WithdrawLiquidity {})?
    };


    // Call on LP token.
    let lp_call = CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: String::from(lp_token_address),
        msg: to_binary(&cw20_msg)?,
        funds: vec![],
    });

    Ok(Response::new().add_message(send_to_treasury(vec![lp_call], &treasury_address)?))
}

/// Function constructs astroport swap messages and forwards them to the treasury
#[allow(clippy::too_many_arguments)]
pub fn astroport_swap(
    deps: Deps,
    _env: Env,
    msg_info: MessageInfo,
    offer_id: String,
    pool_id: String,
    amount: Uint128,
    max_spread: Option<Decimal>,
    belief_price: Option<Decimal>,
) -> DAppResult {
    let state = STATE.load(deps.storage)?;
    let treasury_address = deps.api.addr_humanize(&state.treasury_address)?;

    // Check if caller is trader
    if msg_info.sender != deps.api.addr_humanize(&state.trader)? {
        return Err(DAppError::Unauthorized {});
    }

    // Check if treasury has enough to swap
    has_sufficient(deps, &offer_id, &treasury_address, amount)?;

    let pair_address = load_contract_addr(deps, &pool_id)?;

    let offer_asset_info = get_asset_info(deps, &offer_id)?;

    let swap_msg = vec![asset_into_swap_msg(
        deps,
        pair_address,
        Asset {
            info: offer_asset_info,
            amount,
        },
        max_spread,
        belief_price,
        None,
    )?];

    Ok(Response::new().add_message(send_to_treasury(swap_msg, &treasury_address)?))
}

//----------------------------------------------------------------------------------------
//  GOVERNANCE CONTROLLED SETTERS
//----------------------------------------------------------------------------------------

pub fn update_address_book(
    deps: DepsMut,
    msg_info: MessageInfo,
    to_add: Vec<(String, String)>,
    to_remove: Vec<String>,
) -> DAppResult {
    // Only Admin can call this method
    ADMIN.assert_admin(deps.as_ref(), &msg_info.sender)?;

    for (name, new_address) in to_add.into_iter() {
        // update function for new or existing keys
        let insert = |vault_asset: Option<String>| -> StdResult<String> {
            match vault_asset {
                // Todo: is there a better way to just leave the data untouched?
                Some(present) => Ok(present),
                None => Ok(new_address),
            }
        };
        ADDRESS_BOOK.update(deps.storage, name.as_str(), insert)?;
    }

    for name in to_remove {
        ADDRESS_BOOK.remove(deps.storage, name.as_str());
    }
    Ok(Response::new().add_attribute("action", "updated address book"))
}

/// Updates trader or treasury address
pub fn update_config(
    deps: DepsMut,
    info: MessageInfo,
    treasury_address: Option<String>,
    trader: Option<String>,
) -> DAppResult {
    // Only the admin should be able to call this
    ADMIN.assert_admin(deps.as_ref(), &info.sender)?;

    let mut state = STATE.load(deps.storage)?;

    if let Some(treasury_address) = treasury_address {
        state.treasury_address = deps.api.addr_canonicalize(&treasury_address)?;
    }

    if let Some(trader) = trader {
        state.trader = deps.api.addr_canonicalize(&trader)?;
    }

    STATE.save(deps.storage, &state)?;
    Ok(Response::new().add_attribute("Update:", "Successfull"))
}

//----------------------------------------------------------------------------------------
//  QUERY HANDLERS
//----------------------------------------------------------------------------------------

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Config {} => to_binary(&try_query_config(deps)?),
        QueryMsg::AddressBook { id } => to_binary(&try_query_addressbook(deps, id)?),
    }
}

pub fn try_query_config(deps: Deps) -> StdResult<StateResponse> {
    let state = STATE.load(deps.storage)?;

    Ok(StateResponse {
        treasury_address: deps
            .api
            .addr_humanize(&state.treasury_address)?
            .into_string(),
        trader: deps.api.addr_humanize(&state.trader)?.into_string(),
    })
}

pub fn try_query_addressbook(deps: Deps, id: String) -> StdResult<String> {
    ADDRESS_BOOK.load(deps.storage, id.as_str())
}

//----------------------------------------------------------------------------------------
//  UTIL FUNCTIONS
//----------------------------------------------------------------------------------------

pub fn has_sufficient(
    deps: Deps,
    id: &str,
    address: &Addr,
    required: Uint128,
) -> Result<(), DAppError> {
    // Load asset
    let info = get_asset_info(deps, id)?;
    // Get balance and check
    if query_asset_balance(deps, &info, address.clone())? < required {
        return Err(DAppError::Broke {});
    }
    Ok(())
}