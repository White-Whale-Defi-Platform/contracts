use cosmwasm_std::StdError;
use cw_controllers::AdminError;
use thiserror::Error;
use white_whale::treasury::dapp_base::error::BaseDAppError;

#[derive(Error, Debug, PartialEq)]
pub enum AnchorError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("{0}")]
    Admin(#[from] AdminError),

    #[error("{0}")]
    BaseDAppError(#[from] BaseDAppError),

    #[error("You must provide exactly two assets when adding liquidity")]
    NotTwoAssets {},

    #[error("{} is not part of the provided pool", id)]
    NotInPool { id: String },
}
