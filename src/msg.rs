use crate::state::{Ask, SaleType, TokenId};
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
pub enum AskHookExecuteMsg {
    AskCreatedHook(AskHookMsg),
    AskUpdatedHook(AskHookMsg),
    AskDeletedHook(AskHookMsg),
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {}
