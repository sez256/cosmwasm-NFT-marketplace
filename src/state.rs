use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, BlockInfo, Decimal, Timestamp, Uint128};
use cw_storage_plus::{Index, IndexList, IndexedMap, Item, MultiIndex};
use sg_controllers::Hooks;

use std::fmt;

pub type TokenId = u32;

pub const ASK_HOOKS: Hooks = Hooks::new("ask-hooks");
pub const BID_HOOKS: Hooks = Hooks::new("bid-hooks");
pub const SALE_HOOKS: Hooks = Hooks::new("sale-hooks");

#[cw_serde]
pub enum SaleType {
    FixedPrice,
    Auction,
}

impl fmt::Display for SaleType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            SaleType::FixedPrice => write!(f, "fixed_price"),
            SaleType::Auction => write!(f, "auction"),
        }
    }
}

#[cw_serde]
pub struct Ask {
    pub sale_type: SaleType,
    pub collection: Addr,
    pub token_id: TokenId,
    pub seller: Addr,
    pub price: Uint128,
    pub funds_recipient: Option<Addr>,
    pub reserve_for: Option<Addr>,
    pub finders_fee_bps: Option<u64>,
    pub expires_at: Timestamp,
    pub is_active: bool,
}

/// Primary key for asks: (collection, token_id)
pub type AskKey = (Addr, TokenId);
/// Convenience ask key constructor
pub fn ask_key(collection: &Addr, token_id: TokenId) -> AskKey {
    (collection.clone(), token_id)
}

/// Defines indices for accessing Asks
pub struct AskIndicies<'a> {
    pub collection: MultiIndex<'a, Addr, Ask, AskKey>,
    pub collection_price: MultiIndex<'a, (Addr, u128), Ask, AskKey>,
    pub seller: MultiIndex<'a, Addr, Ask, AskKey>,
}

impl<'a> IndexList<Ask> for AskIndicies<'a> {
    fn get_indexes(&'_ self) -> Box<dyn Iterator<Item = &'_ dyn Index<Ask>> + '_> {
        let v: Vec<&dyn Index<Ask>> = vec![&self.collection, &self.collection_price, &self.seller];
        Box::new(v.into_iter())
    }
}

pub fn asks<'a>() -> IndexedMap<'a, AskKey, Ask, AskIndicies<'a>> {
    let indexes = AskIndicies {
        collection: MultiIndex::new(
            |_pk: &[u8], d: &Ask| d.collection.clone(),
            "asks",
            "asks__collection",
        ),
        collection_price: MultiIndex::new(
            |_pk: &[u8], d: &Ask| (d.collection.clone(), d.price.u128()),
            "asks",
            "asks__collection_price",
        ),
        seller: MultiIndex::new(
            |_pk: &[u8], d: &Ask| d.seller.clone(),
            "asks",
            "asks__seller",
        ),
    };
    IndexedMap::new("asks", indexes)
}
