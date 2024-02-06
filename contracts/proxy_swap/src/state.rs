use cosmwasm_schema::cw_serde;
use cosmwasm_std::Addr;
use cremation_token::state::FractionFormat;
use cw_storage_plus::{Item, Map};

pub const OWNER: Item<Addr> = Item::new("owner");
pub const SWAP_ROUTER: Item<Addr> = Item::new("swap_router");

pub const TOKEN_BUY_TAX: Map<Addr, FractionFormat> = Map::new("token_buy_tax");

#[cw_serde]
pub struct CacheData {
    pub locked: bool,
    pub buyer: Addr,
    pub token_address: Addr,
}
pub const CACHE: Item<CacheData> = Item::new("cache");
