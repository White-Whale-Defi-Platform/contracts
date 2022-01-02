use cosmwasm_std::testing::{mock_env, mock_info};
use cosmwasm_std::DepsMut;

use white_whale::treasury::dapp_base::msg::{BaseExecuteMsg, BaseInstantiateMsg};
use white_whale_testing::dapp_base::common::{
    MEMORY_CONTRACT, TEST_CREATOR, TRADER_CONTRACT, TREASURY_CONTRACT,
};

use crate::contract::{execute, instantiate};
use crate::msg::ExecuteMsg;

pub(crate) fn instantiate_msg() -> BaseInstantiateMsg {
    BaseInstantiateMsg {
        memory_addr: MEMORY_CONTRACT.to_string(),
        treasury_address: TREASURY_CONTRACT.to_string(),
        trader: TRADER_CONTRACT.to_string(),
    }
}

/**
 * Mocks adding asset to the [ADDRESS_BOOK].
 */
#[allow(dead_code)]
pub fn mock_add_to_address_book(deps: DepsMut, asset_address_pair: (String, String)) {
    let env = mock_env();

    let (asset, address) = asset_address_pair;
    // add address
    let msg = ExecuteMsg::Base(BaseExecuteMsg::UpdateAddressBook {
        to_add: vec![(asset, address)],
        to_remove: vec![],
    });

    let info = mock_info(TEST_CREATOR, &[]);
    execute(deps, env.clone(), info, msg).unwrap();
}
