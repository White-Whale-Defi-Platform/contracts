use cosmwasm_std::{to_binary, Addr, Coin, Decimal, Uint128};
use cw20::{BalanceResponse, Cw20QueryMsg};

use terra_multi_test::{App, ContractWrapper};

use crate::msg::{DepositHookMsg, ExecuteMsg, InstantiateMsg, QueryMsg, StateResponse};
use crate::tests::integration_tests::common_integration::{
    init_contracts, mint_some_whale, mock_app, store_token_code,
};
use terra_multi_test::Executor;
use terraswap::asset::Asset;

use white_whale::memory::msg as MemoryMsg;
use white_whale::treasury::msg as TreasuryMsg;
use white_whale::treasury::vault_assets::{ValueRef, VaultAsset};
use white_whale_testing::dapp_base::common::TEST_CREATOR;

use white_whale::treasury::dapp_base::msg::BaseInstantiateMsg;

use super::common_integration::{whitelist_dapp, BaseContracts};
const MILLION: u64 = 1_000_000u64;

fn init_vault_dapp(
    app: &mut App,
    owner: Addr,
    base_contracts: &BaseContracts,
    _token_code_id: u64,
) -> (Addr, Addr) {
    // Upload Vault DApp Contract
    let vault_dapp_contract = Box::new(
        ContractWrapper::new_with_empty(
            crate::contract::execute,
            crate::contract::instantiate,
            crate::contract::query,
        )
        .with_reply(crate::contract::reply),
    );

    let vault_dapp_code_id = app.store_code(vault_dapp_contract);
    let lp_contract_code_id = store_token_code(app);

    let vault_dapp_instantiate_msg = InstantiateMsg {
        base: BaseInstantiateMsg {
            trader: owner.to_string(),
            treasury_address: base_contracts.treasury.to_string(),
            memory_addr: base_contracts.memory.to_string(),
        },
        token_code_id: lp_contract_code_id,
        fee: Decimal::percent(10u64),
        deposit_asset: "ust".to_string(),
        vault_lp_token_name: None,
        vault_lp_token_symbol: None,
    };

    // Init contract
    let vault_dapp_instance = app
        .instantiate_contract(
            vault_dapp_code_id,
            owner.clone(),
            &vault_dapp_instantiate_msg,
            &[],
            "vault_dapp",
            None,
        )
        .unwrap();

    // Get liquidity token addr
    let res: StateResponse = app
        .wrap()
        .query_wasm_smart(vault_dapp_instance.clone(), &QueryMsg::State {})
        .unwrap();
    assert_eq!("Contract #6", res.liquidity_token);
    let liquidity_token = res.liquidity_token;

    // Whitelist vault dapp on treasury
    whitelist_dapp(app, &owner, &base_contracts.treasury, &vault_dapp_instance);

    // Add whale with valueref to whale/ust pool
    // Add whale to vault claimable assets.
    app.execute_contract(
        owner.clone(),
        base_contracts.treasury.clone(),
        &TreasuryMsg::ExecuteMsg::UpdateAssets {
            to_add: vec![
                // uusd is base asset of this vault, so no value_ref
                VaultAsset {
                    asset: Asset {
                        info: terraswap::asset::AssetInfo::NativeToken {
                            denom: "uusd".to_string(),
                        },
                        amount: Uint128::zero(),
                    },
                    value_reference: None,
                },
                // Other asset is WHALE. It's value in uusd is calculated with the provided pool valueref
                VaultAsset {
                    asset: Asset {
                        info: terraswap::asset::AssetInfo::Token {
                            contract_addr: base_contracts.whale.to_string(),
                        },
                        amount: Uint128::zero(),
                    },
                    value_reference: Some(ValueRef::Pool {
                        pair_address: base_contracts.whale_ust_pair.clone(),
                    }),
                },
            ],
            to_remove: vec![],
        },
        &[],
    )
    .unwrap();

    // Add whale to vault claimable assets.
    app.execute_contract(
        owner.clone(),
        vault_dapp_instance.clone(),
        &ExecuteMsg::UpdatePool {
            deposit_asset: None,
            assets_to_add: vec!["whale".to_string()],
            assets_to_remove: vec![],
        },
        &[],
    )
    .unwrap();

    // Add uusd and WHALE to whale/ust pool. Price = 0.5 UST/WHALE
    app.init_bank_balance(
        &base_contracts.whale_ust_pair,
        vec![Coin {
            denom: "uusd".to_string(),
            amount: Uint128::from(1_000u64 * MILLION),
        }],
    )
    .unwrap();

    mint_some_whale(
        app,
        owner.clone(),
        base_contracts.whale.clone(),
        Uint128::from(2_000u64 * MILLION),
        base_contracts.whale_ust_pair.to_string(),
    );

    (vault_dapp_instance, Addr::unchecked(liquidity_token))
}

#[test]
fn proper_initialization() {
    let mut app = mock_app();
    let sender = Addr::unchecked(TEST_CREATOR);
    let (base_contracts, token_code_id) = init_contracts(&mut app);
    let (vault_dapp, vault_l_token) =
        init_vault_dapp(&mut app, sender.clone(), &base_contracts, token_code_id);

    let resp: TreasuryMsg::ConfigResponse = app
        .wrap()
        .query_wasm_smart(&base_contracts.treasury, &TreasuryMsg::QueryMsg::Config {})
        .unwrap();

    // Check config, vault dapp is added
    assert_eq!(1, resp.dapps.len());

    // Add whale and whale_ust token to the memory assets
    // Is tested on unit-test level
    app.execute_contract(
        sender.clone(),
        base_contracts.memory.clone(),
        &MemoryMsg::ExecuteMsg::UpdateAssetAddresses {
            to_add: vec![
                ("whale".to_string(), base_contracts.whale.to_string()),
                (
                    "whale_ust".to_string(),
                    base_contracts.whale_ust.to_string(),
                ),
                ("ust".to_string(), "uusd".to_string()),
            ],
            to_remove: vec![],
        },
        &[],
    )
    .unwrap();

    // Check Memory
    let resp: MemoryMsg::AssetQueryResponse = app
        .wrap()
        .query_wasm_smart(
            &base_contracts.memory,
            &MemoryMsg::QueryMsg::QueryAssets {
                names: vec![
                    "whale".to_string(),
                    "whale_ust".to_string(),
                    "ust".to_string(),
                ],
            },
        )
        .unwrap();

    // Detailed check handled in unit-tests
    assert_eq!("ust".to_string(), resp.assets[0].0);
    assert_eq!("whale".to_string(), resp.assets[1].0);
    assert_eq!("whale_ust".to_string(), resp.assets[2].0);

    // Add whale_ust pair to the memory contracts
    // Is tested on unit-test level
    app.execute_contract(
        sender.clone(),
        base_contracts.memory.clone(),
        &MemoryMsg::ExecuteMsg::UpdateContractAddresses {
            to_add: vec![(
                "whale_ust_pair".to_string(),
                base_contracts.whale_ust_pair.to_string(),
            )],
            to_remove: vec![],
        },
        &[],
    )
    .unwrap();

    // Check Memory
    let resp: MemoryMsg::ContractQueryResponse = app
        .wrap()
        .query_wasm_smart(
            &base_contracts.memory,
            &MemoryMsg::QueryMsg::QueryContracts {
                names: vec!["whale_ust_pair".to_string()],
            },
        )
        .unwrap();

    // Detailed check handled in unit-tests
    assert_eq!("whale_ust_pair".to_string(), resp.contracts[0].0);

    // Check treasury Value
    let treasury_res: TreasuryMsg::TotalValueResponse = app
        .wrap()
        .query_wasm_smart(
            base_contracts.treasury.clone(),
            &TreasuryMsg::QueryMsg::TotalValue {},
        )
        .unwrap();

    assert_eq!(0u128, treasury_res.value.u128());

    // give sender some uusd
    app.init_bank_balance(
        &sender,
        vec![Coin {
            denom: "uusd".to_string(),
            amount: Uint128::from(100u64 * MILLION),
        }],
    )
    .unwrap();

    // Add UST to treasury through vault dapp contract
    app.execute_contract(
        sender.clone(),
        vault_dapp.clone(),
        &ExecuteMsg::ProvideLiquidity {
            asset: Asset {
                info: terraswap::asset::AssetInfo::NativeToken {
                    denom: "uusd".to_string(),
                },
                amount: Uint128::from(10u64 * MILLION),
            },
        },
        &[Coin {
            denom: "uusd".to_string(),
            amount: Uint128::from(10u64 * MILLION),
        }],
    )
    .unwrap();

    // Check treasury Value
    let treasury_res: TreasuryMsg::TotalValueResponse = app
        .wrap()
        .query_wasm_smart(
            base_contracts.treasury.clone(),
            &TreasuryMsg::QueryMsg::TotalValue {},
        )
        .unwrap();

    // Value of vault = deposit
    assert_eq!(10_000_000u128, treasury_res.value.u128());
    // Balance of lp tokens = XX
    let staker_balance: BalanceResponse = app
        .wrap()
        .query_wasm_smart(
            &vault_l_token,
            &Cw20QueryMsg::Balance {
                address: sender.to_string(),
            },
        )
        .unwrap();

    // token balance = sent balance
    assert_eq!(10_000_000u128, staker_balance.balance.u128());

    // add some whale to the treasury
    mint_some_whale(
        &mut app,
        sender.clone(),
        base_contracts.whale.clone(),
        Uint128::from(2_000u64 * MILLION),
        base_contracts.treasury.to_string(),
    );

    // Check treasury Value
    let treasury_res: TreasuryMsg::TotalValueResponse = app
        .wrap()
        .query_wasm_smart(
            base_contracts.treasury.clone(),
            &TreasuryMsg::QueryMsg::TotalValue {},
        )
        .unwrap();

    // Value should be 10_000_000 UST + 0.5 UST/WHALE * 2_000u64*MILLION WHALE
    assert_eq!(
        (10_000_000u64 + 2_000u64 * MILLION / 2) as u128,
        treasury_res.value.u128()
    );

    // Withdraw from vault.
    app.execute_contract(
        sender.clone(),
        vault_l_token.clone(),
        &cw20::Cw20ExecuteMsg::Send {
            contract: vault_dapp.to_string(),
            amount: Uint128::from(10_000_000u128),
            msg: to_binary(&DepositHookMsg::WithdrawLiquidity {}).unwrap(),
        },
        &[],
    )
    .unwrap();

    // Check treasury Value
    let treasury_res: TreasuryMsg::TotalValueResponse = app
        .wrap()
        .query_wasm_smart(
            base_contracts.treasury.clone(),
            &TreasuryMsg::QueryMsg::TotalValue {},
        )
        .unwrap();
    // 10% fee so 10% remains in the pool
    assert_eq!(
        ((10_000_000u64 + 2_000u64 * MILLION / 2) / 10) as u128,
        treasury_res.value.u128()
    );

    //
    assert_eq!(0u128, treasury_res.value.u128());
}
