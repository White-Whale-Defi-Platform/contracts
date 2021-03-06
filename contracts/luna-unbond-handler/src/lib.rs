use cosmwasm_std::Response;

pub use crate::error::UnbondHandlerError;

mod commands;
pub mod contract;
mod error;
mod queries;
mod serde_option;
pub mod state;

type UnbondHandlerResult = Result<Response, UnbondHandlerError>;

#[cfg(test)]
#[cfg(not(target_arch = "wasm32"))]
pub mod tests;
