use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Timestamp, Uint128};
use cw_storage_plus::{Item, Map};

use crate::msg::StakingPeriod;

pub const TOKEN_ADDRESS: Item<Addr> = Item::new("token_address");
pub const REMAINING_REWARDS: Item<Uint128> = Item::new("remaining_rewards");
pub const TOTAL_PENDING_REWARDS: Item<Uint128> = Item::new("total_pending_rewards");

#[cw_serde]
pub struct Staked {
    pub staked_amount: Uint128,
    pub start_time: Timestamp,
    pub period: StakingPeriod,
}
pub const STAKE: Map<&Addr, Staked> = Map::new("stake");
