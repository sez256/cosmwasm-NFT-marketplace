use cosmwasm_std::{StdError, Uint128};
use cw_utils::PaymentError;
use sg_controllers::HookError;
use thiserror::Error;

#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("Unauthorized")]
    Unauthorized {},
    // Add any other custom errors you like here.
    // Look at https://docs.rs/thiserror/1.0.21/thiserror/ for details.
    #[error("Invalid reserve_for address: {reason}")]
    InvalidReserveAddress { reason: String },

    #[error("InvalidPrice")]
    InvalidPrice {},
}
