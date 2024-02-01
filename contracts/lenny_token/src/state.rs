use cosmwasm_std::Addr;
use cw_storage_plus::Item;

pub const SWAP_TAX_TO_TOKEN: Item<Addr> = Item::new("swap_tax_to_token");
