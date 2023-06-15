#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult};
// use cw2::set_contract_version;

use crate::error::ContractError;
use crate::msg::{AskHookMsg, ExecuteMsg, HookAction, InstantiateMsg, QueryMsg};
use crate::state::{ask_key, asks, Ask, SaleType, TokenId, ASK_HOOKS};
use cosmwasm_std::{Addr, Coin, Empty, Event, Storage, Timestamp, Uint128, WasmMsg};
use cw721_base::helpers::Cw721Contract;
use cw_utils::may_pay;
use sg_std::SubMsg;
use std::marker::PhantomData;

pub const NATIVE_DENOM: &str = "CMDX";

/*
// version info for migration info
const CONTRACT_NAME: &str = "crates.io:comdex-nft-marketplace";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");
*/

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    _deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    _msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    Ok(Response::new())
}

pub struct NFTinfo {
    sale_type: SaleType,
    collection: Addr,
    token_id: TokenId,
    price: Coin,
    funds_recipient: Option<Addr>,
    reserve_for: Option<Addr>,
    finders_fee_bps: Option<u64>,
    expires: Timestamp,
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    let api = deps.api;

    match msg {
        ExecuteMsg::SetAsk {
            sale_type,
            collection,
            token_id,
            price,
            funds_recipient,
            reserve_for,
            finders_fee_bps,
            expires,
        } => execute_set_ask(
            deps,
            env,
            info,
            NFTinfo {
                sale_type,
                collection: api.addr_validate(&collection)?,
                token_id,
                price,
                funds_recipient: maybe_addr(api, funds_recipient)?,
                reserve_for: maybe_addr(api, reserve_for)?,
                finders_fee_bps,
                expires,
            },
        ),
    }
}

pub fn execute_set_ask(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    ask_info: NFTinfo,
) -> Result<Response, ContractError> {
    let NFTinfo {
        sale_type,
        collection,
        token_id,
        price,
        funds_recipient,
        reserve_for,
        finders_fee_bps,
        expires,
    } = ask_info;

    price_validate(deps.storage, &price)?;

    Cw721Contract::<Empty, Empty>(collection.clone(), PhantomData, PhantomData).approval(
        &deps.querier,
        token_id.to_string(),
        env.contract.address.to_string(),
        None,
    )?;
    let listing_fee = may_pay(&info, NATIVE_DENOM)?;

    let mut event = Event::new("set-ask")
        .add_attribute("collection", collection.to_string())
        .add_attribute("token_id", token_id.to_string())
        .add_attribute("sale_type", sale_type.to_string());

    if let Some(address) = reserve_for.clone() {
        if address == info.sender {
            return Err(ContractError::InvalidReserveAddress {
                reason: "cannot reserve to the same address".to_string(),
            });
        }
        if sale_type != SaleType::FixedPrice {
            return Err(ContractError::InvalidReserveAddress {
                reason: "can only reserve for fixed_price sales".to_string(),
            });
        }
        event = event.add_attribute("reserve_for", address.to_string());
    };

    let seller = info.sender;

    let ask = Ask {
        sale_type,
        collection,
        token_id,
        seller: seller.clone(),
        price: price.amount,
        funds_recipient,
        reserve_for,
        finders_fee_bps,
        expires_at: expires,
        is_active: true,
    };

    store_ask(deps.storage, &ask)?;
    let mut res = Response::new();
    // if listing_fee > Uint128::zero() {
    //     fair_burn(listing_fee.u128(), None, &mut res);
    // }

    let hook = prepare_ask_hook(deps.as_ref(), &ask, HookAction::Create)?;
    event = event
        .add_attribute("seller", seller)
        .add_attribute("price", price.to_string())
        .add_attribute("expires", expires.to_string());

    Ok(res.add_submessages(hook).add_event(event))
}

fn price_validate(store: &dyn Storage, price: &Coin) -> Result<(), ContractError> {
    if price.amount.is_zero() || price.denom != NATIVE_DENOM {
        return Err(ContractError::InvalidPrice {});
    }

    // if price.amount < SUDO_PARAMS.load(store)?.min_price {
    //     return Err(ContractError::PriceTooSmall(price.amount));
    // }

    Ok(())
}
fn store_ask(store: &mut dyn Storage, ask: &Ask) -> StdResult<()> {
    asks().save(store, ask_key(&ask.collection, ask.token_id), ask)
}

fn prepare_ask_hook(deps: Deps, ask: &Ask, action: HookAction) -> StdResult<Vec<SubMsg>> {
    let submsgs = ASK_HOOKS.prepare_hooks(deps.storage, |h| {
        let msg = AskHookMsg { ask: ask.clone() };
        let execute = WasmMsg::Execute {
            contract_addr: h.to_string(),
            msg: msg.into_binary(action.clone())?,
            funds: vec![],
        };
        Ok(SubMsg::reply_on_error(execute, HookReply::Ask as u64))
    })?;

    Ok(submsgs)
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(_deps: Deps, _env: Env, _msg: QueryMsg) -> StdResult<Binary> {
    unimplemented!()
}

#[cfg(test)]
mod tests {}
