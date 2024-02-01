use cosmwasm_std::{Addr, StdError, Uint128};
use cw20_base::ContractError;

use crate::state::*;

// receive token from terraswap pair, or terraport pair
pub fn is_buy_operation(dex_configs: &DexConfigs, from: &Addr, to: &Addr) -> bool {
    let buy_from_terraswap = from != to
        && dex_configs.terraswap_pairs.contains(from)
        && to != dex_configs.terraswap_router;

    let buy_from_terraport = from != to
        && dex_configs.terraport_pairs.contains(from)
        && to != dex_configs.terraport_router;

    buy_from_terraswap || buy_from_terraport
}

// send token to terraswap router, or terraswap pair
// Or send token to terraport router, or terraport pair
pub fn is_sell_operation(dex_configs: &DexConfigs, from: &Addr, to: &Addr) -> bool {
    let sell_to_terraswap = from != to
        && dex_configs.terraswap_pairs.len() > 0
        && (dex_configs.terraswap_pairs.contains(to) || to == dex_configs.terraport_router);

    let sell_to_terraport = from != to
        && dex_configs.terraport_pairs.len() > 0
        && (dex_configs.terraport_pairs.contains(to) || to == dex_configs.terraswap_router);

    let not_from_dex = !(from == dex_configs.terraswap_router
        || dex_configs.terraport_pairs.contains(from)
        || from == dex_configs.terraport_router
        || dex_configs.terraswap_pairs.contains(from));

    (sell_to_terraswap || sell_to_terraport) && not_from_dex
}

pub fn validate_tax_format(tax: &Option<FractionFormat>) -> Result<(), ContractError> {
    match tax {
        Some(tax) => {
            if tax.numerator > tax.denominator
                && tax.denominator * tax.denominator != Uint128::zero()
            {
                return Err(StdError::generic_err("Invalid fraction format").into());
            }
        }
        None => {}
    }
    Ok(())
}
