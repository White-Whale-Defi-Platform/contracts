use cosmwasm_std::from_binary;
use cosmwasm_std::testing::{mock_dependencies, mock_env};
use white_whale::treasury::dapp_base::common_test::{
    MEMORY_CONTRACT, TRADER_CONTRACT, TREASURY_CONTRACT,
};

use white_whale::treasury::dapp_base::msg::{BaseQueryMsg, BaseStateResponse};

use crate::contract::query;
use crate::msg::QueryMsg;
use crate::tests::base_mocks::mocks::mock_instantiate;

#[test]
pub fn test_config_query() {
    let mut deps = mock_dependencies(&[]);
    mock_instantiate(deps.as_mut());
    let env = mock_env();

    let q_res: BaseStateResponse =
        from_binary(&query(deps.as_ref(), env, QueryMsg::Base(BaseQueryMsg::Config {})).unwrap())
            .unwrap();

    assert_eq!(
        q_res,
        BaseStateResponse {
            treasury_address: TREASURY_CONTRACT.to_string(),
            trader: TRADER_CONTRACT.to_string(),
            memory_address: MEMORY_CONTRACT.to_string(),
        }
    )
}
