use crate::{msg::*, state::*};

use classic_terraswap::asset::AssetInfo;
use cosmwasm_std::{
    attr, to_json_binary, Addr, Binary, CosmosMsg, DepsMut, Env, MessageInfo, Response, Uint128,
    WasmMsg,
};
use cremation_token::{
    contract::execute as cremation_token_execute,
    msg::{ExecuteSwapOperations, SwapOperation},
    state::*,
};

use cw2::set_contract_version;
use cw20::{AllowanceResponse, Cw20ReceiveMsg};
use cw20_base::{
    allowances::{
        deduct_allowance, execute_burn_from, execute_decrease_allowance, execute_increase_allowance,
    },
    contract::{
        execute_burn, execute_mint, execute_update_marketing, execute_update_minter,
        execute_upload_logo, instantiate as cw20_instantiate,
    },
    state::*,
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
    let swap_tax_to_token = deps.api.addr_validate(&msg.swap_tax_to_token)?;

    // Buy - Sell - Transfer Taxes
    TAX_INFO.save(deps.storage, &msg.tax_info)?;
    CREATOR.save(deps.storage, &info.sender)?;
    OWNER.save(deps.storage, &owner)?;
    COLLECT_TAX_ADDRESS.save(deps.storage, &owner)?;
    TAX_FREE_ADDRESSES.save(deps.storage, owner, &true)?;
    SWAP_TAX_TO_TOKEN.save(deps.storage, &swap_tax_to_token)?;

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
        } => execute::send(deps, env, info, contract, amount, msg),
        ExecuteMsg::SendFrom {
            owner,
            contract,
            amount,
            msg,
        } => execute::send_from(deps, env, info, owner, contract, amount, msg),
        ExecuteMsg::Transfer { recipient, amount } => {
            execute::transfer(deps, env, info, recipient, amount)
        }
        ExecuteMsg::TransferFrom {
            owner,
            recipient,
            amount,
        } => execute::transfer_from(deps, env, info, owner, recipient, amount),
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

pub mod execute {
    use cremation_token::{
        contract::execute::{compute_tax, update_balance_with_tax},
        helper::is_sell_operation,
    };

    use super::*;

    pub fn send(
        deps: DepsMut,
        env: Env,
        info: MessageInfo,
        contract: String,
        amount: Uint128,
        msg: Binary,
    ) -> Result<Response, ContractError> {
        let sender_addr = info.sender;
        let rcpt_addr = deps.api.addr_validate(&contract)?;
        let is_transfer = false;
        let tax_amount = compute_tax(deps.storage, &sender_addr, &rcpt_addr, amount, is_transfer);

        update_balance_with_tax(deps.storage, &sender_addr, &rcpt_addr, amount, tax_amount)?;

        let mut attrs = vec![
            attr("action", "send"),
            attr("from", &sender_addr),
            attr("to", &contract),
            attr("amount", amount),
        ];
        if let Some(tax) = tax_amount {
            attrs.push(attr("cw20_tax_amount", tax));
        }

        // create a send message
        let mut messages = vec![Cw20ReceiveMsg {
            sender: sender_addr.to_string(),
            amount,
            msg,
        }
        .into_cosmos_msg(contract)?];

        let msg_opt = swap_collected_tax_to_cw20(deps, env, &sender_addr, &rcpt_addr)?;
        if let Some(swap_msg) = msg_opt {
            attrs.push(attr("action", "collected_tax_swap"));
            messages.push(swap_msg);
        }

        let res = Response::new().add_messages(messages).add_attributes(attrs);
        Ok(res)
    }

    pub fn send_from(
        deps: DepsMut,
        env: Env,
        info: MessageInfo,
        owner: String,
        contract: String,
        amount: Uint128,
        msg: Binary,
    ) -> Result<Response, ContractError> {
        let owner_addr = deps.api.addr_validate(&owner)?;
        let rcpt_addr = deps.api.addr_validate(&contract)?;
        let is_transfer = false;
        let tax_amount = compute_tax(deps.storage, &owner_addr, &rcpt_addr, amount, is_transfer);

        // deduct allowance before doing anything else have enough allowance
        deduct_allowance(deps.storage, &owner_addr, &info.sender, &env.block, amount)?;
        update_balance_with_tax(deps.storage, &owner_addr, &rcpt_addr, amount, tax_amount)?;

        let mut attrs = vec![
            attr("action", "send_from"),
            attr("from", &owner),
            attr("to", &contract),
            attr("by", &info.sender),
            attr("amount", amount),
        ];
        if let Some(tax) = tax_amount {
            attrs.push(attr("cw20_tax_amount", tax));
        }

        // create a send message
        let mut messages = vec![Cw20ReceiveMsg {
            sender: info.sender.into(),
            amount,
            msg,
        }
        .into_cosmos_msg(contract)?];

        let msg_opt = swap_collected_tax_to_cw20(deps, env, &owner_addr, &rcpt_addr)?;
        if let Some(swap_msg) = msg_opt {
            attrs.push(attr("action", "collected_tax_swap"));
            messages.push(swap_msg)
        }

        let res = Response::new().add_messages(messages).add_attributes(attrs);
        Ok(res)
    }

    pub fn transfer(
        deps: DepsMut,
        _env: Env,
        info: MessageInfo,
        recipient: String,
        amount: Uint128,
    ) -> Result<Response, ContractError> {
        let sender_addr = info.sender;
        let rcpt_addr = deps.api.addr_validate(&recipient)?;
        let is_transfer = true;
        let tax_amount = compute_tax(deps.storage, &sender_addr, &rcpt_addr, amount, is_transfer);

        update_balance_with_tax(deps.storage, &sender_addr, &rcpt_addr, amount, tax_amount)?;

        let mut attrs = vec![
            attr("action", "transfer"),
            attr("from", &sender_addr),
            attr("to", &recipient),
            attr("amount", amount),
        ];
        if let Some(tax) = tax_amount {
            attrs.push(attr("cw20_tax_amount", tax));
        }

        Ok(Response::new().add_attributes(attrs))
    }

    pub fn transfer_from(
        deps: DepsMut,
        env: Env,
        info: MessageInfo,
        owner: String,
        recipient: String,
        amount: Uint128,
    ) -> Result<Response, ContractError> {
        let rcpt_addr = deps.api.addr_validate(&recipient)?;
        let owner_addr = deps.api.addr_validate(&owner)?;
        let is_transfer = true;
        let tax_amount = compute_tax(deps.storage, &owner_addr, &rcpt_addr, amount, is_transfer);

        // deduct allowance before doing anything else have enough allowance
        deduct_allowance(deps.storage, &owner_addr, &info.sender, &env.block, amount)?;
        update_balance_with_tax(deps.storage, &owner_addr, &rcpt_addr, amount, tax_amount)?;

        let mut attrs = vec![
            attr("action", "transfer_from"),
            attr("from", &owner),
            attr("to", &recipient),
            attr("by", &info.sender),
            attr("amount", amount),
        ];
        if let Some(tax) = tax_amount {
            attrs.push(attr("cw20_tax_amount", tax));
        }

        Ok(Response::new().add_attributes(attrs))
    }

    fn swap_collected_tax_to_cw20(
        deps: DepsMut,
        env: Env,
        from: &Addr,
        to: &Addr,
    ) -> Result<Option<CosmosMsg>, ContractError> {
        let dex_configs = DEX_CONFIGS.load(deps.storage)?;

        // Only collect tax with sell operation
        if !is_sell_operation(&dex_configs, from, to) {
            return Ok(None);
        }

        let router;
        let operations;
        let swap_to_token = SWAP_TAX_TO_TOKEN.load(deps.storage)?;
        if dex_configs.terraswap_pairs.contains(to) || to == &dex_configs.terraswap_router {
            router = dex_configs.terraswap_router;
            operations = vec![
                SwapOperation::TerraSwap {
                    offer_asset_info: AssetInfo::Token {
                        contract_addr: env.contract.address.to_string(),
                    },
                    ask_asset_info: AssetInfo::NativeToken {
                        denom: "uluna".to_string(),
                    },
                },
                SwapOperation::TerraSwap {
                    offer_asset_info: AssetInfo::NativeToken {
                        denom: "uluna".to_string(),
                    },
                    ask_asset_info: AssetInfo::Token {
                        contract_addr: swap_to_token.to_string(),
                    },
                },
            ];
        } else if dex_configs.terraport_pairs.contains(to) || to == &dex_configs.terraport_router {
            router = dex_configs.terraport_router;
            operations = vec![
                SwapOperation::TerraPort {
                    offer_asset_info: AssetInfo::Token {
                        contract_addr: env.contract.address.to_string(),
                    },
                    ask_asset_info: AssetInfo::NativeToken {
                        denom: "uluna".to_string(),
                    },
                },
                SwapOperation::TerraPort {
                    offer_asset_info: AssetInfo::NativeToken {
                        denom: "uluna".to_string(),
                    },
                    ask_asset_info: AssetInfo::Token {
                        contract_addr: swap_to_token.to_string(),
                    },
                },
            ];
        } else {
            return Ok(None);
        };

        // check balance of collected tax address
        let collect_tax_addr = COLLECT_TAX_ADDRESS.load(deps.storage).unwrap();
        let collected_tax_amount = BALANCES
            .load(deps.storage, &collect_tax_addr)
            .unwrap_or_default();
        if collected_tax_amount < SWAP_COLLECTED_TAX_THRESHOLD {
            return Ok(None);
        }

        // allow this contract to send collected tax to terraswap router
        let update_fn = |allow: Option<AllowanceResponse>| -> Result<_, ContractError> {
            let mut val = allow.unwrap_or_default();
            val.allowance += collected_tax_amount;
            Ok(val)
        };

        ALLOWANCES.update(
            deps.storage,
            (&collect_tax_addr, &env.contract.address),
            update_fn,
        )?;
        ALLOWANCES_SPENDER.update(
            deps.storage,
            (&env.contract.address, &collect_tax_addr),
            update_fn,
        )?;

        // swap collected tax to native token
        let cw20_send_msg = ExecuteMsg::SendFrom {
            owner: collect_tax_addr.to_string(),
            contract: router.to_string(),
            amount: collected_tax_amount,
            msg: to_json_binary(&ExecuteSwapOperations {
                operations,
                to: Some(collect_tax_addr.to_string()),
                minimum_receive: None,
                deadline: None,
            })
            .unwrap(),
        };
        let msg = WasmMsg::Execute {
            contract_addr: env.contract.address.to_string(),
            msg: to_json_binary(&cw20_send_msg).unwrap(),
            funds: vec![],
        };
        Ok(Some(msg.into()))
    }
}
