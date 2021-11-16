#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_binary, Addr, Binary, Coin, CosmosMsg, Deps, DepsMut, Env, MessageInfo, QuerierWrapper,
    QueryRequest, Response, StdError, StdResult, WasmMsg, WasmQuery,
};

use terraswap::asset::AssetInfo;
use whitewhale_liquidation_helpers::astroport_helper::{
    trade_cw20_for_native_on_astroport, trade_native_for_cw20_on_astroport,
    trade_native_for_native_on_astroport,
};
use whitewhale_liquidation_helpers::dex_arb_helper::{
    CallbackMsg, ConfigResponse, DexInfo, ExecuteMsg, InstantiateMsg, PoolInfo, QueryMsg,
    StateResponse, UpdateConfigMsg,
};
use whitewhale_liquidation_helpers::flashloan_helper::build_flash_loan_msg;
use whitewhale_liquidation_helpers::helper::{
    build_send_cw20_token_msg, build_send_native_asset_msg, get_denom_amount_from_coins,
    option_string_to_addr, query_balance, query_token_balance,
};
use whitewhale_liquidation_helpers::tax::{compute_tax, deduct_tax};
use whitewhale_liquidation_helpers::terraswap_helper::{
    trade_cw20_on_terraswap, trade_native_on_terraswap,
};

use crate::state::{Config, State, ASTROPORT, CONFIG, LOOP, STATE, TERRASWAP};
use cosmwasm_bignumber::Uint256;

const UUSD_DENOM: &str = "uusd";

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> StdResult<Response> {
    let config = Config {
        owner: deps.api.addr_validate(&msg.owner)?,
        ust_vault_address: deps.api.addr_validate(&msg.ust_vault_address)?,
    };

    CONFIG.save(deps.storage, &config)?;
    STATE.save(
        deps.storage,
        &State {
            total_arbs: 0u64,
            total_ust_profit: Uint256::zero(),
        },
    )?;
    LOOP.save(deps.storage, &Loop { loop_pools: vec![] })?;
    TERRASWAP.save(
        deps.storage,
        &Loop {
            terraswap_pools: vec![],
        },
    )?;
    ASTROPORT.save(
        deps.storage,
        &Loop {
            astroport_pools: vec![],
        },
    )?;

    Ok(Response::default())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> StdResult<Response> {
    match msg {
        ExecuteMsg::UpdateConfig { new_config } => handle_update_config(deps, info, new_config),
        ExecuteMsg::AddPool { dex, new_asset } => handle_add_pool(deps, info, dex, new_asset),
        ExecuteMsg::InitiateUstArbitrage {
            buy_side,
            sell_side,
            ust_to_borrow,
            asset,
        } => handle_ust_arbitrage(deps, _env, info, buy_side, sell_side, ust_to_borrow, asset),
        ExecuteMsg::Callback(msg) => _handle_callback(deps, _env, info, msg),
    }
}

fn _handle_callback(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: CallbackMsg,
) -> StdResult<Response> {
    // Callback functions can only be called this contract itself
    if info.sender != env.contract.address {
        return Err(StdError::generic_err(
            "callbacks cannot be invoked externally",
        ));
    }
    match msg {
        CallbackMsg::InitiateArbCallback {
            buy_side,
            sell_side,
            asset,
        } => initiate_arb_callback(deps, env, info, buy_side, sell_side, asset),
        CallbackMsg::AfterBuyCallback {
            sell_side,
            asset,
            amount,
        } => after_buy_callback(deps, env, sell_side, asset, amount),
        CallbackMsg::AfterSellCallback { arb_amount } => after_sell_callback(deps, env, arb_amount),
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Config {} => to_binary(&query_config(deps)?),
        QueryMsg::State {} => to_binary(&query_state(deps)?),
    }
}

//----------------------------------------------------------------------------------------
// EXECUTE FUNCTION HANDLERS
//----------------------------------------------------------------------------------------

/// @dev Admin function to update Configuration parameters
/// @param new_config : Same as UpdateConfigMsg struct
pub fn handle_update_config(
    deps: DepsMut,
    info: MessageInfo,
    new_config: UpdateConfigMsg,
) -> StdResult<Response> {
    let mut config = CONFIG.load(deps.storage)?;

    // CHECK :: Only owner can call this function
    if info.sender != config.owner {
        return Err(StdError::generic_err("Unauthorized"));
    }

    // UPDATE :: ADDRESSES IF PROVIDED
    config.owner = option_string_to_addr(deps.api, new_config.owner, config.owner)?;
    config.ust_vault_address = option_string_to_addr(
        deps.api,
        new_config.ust_vault_address,
        config.ust_vault_address,
    )?;

    CONFIG.save(deps.storage, &config)?;
    Ok(Response::new().add_attribute("action", "dex_arb_helper::ExecuteMsg::UpdateConfig"))
}

/// @dev Admin function to add a new pool supported by Astroport / Terraswap / Loop DEX
/// @param new_asset :
pub fn handle_add_pool(
    deps: DepsMut,
    info: MessageInfo,
    dex: DexInfo,
    new_pool: PoolInfo,
) -> StdResult<Response> {
    let config = CONFIG.load(deps.storage)?;

    // CHECK :: Only owner can call this function
    if info.sender != config.owner {
        return Err(StdError::generic_err("Unauthorized"));
    }

    // Init response
    let mut response = Response::new();

    match dex {
        DexInfo::Terraswap {} => {
            let mut terraswap_pools_vec = TERRASWAP.load(deps.storage)?;
            for pool_ in terraswap_pools_vec.terraswap_pools.iter() {
                if pool_.asset_token == new_pool.asset_token.clone() {
                    return Err(StdError::generic_err("Already Supported"));
                }
            }
            terraswap_pools_vec.terraswap_pools.push(new_pool.clone());
            response = response
                .add_attribute("action", "Add_Terraswap_Pool")
                .add_attribute("pool_address", new_pool.pair_address.clone());
            TERRASWAP.save(deps.storage, &terraswap_pools_vec)?;
        }
        DexInfo::Loop {} => {
            let mut loop_pools_vec = LOOP.load(deps.storage)?;
            for pool_ in loop_pools_vec.loop_pools.iter() {
                if pool_.asset_token == new_pool.asset_token.clone() {
                    return Err(StdError::generic_err("Already Supported"));
                }
            }
            loop_pools_vec.loop_pools.push(new_pool.clone());
            response = response
                .add_attribute("action", "Add_Loop_Pool")
                .add_attribute("pool_address", new_pool.pair_address);
            LOOP.save(deps.storage, &loop_pools_vec)?;
        }
        DexInfo::Astroport {} => {
            let mut astroport_pools_vec = ASTROPORT.load(deps.storage)?;
            for pool_ in astroport_pools_vec.loop_pools.iter() {
                if pool_.asset_token == new_pool.asset_token.clone() {
                    return Err(StdError::generic_err("Already Supported"));
                }
            }
            astroport_pools_vec.loop_pools.push(new_pool.clone());
            response = response
                .add_attribute("action", "Add_Loop_Pool")
                .add_attribute("pool_address", new_pool.pair_address);
            ASTROPORT.save(deps.storage, &astroport_pools_vec)?;
        }
    }

    Ok(response)
}

/// @dev
/// @param  :
pub fn handle_ust_arbitrage(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    buy_side: DexInfo,
    sell_side: DexInfo,
    ust_to_borrow: Uint256,
    asset: AssetInfo,
) -> StdResult<Response> {
    let config = CONFIG.load(deps.storage)?;

    let callback_binary = to_binary(
        &CallbackMsg::InitiateArbCallback {
            buy_side: buy_side,
            sell_side: sell_side,
            asset: asset,
        }
        .to_cosmos_msg(&env.contract.address)?,
    )?;

    let flash_loan_msg = build_flash_loan_msg(
        config.ust_vault_address.to_string(),
        UUSD_DENOM,
        ust_to_borrow,
        callback_binary,
    )?;

    Ok(Response::new()
        .add_message(flash_loan_msg)
        .add_attribute("action", "dex_arb_helper::ExecuteMsg::Arbitrage")
        .add_attribute("loan_asked", ust_to_borrow.to_string()))
}

//----------------------------------------------------------------------------------------
//  CALLBACK FUNCTION HANDLERS
//----------------------------------------------------------------------------------------

pub fn initiate_arb_callback(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    buy_side: DexInfo,
    sell_side: DexInfo,
    asset: AssetInfo,
) -> StdResult<Response> {
    let config = CONFIG.load(deps.storage)?;

    // get UST sent for the liquidation
    let ust_amount = get_denom_amount_from_coins(&info.funds, &config.stable_denom);

    // COSMOS MSGS ::
    // 1. Add Buy debt asset from buy_side DEX Msg
    // 2. Add 'AfterBuyCallback' Callback Msg

    // Init response
    let mut response = Response::new();

    let asset_type: String;
    let asset_identifer: String;
    match asset.clone() {
        AssetInfo::NativeToken { denom } => {
            asset_type = "native".to_string();
            asset_identifer = denom;
        }
        AssetInfo::Token { contract_addr } => {
            asset_type = "cw20".to_string();
            asset_identifer = contract_addr;
        }
    }

    // Add the specific Cosmos Msg
    match buy_side.clone() {
        DexInfo::Astroport {} => {
            let astroport_pools_vec = ASTROPORT.load(deps.storage)?;
            let mut pair_address = "".to_string();
            for pool_ in astroport_pools_vec.astroport_pools.iter() {
                if pool_.asset_token == asset.clone() {
                    pair_address = pool_.pair_address.to_string();
                }
            }
            if pair_address.clone() == "".to_string() {
                return Err(StdError::generic_err(
                    "Astroport pair info not found for the asset to be arbitraged",
                ));
            }
            let trade_msg = trade_native_on_astroport(
                deps.as_ref(),
                pair_address,
                UUSD_DENOM,
                ust_amount.into(),
            )?;
            response = response
                .add_message(trade_msg)
                .add_attribute("astroport_buy", asset_identifer.to_string());
        }
        DexInfo::Terraswap {} => {
            let terraswap_pools_vec = TERRASWAP.load(deps.storage)?;
            let mut pair_address = "".to_string();
            for pool_ in terraswap_pools_vec.terraswap_pools.iter() {
                if pool_.asset_token == asset.clone() {
                    pair_address = pool_.pair_address.to_string();
                }
            }
            if pair_address.clone() == "".to_string() {
                return Err(StdError::generic_err(
                    "Terraswap pair info not found for the asset to be arbitraged",
                ));
            }
            let trade_msg = trade_native_on_terraswap(
                deps.as_ref(),
                pair_address,
                UUSD_DENOM,
                ust_amount.into(),
            )?;
            response = response
                .add_message(trade_msg)
                .add_attribute("terraswap_buy", asset_identifer.to_string());
        }
        DexInfo::Loop {} => {
            let loop_pools_vec = LOOP.load(deps.storage)?;
            let mut pair_address_ = "".to_string();
            for pool_ in loop_pools_vec.loop_pools.iter() {
                if pool_.asset_token == asset.clone() {
                    pair_address_ = pool_.pair_address.to_string();
                }
            }
            if pair_address_.clone() == "".to_string() {
                return Err(StdError::generic_err(
                    "Loop pair info not found for the asset to be arbitraged",
                ));
            }
            let trade_msg = trade_native_on_terraswap(
                deps.as_ref(),
                pair_address_,
                UUSD_DENOM,
                ust_amount.into(),
            )?;
            response = response
                .add_message(trade_msg)
                .add_attribute("loop_buy", asset_identifer.to_string());
        }
    }

    // Callback Cosmos Msg
    let after_buy_callback_msg = CallbackMsg::AfterBuyCallback {
        sell_side: sell_side,
        asset: asset,
        amount: ust_amount,
    }
    .to_cosmos_msg(&env.contract.address)?;
    response = response.add_message(after_buy_callback_msg);

    Ok(response)
}

pub fn after_buy_callback(
    deps: DepsMut,
    env: Env,
    sell_side: DexInfo,
    asset: AssetInfo,
    arb_amount: Uint256,
) -> StdResult<Response> {
    let config = CONFIG.load(deps.storage)?;

    // Init response
    let mut response = Response::new();

    // COSMOS MSGS ::
    // 1. Add Sell_Asset Msg
    // 2. Add 'AfterSellCallback' Callback Msg

    let asset_type: String;
    let asset_identifer: String;
    let asset_balance: Uint256;
    match asset.clone() {
        AssetInfo::NativeToken { denom } => {
            asset_type = "native".to_string();
            asset_identifer = denom.clone();
            asset_balance =
                query_balance(&deps.querier, env.contract.address.clone(), denom.clone())?.into();
        }
        AssetInfo::Token { contract_addr } => {
            asset_type = "cw20".to_string();
            asset_identifer = contract_addr.clone();
            asset_balance = query_token_balance(
                &deps.querier,
                deps.api.addr_validate(&contract_addr.clone())?,
                env.contract.address.clone(),
            )?
            .into();
        }
    }

    match sell_side.clone() {
        DexInfo::Astroport {} => {
            let astroport_pools_vec = ASTROPORT.load(deps.storage)?;
            let mut pair_address = "".to_string();
            for pool_ in astroport_pools_vec.terraswap_pools.iter() {
                if pool_.asset_token == asset.clone() {
                    pair_address = pool_.pair_address.to_string();
                }
            }
            if asset_type == "native".to_string() {
                let trade_msg = trade_native_on_astroport(
                    deps.as_ref(),
                    pair_address,
                    asset_identifer.clone(),
                    asset_balance.into(),
                )?;
                response = response
                    .add_message(trade_msg)
                    .add_attribute("astroport_sell_native", asset_identifer.to_string());
            } else {
                let trade_msg = trade_cw20_on_astroport(
                    pair_address,
                    deps.api.addr_validate(&asset_identifer)?,
                    asset_balance.into(),
                )?;
                response = response
                    .add_message(trade_msg)
                    .add_attribute("astroport_sell_cw20", asset_identifer.to_string());
            }
        }
        DexInfo::Terraswap {} => {
            let terraswap_pools_vec = TERRASWAP.load(deps.storage)?;
            let mut pair_address = "".to_string();
            for pool_ in terraswap_pools_vec.terraswap_pools.iter() {
                if pool_.asset_token == asset.clone() {
                    pair_address = pool_.pair_address.to_string();
                }
            }
            if pair_address.clone() == "".to_string() {
                return Err(StdError::generic_err(
                    "Loop pair info not found for the asset to be arbitraged",
                ));
            }
            let trade_msg: CosmosMsg;
            if asset_type == "native".to_string() {
                trade_msg = trade_native_on_terraswap(
                    deps.as_ref(),
                    pair_address,
                    asset_identifer.clone(),
                    asset_balance.into(),
                )?;
            } else {
                trade_msg = trade_cw20_on_terraswap(
                    pair_address,
                    asset_identifer.clone(),
                    asset_balance.into(),
                )?;
            }
            response = response
                .add_message(trade_msg)
                .add_attribute("terraswap_sell", asset_identifer.clone().to_string());
        }
        DexInfo::Loop {} => {
            let loop_pools_vec = LOOP.load(deps.storage)?;
            let mut pair_address = "".to_string();
            for pool_ in loop_pools_vec.loop_pools.iter() {
                if pool_.asset_token == asset.clone() {
                    pair_address = pool_.pair_address.to_string();
                }
            }
            if pair_address.clone() == "".to_string() {
                return Err(StdError::generic_err(
                    "Loop pair info not found for the asset to be arbitraged",
                ));
            }
            let trade_msg: CosmosMsg;
            if asset_type == "native".to_string() {
                trade_msg = trade_native_on_terraswap(
                    deps.as_ref(),
                    pair_address,
                    asset_identifer.clone(),
                    asset_balance.into(),
                )?;
            } else {
                trade_msg = trade_cw20_on_terraswap(
                    pair_address,
                    asset_identifer.clone(),
                    asset_balance.into(),
                )?;
            }
            response = response
                .add_message(trade_msg)
                .add_attribute("loop_sell", asset_identifer.to_string());
        }
    }

    response = response.add_attribute("sell_amount", asset_balance.to_string());

    let after_sell_callback_msg = CallbackMsg::AfterSellCallback {
        arb_amount: arb_amount,
    }
    .to_cosmos_msg(&env.contract.address)?;
    response = response.add_message(after_sell_callback_msg);

    Ok(response)
}

pub fn after_sell_callback(deps: DepsMut, env: Env, arb_amount: Uint256) -> StdResult<Response> {
    let config = CONFIG.load(deps.storage)?;
    let mut state = STATE.load(deps.storage)?;

    // Query UST Balance
    let cur_ust_balance = query_balance(&deps.querier, env.contract.address, UUSD_DENOM.clone())?;
    let tax = compute_tax(
        deps.as_ref(),
        &Coin {
            denom: UUSD_DENOM.clone(),
            amount: cur_ust_balance,
        },
    )?;
    let computed_ust_balance = Uint256::from(cur_ust_balance - tax);

    if arb_amount >= computed_ust_balance {
        return Err(StdError::generic_err(format!(
            "UST received post arbitrage = {} is less than minimum needed UST balance = {}",
            computed_ust_balance, arb_amount
        )));
    }

    let profit = computed_ust_balance - arb_amount;
    state.total_arbs += 1u64;
    state.total_ust_profit += profit;

    // COSMOS MSGS ::
    // 1. Send UST Back to the UST arb strategy
    let send_native_asset_msg = build_send_native_asset_msg(
        deps.as_ref(),
        config.ust_vault_address.clone(),
        &UUSD_DENOM,
        cur_ust_balance.into(),
    )?;
    STATE.save(deps.storage, &state)?;

    Ok(Response::new()
        .add_message(send_native_asset_msg)
        .add_attribute("return_ust_to_vault", cur_ust_balance.to_string())
        .add_attribute("profit", profit.to_string()))
}

//-----------------------------------------------------------
// QUERY HANDLERS
//-----------------------------------------------------------

/// @dev Returns the contract's configuration
pub fn query_config(deps: Deps) -> StdResult<ConfigResponse> {
    let config = CONFIG.load(deps.storage)?;
    let loop_pools_vec = LOOP.load(deps.storage)?;
    let terraswap_pools_vec = TERRASWAP.load(deps.storage)?;
    let astroport_pools_vec = ASTROPORT.load(deps.storage)?;

    Ok(ConfigResponse {
        owner: config.owner.to_string(),
        ust_vault_address: config.ust_vault_address.to_string(),
        loop_pools: loop_pools_vec,
        terraswap_pools: terraswap_pools_vec,
        astroport_pools: astroport_pools_vec,
    })
}

/// @dev Returns the contract's state
pub fn query_state(deps: Deps) -> StdResult<StateResponse> {
    let state = STATE.load(deps.storage)?;

    Ok(StateResponse {
        total_arbs: state.total_arbs,
        total_ust_profit: state.total_ust_profit,
    })
}
