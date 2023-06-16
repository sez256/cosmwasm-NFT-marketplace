#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{Binary, Deps, DepsMut, Env, MessageInfo, StdResult};
// use cw2::set_contract_version;

use crate::error::ContractError;
use crate::msg::{AskHookMsg, ExecuteMsg, HookAction, InstantiateMsg, QueryMsg};
use crate::state::{
    ask_key, asks, bid_key, bids, Ask, Bid, Order, SaleType, SudoParams, TokenId, ASK_HOOKS,
    SUDO_PARAMS,
};
use cosmwasm_std::{
    coin, Addr, BankMsg, Coin, Decimal, Empty, Event, StdError, Storage, Timestamp, Uint128,
    WasmMsg,
};
use cw721_base::helpers::Cw721Contract;
use cw_utils::{may_pay, maybe_addr, must_pay};
use sg_std::{Response, SubMsg};
use std::cmp::Ordering;
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

pub struct BidInfo {
    collection: Addr,
    token_id: TokenId,
    expires: Timestamp,
    finder: Option<Addr>,
    finders_fee_bps: Option<u64>,
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
        ExecuteMsg::SetBid {
            collection,
            token_id,
            expires,
            finder,
            finders_fee_bps,
            sale_type,
        } => execute_set_bid(
            deps,
            env,
            info,
            sale_type,
            BidInfo {
                collection: api.addr_validate(&collection)?,
                token_id,
                expires,
                finder: maybe_addr(api, finder)?,
                finders_fee_bps,
            },
            false,
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
enum HookReply {
    Ask = 1,
    Sale,
    Bid,
    CollectionBid,
}

impl From<u64> for HookReply {
    fn from(item: u64) -> Self {
        match item {
            1 => HookReply::Ask,
            2 => HookReply::Sale,
            3 => HookReply::Bid,
            4 => HookReply::CollectionBid,
            _ => panic!("invalid reply type"),
        }
    }
}

pub fn execute_set_bid(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    sale_type: SaleType,
    bid_info: BidInfo,
    buy_now: bool,
) -> Result<Response, ContractError> {
    let BidInfo {
        collection,
        token_id,
        finders_fee_bps,
        expires,
        finder,
    } = bid_info;
    let params = SUDO_PARAMS.load(deps.storage)?;

    if let Some(finder) = finder.clone() {
        if info.sender == finder {
            return Err(ContractError::InvalidFinder(
                "bidder cannot be finder".to_string(),
            ));
        }
    }
    let bid_price = must_pay(&info, NATIVE_DENOM)?;
    if bid_price < params.min_price {
        return Err(ContractError::PriceTooSmall(bid_price));
    }
    params.bid_expiry.is_valid(&env.block, expires)?;
    if let Some(finders_fee_bps) = finders_fee_bps {
        if Decimal::percent(finders_fee_bps) > params.max_finders_fee_percent {
            return Err(ContractError::InvalidFindersFeeBps(finders_fee_bps));
        }
    }
    let bidder = info.sender;
    let mut res = Response::new();
    let bid_key = bid_key(&collection, token_id, &bidder);
    let ask_key = ask_key(&collection, token_id);

    if let Some(existing_bid) = bids().may_load(deps.storage, bid_key.clone())? {
        bids().remove(deps.storage, bid_key)?;
        let refund_bidder = BankMsg::Send {
            to_address: bidder.to_string(),
            amount: vec![coin(existing_bid.price.u128(), NATIVE_DENOM)],
        };
        res = res.add_message(refund_bidder)
    }
    let existing_ask = asks().may_load(deps.storage, ask_key.clone())?;

    if let Some(ask) = existing_ask.clone() {
        if ask.is_expired(&env.block) {
            return Err(ContractError::AskExpired {});
        }
        if !ask.is_active {
            return Err(ContractError::AskNotActive {});
        }
        if let Some(reserved_for) = ask.reserve_for {
            if reserved_for != bidder {
                return Err(ContractError::TokenReserved {});
            }
        }
    } else if buy_now {
        return Err(ContractError::ItemNotForSale {});
    }
    let save_bid = |store| -> StdResult<_> {
        let bid = Bid::new(
            collection.clone(),
            token_id,
            bidder.clone(),
            bid_price,
            finders_fee_bps,
            expires,
        );
        store_bid(store, &bid)?;
        Ok(Some(bid))
    };

    let bid = match existing_ask {
        Some(ask) => match ask.sale_type {
            SaleType::FixedPrice => {
                // check if bid matches ask price then execute the sale
                // if the bid is lower than the ask price save the bid
                // otherwise return an error
                match bid_price.cmp(&ask.price) {
                    Ordering::Greater => {
                        return Err(ContractError::InvalidPrice {});
                    }
                    Ordering::Less => save_bid(deps.storage)?,
                    Ordering::Equal => {
                        asks().remove(deps.storage, ask_key)?;
                        let owner = match Cw721Contract::<Empty, Empty>(
                            ask.collection.clone(),
                            PhantomData,
                            PhantomData,
                        )
                        .owner_of(
                            &deps.querier,
                            ask.token_id.to_string(),
                            false,
                        ) {
                            Ok(res) => res.owner,
                            Err(_) => return Err(ContractError::InvalidListing {}),
                        };
                        if ask.seller != owner {
                            return Err(ContractError::InvalidListing {});
                        }
                        finalize_sale(
                            deps.as_ref(),
                            ask,
                            bid_price,
                            bidder.clone(),
                            finder,
                            &mut res,
                        )?;
                        None
                    }
                }
            }
            SaleType::Auction => {
                // check if bid price is equal or greater than ask price then place the bid
                // otherwise return an error
                match bid_price.cmp(&ask.price) {
                    Ordering::Greater => save_bid(deps.storage)?,
                    Ordering::Equal => save_bid(deps.storage)?,
                    Ordering::Less => {
                        return Err(ContractError::InvalidPrice {});
                    }
                }
            }
        },
        None => save_bid(deps.storage)?,
    };
    let hook = if let Some(bid) = bid {
        prepare_bid_hook(deps.as_ref(), &bid, HookAction::Create)?
    } else {
        vec![]
    };

    let event = Event::new("set-bid")
        .add_attribute("collection", collection.to_string())
        .add_attribute("sale_type", sale_type.to_string())
        .add_attribute("token_id", token_id.to_string())
        .add_attribute("bidder", bidder)
        .add_attribute("bid_price", bid_price.to_string())
        .add_attribute("expires", expires.to_string());

    Ok(res.add_submessages(hook).add_event(event))
}

fn store_bid(store: &mut dyn Storage, bid: &Bid) -> StdResult<()> {
    bids().save(
        store,
        bid_key(&bid.collection, bid.token_id, &bid.bidder),
        bid,
    )
}

fn finalize_sale(
    deps: Deps,
    ask: Ask,
    price: Uint128,
    buyer: Addr,
    finder: Option<Addr>,
    res: &mut Response,
) -> StdResult<()> {
    payout(
        deps,
        ask.collection.clone(),
        price,
        ask.funds_recipient
            .clone()
            .unwrap_or_else(|| ask.seller.clone()),
        finder,
        ask.finders_fee_bps,
        res,
    )?;

    let cw721_transfer_msg = Cw721ExecuteMsg::TransferNft {
        token_id: ask.token_id.to_string(),
        recipient: buyer.to_string(),
    };

    let exec_cw721_transfer = WasmMsg::Execute {
        contract_addr: ask.collection.to_string(),
        msg: to_binary(&cw721_transfer_msg)?,
        funds: vec![],
    };
    res.messages.push(SubMsg::new(exec_cw721_transfer));

    res.messages
        .append(&mut prepare_sale_hook(deps, &ask, buyer.clone())?);

    let event = Event::new("finalize-sale")
        .add_attribute("collection", ask.collection.to_string())
        .add_attribute("token_id", ask.token_id.to_string())
        .add_attribute("seller", ask.seller.to_string())
        .add_attribute("buyer", buyer.to_string())
        .add_attribute("price", price.to_string());
    res.events.push(event);

    Ok(())
}

fn prepare_bid_hook(deps: Deps, bid: &Bid, action: HookAction) -> StdResult<Vec<SubMsg>> {
    let submsgs = BID_HOOKS.prepare_hooks(deps.storage, |h| {
        let msg = BidHookMsg { bid: bid.clone() };
        let execute = WasmMsg::Execute {
            contract_addr: h.to_string(),
            msg: msg.into_binary(action.clone())?,
            funds: vec![],
        };
        Ok(SubMsg::reply_on_error(execute, HookReply::Bid as u64))
    })?;

    Ok(submsgs)
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(_deps: Deps, _env: Env, _msg: QueryMsg) -> StdResult<Binary> {
    unimplemented!()
}

#[cfg(test)]
mod tests {}
