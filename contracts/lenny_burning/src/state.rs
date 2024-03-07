use cosmwasm_std::{Addr, Uint128};
use cw_storage_plus::{Item, Map};

use crate::{BurnLimit, BurnedToday, RewardInfo};

pub const OWNER: Item<Addr> = Item::new("owner");
pub const REWARD_ADDRESS: Item<Addr> = Item::new("reward_address");
pub const REWARD_INFO: Item<RewardInfo> = Item::new("reward_info");

pub const BURNED_AMOUNT: Item<Uint128> = Item::new("burned_amount");
pub const BURN_LIMIT: Item<BurnLimit> = Item::new("burn_limit");

pub const TOTAL_BURNED_TODAY: Item<BurnedToday> = Item::new("total_burned_today");
pub const BURNED_TODAY_BY_ADDRESS: Map<Addr, BurnedToday> = Map::new("burned_today_by_address");
