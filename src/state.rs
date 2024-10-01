use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Uint256};
use cw_storage_plus::{Item, Map};

#[cw_serde]
pub struct Config {
    pub cw721_address: Addr,
    pub cw20_address: Addr,
}

#[cw_serde]
pub struct Listing {
    pub nft_id: String,
    pub price: Uint256,
    pub owner: Addr,
    pub tradeable: bool,
}

#[cw_serde]
pub struct Trade {
    pub asked_id: String,
    pub to_trade_id: String,
    pub trader: Addr,
}

#[cw_serde]
pub struct Offer {
    pub asked_id: String,
    pub offerer: Addr,
    pub amount_offered: Uint256,
    pub amount_type: CoinType,
}

#[cw_serde]
pub enum CoinType {
    Native,
    Cw20,
}

pub const CONFIG: Item<Config> = Item::new("config");
pub const LISTINGS: Map<String, Listing> = Map::new("listings"); // (token_id)
pub const TRADES: Map<(String, String), Trade> = Map::new("trades"); // (token_id, address)
pub const OFFERS: Map<(String, String), Offer> = Map::new("offers"); // (token_id, address)
pub const LISTING_COUNTER: Item<u128> = Item::new("listing_counter");
