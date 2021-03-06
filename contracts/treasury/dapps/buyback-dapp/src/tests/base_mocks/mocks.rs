use cosmwasm_std::testing::{mock_env, mock_info};
use cosmwasm_std::DepsMut;

use white_whale::treasury::dapp_base::common_test::{
    MEMORY_CONTRACT, TEST_CREATOR, TRADER_CONTRACT, TREASURY_CONTRACT,
};
use white_whale::treasury::dapp_base::msg::BaseInstantiateMsg;
use cosmwasm_std::{Addr};
use crate::contract::instantiate;
use crate::msg::InstantiateMsg;
pub(crate) fn instantiate_msg() -> InstantiateMsg {
    InstantiateMsg{
        whale_vust_lp: Addr::unchecked("vust_lp"),
        vust_token: Addr::unchecked("vust_token"),
        whale_token: Addr::unchecked("whale_token"),
        base: BaseInstantiateMsg {
            memory_addr: MEMORY_CONTRACT.to_string(),
            treasury_address: TREASURY_CONTRACT.to_string(),
            trader: TRADER_CONTRACT.to_string(),
            }
    }
}

/**
 * Mocks instantiation of the contract.
 */
pub fn mock_instantiate(deps: DepsMut) {
    let info = mock_info(TEST_CREATOR, &[]);
    let _res = instantiate(deps, mock_env(), info, instantiate_msg())
        .expect("contract successfully handles InstantiateMsg");
}

// /**
//  * Mocks adding asset to the [ADDRESS_BOOK].
//  */
// #[allow(dead_code)]
// pub fn mock_add_to_address_book(deps: DepsMut, asset_address_pair: (String, String)) {
//     let env = mock_env();

//     let (asset, address) = asset_address_pair;
//     // add address
//     let msg = ExecuteMsg::Base(BaseExecuteMsg::UpdateAddressBook {
//         to_add: vec![(asset, address)],
//         to_remove: vec![],
//     });

//     let info = mock_info(TEST_CREATOR, &[]);
//     execute(deps, env.clone(), info, msg).unwrap();
// }
