use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Decimal, Uint128};
use cw_storage_plus::{Item, Map};

pub const OWNER: Item<Addr> = Item::new("owner");

pub const REWARD_WHITELIST: Map<Addr, Decimal> = Map::new("reward_whitelist");

pub const BURNED_AMOUNT: Item<Uint128> = Item::new("burned_amount");

pub const DEVELOPMENT_FEE_RATIO: Item<Decimal> = Item::new("development_fee");
pub const DEVELOPMENT_FEE_BENEFICIARY: Item<Addr> = Item::new("fee_beneficiary");

pub const SWAP_ROUTER: Item<Addr> = Item::new("swap_router");

#[cw_serde]
pub struct CachedData {
    pub locked: bool,
    pub burner: Addr,
}

pub const CACHE: Item<CachedData> = Item::new("cache");
