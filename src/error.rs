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

    #[error("InvalidFinder: {0}")]
    InvalidFinder(String),

    #[error("PriceTooSmall: {0}")]
    PriceTooSmall(Uint128),

    #[error("Invalid finders fee bps: {0}")]
    InvalidFindersFeeBps(u64),

    #[error("AskExpired")]
    AskExpired {},

    #[error("AskNotActive")]
    AskNotActive {},

    #[error("AskUnchanged")]
    AskUnchanged {},

    #[error("Token reserved")]
    TokenReserved {},

    #[error("Item not for sale")]
    ItemNotForSale {},

    #[error("InvalidListingFee: {0}")]
    InvalidListingFee(Uint128),

    #[error("InvalidListing/StaleListing")]
    InvalidListing {},

    #[error("BidExpired")]
    BidExpired {},

    #[error("UnauthorizedOwner")]
    UnauthorizedOwner {},

    #[error("Collection not tradable yet")]
    CollectionNotTradable {},
}
