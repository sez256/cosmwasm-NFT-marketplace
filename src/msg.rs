use crate::state::{Ask, Bid, SaleType, TokenId};
use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{to_binary, Addr, Binary, Coin, StdResult, Timestamp};

#[cw_serde]
pub struct InstantiateMsg {}

#[cw_serde]
pub enum ExecuteMsg {
    SetAsk {
        sale_type: SaleType,
        collection: String,
        token_id: TokenId,
        price: Coin,
        funds_recipient: Option<String>,
        reserve_for: Option<String>,
        finders_fee_bps: Option<u64>,
        expires: Timestamp,
    },
    SetBid {
        collection: String,
        token_id: TokenId,
        expires: Timestamp,
        sale_type: SaleType,
        finder: Option<String>,
        finders_fee_bps: Option<u64>,
    },
    BuyNow {
        collection: String,
        token_id: TokenId,
        expires: Timestamp,
        finder: Option<String>,
        finders_fee_bps: Option<u64>,
    },
    AcceptBid {
        collection: String,
        token_id: TokenId,
        bidder: String,
        finder: Option<String>,
    },
}
#[cw_serde]
pub struct BidHookMsg {
    pub bid: Bid,
}
impl BidHookMsg {
    pub fn new(bid: Bid) -> Self {
        BidHookMsg { bid }
    }

    /// serializes the message
    pub fn into_binary(self, action: HookAction) -> StdResult<Binary> {
        let msg = match action {
            HookAction::Create => BidExecuteMsg::BidCreatedHook(self),
            HookAction::Update => BidExecuteMsg::BidUpdatedHook(self),
            HookAction::Delete => BidExecuteMsg::BidDeletedHook(self),
        };
        to_binary(&msg)
    }
}
// This is just a helper to properly serialize the above message
#[cw_serde]
pub enum BidExecuteMsg {
    BidCreatedHook(BidHookMsg),
    BidUpdatedHook(BidHookMsg),
    BidDeletedHook(BidHookMsg),
}

#[cw_serde]
pub enum HookAction {
    Create,
    Update,
    Delete,
}

#[cw_serde]
pub struct AskHookMsg {
    pub ask: Ask,
}

impl AskHookMsg {
    pub fn new(ask: Ask) -> Self {
        AskHookMsg { ask }
    }

    /// serializes the message
    pub fn into_binary(self, action: HookAction) -> StdResult<Binary> {
        let msg = match action {
            HookAction::Create => AskHookExecuteMsg::AskCreatedHook(self),
            HookAction::Update => AskHookExecuteMsg::AskUpdatedHook(self),
            HookAction::Delete => AskHookExecuteMsg::AskDeletedHook(self),
        };
        to_binary(&msg)
    }
}
#[cw_serde]
pub struct SaleHookMsg {
    pub collection: String,
    pub token_id: u32,
    pub price: Coin,
    pub seller: String,
    pub buyer: String,
}

impl SaleHookMsg {
    pub fn new(
        collection: String,
        token_id: u32,
        price: Coin,
        seller: String,
        buyer: String,
    ) -> Self {
        SaleHookMsg {
            collection,
            token_id,
            price,
            seller,
            buyer,
        }
    }

    /// serializes the message
    pub fn into_binary(self) -> StdResult<Binary> {
        let msg = SaleExecuteMsg::SaleHook(self);
        to_binary(&msg)
    }
}
pub struct MintMsg {
    pub owner: String,
    pub token_uri: Option<String>,
    pub price: Vec<Coin>,
}
#[cw_serde]
pub enum SaleExecuteMsg {
    SaleHook(SaleHookMsg),
}
#[cw_serde]
pub enum AskHookExecuteMsg {
    AskCreatedHook(AskHookMsg),
    AskUpdatedHook(AskHookMsg),
    AskDeletedHook(AskHookMsg),
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {}
