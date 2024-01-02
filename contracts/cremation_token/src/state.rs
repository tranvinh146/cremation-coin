use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Uint128};
use cw_storage_plus::{Item, Map};

#[cw_serde]
pub struct Config {
    pub terraswap_pair: Addr,
    pub terraswap_router: Addr,
}
pub const CONFIG: Item<Config> = Item::new("config");

#[cw_serde]
pub struct DexConfigs {
    pub terraswap_router: Addr,
    pub terraswap_pairs: Vec<Addr>,
    pub terraport_router: Addr,
    pub terraport_pairs: Vec<Addr>,
}
pub const DEX_CONFIGS: Item<DexConfigs> = Item::new("dex_configs");

#[cw_serde]
pub struct FractionFormat {
    pub numerator: Uint128,
    pub denominator: Uint128,
}

#[cw_serde]
pub struct TaxInfo {
    pub buy_tax: Option<FractionFormat>,
    pub sell_tax: Option<FractionFormat>,
    pub transfer_tax: Option<FractionFormat>,
}
pub const TAX_INFO: Item<TaxInfo> = Item::new("tax_info");

pub const COLLECT_TAX_ADDRESS: Item<Addr> = Item::new("collect_tax_address");
pub const TAX_FREE_ADDRESSES: Map<Addr, bool> = Map::new("tax_free_addresses");

pub const OWNER: Item<Addr> = Item::new("owner");
pub const CREATOR: Item<Addr> = Item::new("creator");
