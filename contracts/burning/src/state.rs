use cosmwasm_std::{Addr, Decimal, Uint128};
use cw_storage_plus::{Item, Map};

pub const OWNER: Item<Addr> = Item::new("owner");

pub const REWARD_WHITELIST: Map<Addr, Decimal> = Map::new("reward_whitelist");

pub const BURNED_AMOUNT: Item<Uint128> = Item::new("burned_amount");

pub const DEVELOPMENT_FEE_RATIO: Item<Decimal> = Item::new("development_fee");
pub const DEVELOPMENT_FEE_BENEFICIARY: Item<Addr> = Item::new("fee_beneficiary");