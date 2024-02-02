use crate::{msg::*, state::*};

use classic_terraswap::asset::AssetInfo;
use cosmwasm_std::{
    attr, to_json_binary, Addr, Binary, CosmosMsg, Decimal, Deps, DepsMut, Env, MessageInfo,
    Response, StdError, StdResult, Storage, Uint128, WasmMsg,
};
use cw2::set_contract_version;
use cw20::{AllowanceResponse, Cw20ReceiveMsg};
use cw20_base::{
    allowances::{
        deduct_allowance, execute_burn_from, execute_decrease_allowance,
        execute_increase_allowance, query_allowance,
    },
    contract::{
        execute_burn, execute_mint, execute_update_marketing, execute_update_minter,
        execute_upload_logo, instantiate as cw20_instantiate, query_balance, query_download_logo,
        query_marketing_info, query_minter, query_token_info,
    },
    enumerable::{query_all_accounts, query_owner_allowances, query_spender_allowances},
    state::{ALLOWANCES, ALLOWANCES_SPENDER, BALANCES},
    ContractError,
};

// version info for migration info
const CONTRACT_NAME: &str = "cremation-token";
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
        } => execute::set_dex_configs(
            deps,
            env,
            info,
            terraswap_router,
            terraswap_pairs,
            terraport_router,
            terraport_pairs,
        ),
        ExecuteMsg::UpdateOwner { new_owner } => execute::update_owner(deps, env, info, new_owner),
        ExecuteMsg::AddNewPairs {
            dex,
            pair_addresses,
        } => execute::add_new_pairs(deps, env, info, dex, pair_addresses),
        ExecuteMsg::UpdateCollectTaxAddress {
            new_collect_tax_addr,
        } => execute::update_collecting_tax_address(deps, env, info, new_collect_tax_addr),
        ExecuteMsg::UpdateTaxInfo {
            buy_tax,
            sell_tax,
            transfer_tax,
        } => execute::update_tax_info(deps, env, info, buy_tax, sell_tax, transfer_tax),
        ExecuteMsg::SetTaxFreeAddress { address, tax_free } => {
            execute::set_tax_free_address(deps, env, info, address, tax_free)
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
    use crate::helper::*;

    use super::*;

    pub fn set_dex_configs(
        deps: DepsMut,
        _env: Env,
        info: MessageInfo,
        terraswap_router: String,
        terraswap_pairs: Vec<String>,
        terraport_router: String,
        terraport_pairs: Vec<String>,
    ) -> Result<Response, ContractError> {
        let creator = CREATOR.load(deps.storage)?;
        if info.sender != creator {
            return Err(ContractError::Unauthorized {});
        }

        if DEX_CONFIGS.exists(deps.storage) {
            return Err(StdError::generic_err("Config has already initialized").into());
        }

        let terraswap_router = deps.api.addr_validate(&terraswap_router)?;
        let terraswap_pairs = terraswap_pairs
            .into_iter()
            .map(|pair| deps.api.addr_validate(&pair))
            .collect::<StdResult<Vec<Addr>>>()?;
        let terraport_router = deps.api.addr_validate(&terraport_router)?;
        let terraport_pairs = terraport_pairs
            .into_iter()
            .map(|pair| deps.api.addr_validate(&pair))
            .collect::<StdResult<Vec<Addr>>>()?;

        let config = DexConfigs {
            terraswap_router,
            terraswap_pairs,
            terraport_router,
            terraport_pairs,
        };
        DEX_CONFIGS.save(deps.storage, &config)?;
        Ok(Response::new())
    }

    pub fn add_new_pairs(
        deps: DepsMut,
        _env: Env,
        info: MessageInfo,
        dex: Dex,
        pairs_addresses: Vec<String>,
    ) -> Result<Response, ContractError> {
        let owner = OWNER.load(deps.storage).unwrap();
        if info.sender != owner {
            return Err(ContractError::Unauthorized {});
        }

        let pairs_addresses = pairs_addresses
            .into_iter()
            .map(|pair| deps.api.addr_validate(&pair))
            .collect::<StdResult<Vec<Addr>>>()?;

        let mut dex_configs = DEX_CONFIGS.load(deps.storage).unwrap();
        match dex {
            Dex::Terraswap => {
                dex_configs.terraswap_pairs.extend(pairs_addresses);
            }
            Dex::Terraport => {
                dex_configs.terraport_pairs.extend(pairs_addresses);
            }
        }
        DEX_CONFIGS.save(deps.storage, &dex_configs)?;

        Ok(Response::new())
    }

    pub fn update_owner(
        deps: DepsMut,
        _env: Env,
        info: MessageInfo,
        new_owner: String,
    ) -> Result<Response, ContractError> {
        let current_owner = OWNER.load(deps.storage)?;
        if info.sender != current_owner {
            return Err(ContractError::Unauthorized {});
        }
        let new_owner = deps.api.addr_validate(&new_owner)?;
        OWNER.save(deps.storage, &new_owner)?;
        Ok(Response::new())
    }

    pub fn update_collecting_tax_address(
        deps: DepsMut,
        _env: Env,
        info: MessageInfo,
        new_collect_tax_addr: String,
    ) -> Result<Response, ContractError> {
        let owner = OWNER.load(deps.storage)?;
        if info.sender != owner {
            return Err(ContractError::Unauthorized {});
        }

        let new_collect_tax_addr = deps.api.addr_validate(&new_collect_tax_addr)?;
        let old_collect_tax_address = COLLECT_TAX_ADDRESS.load(deps.storage)?;
        if old_collect_tax_address == new_collect_tax_addr {
            return Err(StdError::generic_err("New address must be different").into());
        }

        COLLECT_TAX_ADDRESS.save(deps.storage, &new_collect_tax_addr)?;

        // TAX_FREE_ADDRESSES.save(deps.storage, old_collect_tax_address, &false)?;
        TAX_FREE_ADDRESSES.save(deps.storage, new_collect_tax_addr, &true)?;

        Ok(Response::new())
    }

    pub fn update_tax_info(
        deps: DepsMut,
        _env: Env,
        info: MessageInfo,
        buy_tax: Option<FractionFormat>,
        sell_tax: Option<FractionFormat>,
        transfer_tax: Option<FractionFormat>,
    ) -> Result<Response, ContractError> {
        let owner = OWNER.load(deps.storage)?;
        if info.sender != owner {
            return Err(ContractError::Unauthorized {});
        }

        validate_tax_format(&buy_tax)?;
        validate_tax_format(&sell_tax)?;
        validate_tax_format(&transfer_tax)?;

        let tax_info = TaxInfo {
            buy_tax,
            sell_tax,
            transfer_tax,
        };
        TAX_INFO.save(deps.storage, &tax_info)?;
        Ok(Response::new())
    }

    pub fn set_tax_free_address(
        deps: DepsMut,
        _env: Env,
        info: MessageInfo,
        address: String,
        tax_free: bool,
    ) -> Result<Response, ContractError> {
        let owner = OWNER.load(deps.storage)?;
        if info.sender != owner {
            return Err(ContractError::Unauthorized {});
        }
        let address = deps.api.addr_validate(&address)?;
        TAX_FREE_ADDRESSES.save(deps.storage, address, &tax_free)?;
        Ok(Response::new())
    }

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

        let msg_opt = swap_collected_tax_to_native(deps, env, &sender_addr, &rcpt_addr)?;
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

        let msg_opt = swap_collected_tax_to_native(deps, env, &owner_addr, &rcpt_addr)?;
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

    pub fn update_balance_with_tax(
        storage: &mut dyn Storage,
        from: &Addr,
        to: &Addr,
        amount: Uint128,
        tax_amount: Option<Uint128>,
    ) -> StdResult<()> {
        // Update sender balance, return error if insufficient funds
        BALANCES.update(storage, &from, |balance: Option<Uint128>| -> StdResult<_> {
            Ok(balance.unwrap_or_default().checked_sub(amount)?)
        })?;

        // update receiver balance
        match tax_amount {
            Some(tax) => {
                let received_amount = amount.checked_sub(tax)?;
                let collecting_tax_addr = COLLECT_TAX_ADDRESS.load(storage)?;
                assert_eq!(received_amount + tax, amount);

                BALANCES.update(storage, &to, |balance: Option<Uint128>| -> StdResult<_> {
                    Ok(balance.unwrap_or_default() + received_amount)
                })?;
                BALANCES.update(
                    storage,
                    &collecting_tax_addr,
                    |balance: Option<Uint128>| -> StdResult<_> {
                        Ok(balance.unwrap_or_default() + tax)
                    },
                )?;
            }
            None => {
                BALANCES.update(storage, &to, |balance: Option<Uint128>| -> StdResult<_> {
                    Ok(balance.unwrap_or_default() + amount)
                })?;
            }
        };

        Ok(())
    }

    pub fn compute_tax(
        store: &dyn Storage,
        from: &Addr,
        to: &Addr,
        amount: Uint128,
        is_transfer: bool,
    ) -> Option<Uint128> {
        let dex_configs = DEX_CONFIGS.load(store).unwrap();
        let tax_info = TAX_INFO.load(store).unwrap();

        if TAX_FREE_ADDRESSES.has(store, from.clone()) || TAX_FREE_ADDRESSES.has(store, to.clone())
        {
            return None;
        }

        let is_buy = tax_info.buy_tax.is_some() && is_buy_operation(&dex_configs, &from, &to);
        let is_sell = tax_info.sell_tax.is_some() && is_sell_operation(&dex_configs, &from, &to);
        let is_transfer = tax_info.transfer_tax.is_some() && is_transfer;

        match (is_transfer, is_buy, is_sell) {
            (true, false, false) => match tax_info.transfer_tax {
                Some(tax) => {
                    let tax_amount = amount
                        .checked_mul(tax.numerator)
                        .unwrap()
                        .checked_div(tax.denominator)
                        .unwrap();
                    return Some(tax_amount);
                }
                None => {
                    return None;
                }
            },
            (_, true, false) => match tax_info.buy_tax {
                Some(tax) => {
                    let tax_amount = amount
                        .checked_mul(tax.numerator)
                        .unwrap()
                        .checked_div(tax.denominator)
                        .unwrap();
                    return Some(tax_amount);
                }
                None => {
                    return None;
                }
            },
            (_, false, true) => match tax_info.sell_tax {
                Some(tax) => {
                    let tax_amount = amount
                        .checked_mul(tax.numerator)
                        .unwrap()
                        .checked_div(tax.denominator)
                        .unwrap();
                    return Some(tax_amount);
                }
                None => {
                    return None;
                }
            },
            _ => None,
        }
    }

    fn swap_collected_tax_to_native(
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
        let swap_operation;
        if dex_configs.terraswap_pairs.contains(to) || to == &dex_configs.terraswap_router {
            router = dex_configs.terraswap_router;
            swap_operation = SwapOperation::TerraSwap {
                offer_asset_info: AssetInfo::Token {
                    contract_addr: env.contract.address.to_string(),
                },
                ask_asset_info: AssetInfo::NativeToken {
                    denom: "uluna".to_string(),
                },
            };
        } else if dex_configs.terraport_pairs.contains(to) || to == &dex_configs.terraport_router {
            router = dex_configs.terraport_router;
            swap_operation = SwapOperation::TerraPort {
                offer_asset_info: AssetInfo::Token {
                    contract_addr: env.contract.address.to_string(),
                },
                ask_asset_info: AssetInfo::NativeToken {
                    denom: "uluna".to_string(),
                },
            };
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
            msg: to_json_binary(&RouterExecuteMsg::ExecuteSwapOperations {
                operations: vec![swap_operation],
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

pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        // ======= Existed queries from cw20-base =======
        QueryMsg::Balance { address } => to_json_binary(&query_balance(deps, address)?),
        QueryMsg::TokenInfo {} => to_json_binary(&query_token_info(deps)?),
        QueryMsg::Minter {} => to_json_binary(&query_minter(deps)?),
        QueryMsg::Allowance { owner, spender } => {
            to_json_binary(&query_allowance(deps, owner, spender)?)
        }
        QueryMsg::AllAllowances {
            owner,
            start_after,
            limit,
        } => to_json_binary(&query_owner_allowances(deps, owner, start_after, limit)?),
        QueryMsg::AllSpenderAllowances {
            spender,
            start_after,
            limit,
        } => to_json_binary(&query_spender_allowances(
            deps,
            spender,
            start_after,
            limit,
        )?),
        QueryMsg::AllAccounts { start_after, limit } => {
            to_json_binary(&query_all_accounts(deps, start_after, limit)?)
        }
        QueryMsg::MarketingInfo {} => to_json_binary(&query_marketing_info(deps)?),
        QueryMsg::DownloadLogo {} => to_json_binary(&query_download_logo(deps)?),

        // ======= Extend queries for lenny-coin =======
        QueryMsg::DexConfigs {} => to_json_binary(&query::dex_configs(deps)?),
        QueryMsg::Owner {} => to_json_binary(&query::owner(deps)?),
        QueryMsg::CollectTaxAddress {} => to_json_binary(&query::collect_tax_address(deps)?),
        QueryMsg::TaxInfo {} => to_json_binary(&query::tax_info(deps)?),
        QueryMsg::TaxFreeAddress { address } => {
            to_json_binary(&query::tax_free_address(deps, address)?)
        }
    }
}

pub mod query {
    use super::*;

    pub fn dex_configs(deps: Deps) -> StdResult<DexConfigsResponse> {
        let dex_configs = DEX_CONFIGS.load(deps.storage)?;
        Ok(DexConfigsResponse {
            terraswap_router: dex_configs.terraswap_router,
            terraswap_pairs: dex_configs.terraswap_pairs,
            terraport_router: dex_configs.terraport_router,
            terraport_pairs: dex_configs.terraport_pairs,
        })
    }

    pub fn owner(deps: Deps) -> StdResult<OwnerResponse> {
        let owner = OWNER.load(deps.storage)?;
        Ok(OwnerResponse { owner })
    }

    pub fn collect_tax_address(deps: Deps) -> StdResult<CollectTaxAddressResponse> {
        let collect_tax_address = COLLECT_TAX_ADDRESS.load(deps.storage)?;
        Ok(CollectTaxAddressResponse {
            collect_tax_address,
        })
    }

    pub fn tax_info(deps: Deps) -> StdResult<TaxInfoResponse> {
        let tax_info = TAX_INFO.load(deps.storage)?;
        let buy_tax = match tax_info.buy_tax {
            Some(tax) => Decimal::from_ratio(tax.numerator, tax.denominator),
            None => Decimal::zero(),
        };
        let sell_tax = match tax_info.sell_tax {
            Some(tax) => Decimal::from_ratio(tax.numerator, tax.denominator),
            None => Decimal::zero(),
        };
        let transfer_tax = match tax_info.transfer_tax {
            Some(tax) => Decimal::from_ratio(tax.numerator, tax.denominator),
            None => Decimal::zero(),
        };
        Ok(TaxInfoResponse {
            buy_tax,
            sell_tax,
            transfer_tax,
        })
    }

    pub fn tax_free_address(deps: Deps, address: String) -> StdResult<TaxFreeAddressResponse> {
        let addr = deps.api.addr_validate(&address)?;
        let tax_free_opt = TAX_FREE_ADDRESSES.may_load(deps.storage, addr.clone())?;
        match tax_free_opt {
            Some(tax_free) => Ok(TaxFreeAddressResponse { tax_free }),
            None => Ok(TaxFreeAddressResponse { tax_free: false }),
        }
    }
}
