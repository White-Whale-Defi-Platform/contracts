use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use cosmwasm_std::{StdError, StdResult};

use terraswap::asset::AssetInfo;


#[derive(Deserialize, Serialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct DepositInfo {
    pub asset_info: AssetInfo
}

impl DepositInfo {
    pub fn assert(&self, asset_info: &AssetInfo) -> StdResult<()> {
        if asset_info == &self.asset_info {
            return Ok(());
        }

        Err(StdError::generic_err(format!("Invalid deposit asset. Expected {}, got {}.", self.asset_info, asset_info)))
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    pub const TEST_DENOM1: &str = "uusd";
    pub const TEST_DENOM2: &str = "uluna";
    pub const TEST_ADDR1: &str = "1234";
    pub const TEST_ADDR2: &str = "4321";

    #[test]
    fn test_failing_assert_for_native_tokens() {
        let deposit_info = DepositInfo{ asset_info: AssetInfo::NativeToken{ denom: TEST_DENOM1.to_string() } };
        let other_native_token = AssetInfo::NativeToken{ denom: TEST_DENOM2.to_string() };
        assert!(deposit_info.assert(&other_native_token).is_err());
    }

    #[test]
    fn test_passing_assert_for_native_tokens() {
        let deposit_info = DepositInfo{ asset_info: AssetInfo::NativeToken{ denom: TEST_DENOM1.to_string() } };
        let other_native_token = AssetInfo::NativeToken{ denom: TEST_DENOM1.to_string() };
        assert!(deposit_info.assert(&other_native_token).is_ok());
    }

    #[test]
    fn test_failing_assert_for_nonnative_tokens() {
        let deposit_info = DepositInfo{ asset_info: AssetInfo::Token{ contract_addr: TEST_ADDR1.to_string() } };
        let other_native_token = AssetInfo::Token{ contract_addr: TEST_ADDR2.to_string() };
        assert!(deposit_info.assert(&other_native_token).is_err());
    }

    #[test]
    fn test_passing_assert_for_nonnative_tokens() {
        let deposit_info = DepositInfo{ asset_info: AssetInfo::Token{ contract_addr: TEST_ADDR1.to_string() } };
        let other_native_token = AssetInfo::Token{ contract_addr: TEST_ADDR1.to_string() };
        assert!(deposit_info.assert(&other_native_token).is_ok());
    }

    #[test]
    fn test_failing_assert_for_mixed_tokens() {
        let deposit_info = DepositInfo{ asset_info: AssetInfo::NativeToken{ denom: TEST_DENOM1.to_string() } };
        let other_native_token = AssetInfo::Token{ contract_addr: TEST_DENOM1.to_string() };
        assert!(deposit_info.assert(&other_native_token).is_err());
    }
}
