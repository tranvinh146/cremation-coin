use crate::msg::*;

use cosmwasm_std::{DepsMut, Env, MessageInfo, Response, Uint128};
use cremation_token::{contract::execute as cremation_token_execute, state::*};

use cw2::set_contract_version;
use cw20_base::{
    allowances::{execute_burn_from, execute_decrease_allowance, execute_increase_allowance},
    contract::{
        execute_burn, execute_mint, execute_update_marketing, execute_update_minter,
        execute_upload_logo, instantiate as cw20_instantiate,
    },
    ContractError,
};

// version info for migration info
const CONTRACT_NAME: &str = "lenny-token";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

pub const SWAP_COLLECTED_TAX_THRESHOLD: Uint128 = Uint128::new(10_000 * 1_000_000);

pub fn instantiate(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    let owner = deps.api.addr_validate(&msg.owner)?;

    // Buy - Sell - Transfer Taxes
    TAX_INFO.save(deps.storage, &msg.tax_info)?;
    CREATOR.save(deps.storage, &info.sender)?;
    OWNER.save(deps.storage, &owner)?;
    COLLECT_TAX_ADDRESS.save(deps.storage, &owner)?;
    TAX_FREE_ADDRESSES.save(deps.storage, owner, &true)?;

    cw20_instantiate(deps, env.clone(), info, msg.cw20_instantiate_msg)
}

pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        // ======= Extend executes for lenny-coin =======
        ExecuteMsg::SetDexConfigs {
            terraswap_router,
            terraswap_pairs,
            terraport_router,
            terraport_pairs,
        } => cremation_token_execute::set_dex_configs(
            deps,
            env,
            info,
            terraswap_router,
            terraswap_pairs,
            terraport_router,
            terraport_pairs,
        ),
        ExecuteMsg::UpdateOwner { new_owner } => {
            cremation_token_execute::update_owner(deps, env, info, new_owner)
        }
        ExecuteMsg::AddNewPairs {
            dex,
            pair_addresses,
        } => cremation_token_execute::add_new_pairs(deps, env, info, dex, pair_addresses),
        ExecuteMsg::RemovePair { dex, pair_address } => {
            cremation_token_execute::remove_pair(deps, env, info, dex, pair_address)
        }
        ExecuteMsg::UpdateCollectTaxAddress {
            new_collect_tax_addr,
        } => cremation_token_execute::update_collecting_tax_address(
            deps,
            env,
            info,
            new_collect_tax_addr,
        ),
        ExecuteMsg::UpdateTaxInfo {
            buy_tax,
            sell_tax,
            transfer_tax,
        } => cremation_token_execute::update_tax_info(
            deps,
            env,
            info,
            buy_tax,
            sell_tax,
            transfer_tax,
        ),
        ExecuteMsg::SetTaxFreeAddress { address, tax_free } => {
            cremation_token_execute::set_tax_free_address(deps, env, info, address, tax_free)
        }

        // ======= Existed executes from cw20-base =======
        ExecuteMsg::Send {
            contract,
            amount,
            msg,
        } => cremation_token_execute::send(deps, env, info, contract, amount, msg),
        ExecuteMsg::SendFrom {
            owner,
            contract,
            amount,
            msg,
        } => cremation_token_execute::send_from(deps, env, info, owner, contract, amount, msg),
        ExecuteMsg::Transfer { recipient, amount } => {
            cremation_token_execute::transfer(deps, env, info, recipient, amount)
        }
        ExecuteMsg::TransferFrom {
            owner,
            recipient,
            amount,
        } => cremation_token_execute::transfer_from(deps, env, info, owner, recipient, amount),
        ExecuteMsg::UpdateMinter { new_minter } => {
            execute_update_minter(deps, env, info, new_minter)
        }
        ExecuteMsg::Mint { recipient, amount } => execute_mint(deps, env, info, recipient, amount),
        ExecuteMsg::Burn { amount } => execute_burn(deps, env, info, amount),
        ExecuteMsg::BurnFrom { owner, amount } => execute_burn_from(deps, env, info, owner, amount),
        ExecuteMsg::IncreaseAllowance {
            spender,
            amount,
            expires,
        } => execute_increase_allowance(deps, env, info, spender, amount, expires),
        ExecuteMsg::DecreaseAllowance {
            spender,
            amount,
            expires,
        } => execute_decrease_allowance(deps, env, info, spender, amount, expires),
        ExecuteMsg::UpdateMarketing {
            project,
            description,
            marketing,
        } => execute_update_marketing(deps, env, info, project, description, marketing),
        ExecuteMsg::UploadLogo(logo) => execute_upload_logo(deps, env, info, logo),
    }
}
