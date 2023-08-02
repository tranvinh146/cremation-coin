use cosmwasm_std::{Addr, Timestamp};
use cw_storage_plus::Item;

pub const UNLOCK_TIME: Item<Timestamp> = Item::new("unlock_time");
pub const OWNER: Item<Addr> = Item::new("owner");
