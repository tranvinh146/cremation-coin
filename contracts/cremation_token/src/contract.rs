use crate::{msg::*, state::*};

use classic_terraswap::{
    asset::AssetInfo,
    router::{Cw20HookMsg, SwapOperation},
};
use cosmwasm_std::{
    attr, to_binary, Addr, Binary, CosmosMsg, Deps, DepsMut, Env, MessageInfo, Response, StdError,
    StdResult, Storage, Uint128,
};
use cw2::set_contract_version;
use cw20::Cw20ReceiveMsg;
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
    state::BALANCES,
    ContractError,
};

// version info for migration info
const CONTRACT_NAME: &str = "cremation-token";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

// const CREATE_PAIR_REPLY_ID: u64 = 1;
const SWAP_COLLECTED_TAX_THRESHOLD: Uint128 = Uint128::new(1_000_000_000_000 / 20_000);

pub fn instantiate(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    // Buy - Sell - Transfer Taxes
    TAX_INFO.save(deps.storage, &msg.tax_info)?;
    CREATOR.save(deps.storage, &info.sender)?;
    OWNER.save(deps.storage, &msg.owner)?;
    COLLECT_TAX_ADDRESS.save(deps.storage, &msg.owner)?;
    TAX_FREE_ADDRESSES.save(deps.storage, msg.owner, &true)?;

    cw20_instantiate(deps, env.clone(), info, msg.cw20_instantiate_msg)
}

pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        // ======= Extend executes for cremation-coin =======
        ExecuteMsg::SetConfig {
            terraswap_router,
            terraswap_pair,
        } => execute::set_config(deps, env, info, terraswap_router, terraswap_pair),
        ExecuteMsg::UpdateOwner { new_owner } => execute::update_owner(deps, env, info, new_owner),
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
    use super::*;

    pub fn set_config(
        deps: DepsMut,
        _env: Env,
        info: MessageInfo,
        terraswap_router: Addr,
        terraswap_pair: Addr,
    ) -> Result<Response, ContractError> {
        let creator = CREATOR.load(deps.storage)?;
        if info.sender != creator {
            return Err(ContractError::Unauthorized {});
        }

        if CONFIG.exists(deps.storage) {
            return Err(StdError::generic_err("Config has already initialized").into());
        }

        let config = Config {
            terraswap_router,
            terraswap_pair,
        };
        CONFIG.save(deps.storage, &config)?;
        Ok(Response::new())
    }

    pub fn update_owner(
        deps: DepsMut,
        _env: Env,
        info: MessageInfo,
        new_owner: Addr,
    ) -> Result<Response, ContractError> {
        let current_owner = OWNER.load(deps.storage)?;
        if info.sender != current_owner {
            return Err(ContractError::Unauthorized {});
        }
        OWNER.save(deps.storage, &new_owner)?;
        Ok(Response::new())
    }

    pub fn update_collecting_tax_address(
        deps: DepsMut,
        _env: Env,
        info: MessageInfo,
        new_collect_tax_addr: Addr,
    ) -> Result<Response, ContractError> {
        let owner = OWNER.load(deps.storage)?;
        if info.sender != owner {
            return Err(ContractError::Unauthorized {});
        }
        COLLECT_TAX_ADDRESS.save(deps.storage, &new_collect_tax_addr)?;
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
        address: Addr,
        tax_free: bool,
    ) -> Result<Response, ContractError> {
        let owner = OWNER.load(deps.storage)?;
        if info.sender != owner {
            return Err(ContractError::Unauthorized {});
        }
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
        let config = CONFIG.load(deps.storage)?;
        let sender_addr = info.sender;
        let rcpt_addr = deps.api.addr_validate(&contract)?;
        let tax_amount = compute_tax(deps.storage, &sender_addr, &rcpt_addr, amount);

        update_balance_with_tax(deps.storage, &sender_addr, &rcpt_addr, amount, tax_amount)?;

        let mut attrs = vec![
            attr("action", "send"),
            attr("from", &sender_addr),
            attr("to", &contract),
            attr("amount", amount),
        ];
        if let Some(tax) = tax_amount {
            attrs.push(attr("tax_amount", tax));
        }

        // create a send message
        let mut messages = vec![Cw20ReceiveMsg {
            sender: sender_addr.to_string(),
            amount,
            msg,
        }
        .into_cosmos_msg(contract)?];

        if is_sell_operation(&config, sender_addr, rcpt_addr) {
            let msg_opt = swap_collected_tax_to_native(deps.as_ref(), env, config.terraswap_router);
            if let Some(swap_msg) = msg_opt {
                messages.push(swap_msg)
            }
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
        let config = CONFIG.load(deps.storage)?;
        let owner_addr = deps.api.addr_validate(&owner)?;
        let rcpt_addr = deps.api.addr_validate(&contract)?;
        let tax_amount = compute_tax(deps.storage, &owner_addr, &rcpt_addr, amount);

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
            attrs.push(attr("tax_amount", tax));
        }

        // create a send message
        let mut messages = vec![Cw20ReceiveMsg {
            sender: info.sender.into(),
            amount,
            msg,
        }
        .into_cosmos_msg(contract)?];

        if is_sell_operation(&config, owner_addr, rcpt_addr) {
            let msg_opt = swap_collected_tax_to_native(deps.as_ref(), env, config.terraswap_router);
            if let Some(swap_msg) = msg_opt {
                messages.push(swap_msg)
            }
        }

        let res = Response::new().add_messages(messages).add_attributes(attrs);
        Ok(res)
    }

    pub fn transfer(
        deps: DepsMut,
        env: Env,
        info: MessageInfo,
        recipient: String,
        amount: Uint128,
    ) -> Result<Response, ContractError> {
        let config = CONFIG.load(deps.storage)?;
        let sender_addr = info.sender;
        let rcpt_addr = deps.api.addr_validate(&recipient)?;
        let tax_amount = compute_tax(deps.storage, &sender_addr, &rcpt_addr, amount);

        update_balance_with_tax(deps.storage, &sender_addr, &rcpt_addr, amount, tax_amount)?;

        let mut attrs = vec![
            attr("action", "transfer"),
            attr("from", &sender_addr),
            attr("to", &recipient),
            attr("amount", amount),
        ];
        if let Some(tax) = tax_amount {
            attrs.push(attr("tax_amount", tax));
        }

        if is_sell_operation(&config, sender_addr, rcpt_addr) {
            let msg_opt = swap_collected_tax_to_native(deps.as_ref(), env, config.terraswap_router);
            if let Some(swap_msg) = msg_opt {
                return Ok(Response::new().add_message(swap_msg).add_attributes(attrs));
            }
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
        let config = CONFIG.load(deps.storage)?;
        let rcpt_addr = deps.api.addr_validate(&recipient)?;
        let owner_addr = deps.api.addr_validate(&owner)?;
        let tax_amount = compute_tax(deps.storage, &owner_addr, &rcpt_addr, amount);

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
            attrs.push(attr("tax_amount", tax));
        }

        if is_sell_operation(&config, owner_addr, rcpt_addr) {
            let msg_opt = swap_collected_tax_to_native(deps.as_ref(), env, config.terraswap_router);
            if let Some(swap_msg) = msg_opt {
                return Ok(Response::new()
                    .add_messages(vec![swap_msg])
                    .add_attributes(attrs));
            }
        }

        Ok(Response::new().add_attributes(attrs))
    }

    fn update_balance_with_tax(
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

    fn compute_tax(
        store: &dyn Storage,
        from: &Addr,
        to: &Addr,
        amount: Uint128,
    ) -> Option<Uint128> {
        let config = CONFIG.load(store).unwrap();
        let tax_info = TAX_INFO.load(store).unwrap();

        if TAX_FREE_ADDRESSES.has(store, from.clone()) || TAX_FREE_ADDRESSES.has(store, to.clone())
        {
            return None;
        }

        let is_buy = is_buy_operation(&config, from.to_owned(), to.to_owned());
        let is_sell = is_sell_operation(&config, from.to_owned(), to.to_owned());

        match (is_buy, is_sell) {
            (true, false) => match tax_info.buy_tax {
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
            (false, true) => match tax_info.sell_tax {
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
            _ => match tax_info.transfer_tax {
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
        }
    }

    fn swap_collected_tax_to_native(
        deps: Deps,
        env: Env,
        terraswap_router: Addr,
    ) -> Option<CosmosMsg> {
        // check balance of collected tax address
        let collect_tax_addr = COLLECT_TAX_ADDRESS.load(deps.storage).unwrap();
        let collected_tax_amount = BALANCES.load(deps.storage, &collect_tax_addr).unwrap();
        if collected_tax_amount < SWAP_COLLECTED_TAX_THRESHOLD {
            return None;
        }

        // swap collected tax to native token
        let cw20_receive_msg = Cw20ReceiveMsg {
            sender: collect_tax_addr.to_string(),
            amount: collected_tax_amount,
            msg: to_binary(&Cw20HookMsg::ExecuteSwapOperations {
                operations: vec![SwapOperation::TerraSwap {
                    offer_asset_info: AssetInfo::Token {
                        contract_addr: env.contract.address.into(),
                    },
                    ask_asset_info: AssetInfo::NativeToken {
                        denom: "uluna".to_string(),
                    },
                }],
                minimum_receive: None,
                to: Some(collect_tax_addr.to_string()),
                deadline: None,
            })
            .unwrap(),
        };
        let msg = cw20_receive_msg.into_cosmos_msg(terraswap_router).unwrap();
        Some(msg)
    }

    // receive token from terraswap pair
    fn is_buy_operation(config: &Config, from: Addr, _to: Addr) -> bool {
        from == config.terraswap_pair
    }

    // send token to terraswap router, or terraswap pair
    fn is_sell_operation(config: &Config, from: Addr, to: Addr) -> bool {
        to == config.terraswap_router
            || (from != config.terraswap_router && to == config.terraswap_pair)
    }
}

pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        // ======= Existed queries from cw20-base =======
        QueryMsg::Balance { address } => to_binary(&query_balance(deps, address)?),
        QueryMsg::TokenInfo {} => to_binary(&query_token_info(deps)?),
        QueryMsg::Minter {} => to_binary(&query_minter(deps)?),
        QueryMsg::Allowance { owner, spender } => {
            to_binary(&query_allowance(deps, owner, spender)?)
        }
        QueryMsg::AllAllowances {
            owner,
            start_after,
            limit,
        } => to_binary(&query_owner_allowances(deps, owner, start_after, limit)?),
        QueryMsg::AllSpenderAllowances {
            spender,
            start_after,
            limit,
        } => to_binary(&query_spender_allowances(
            deps,
            spender,
            start_after,
            limit,
        )?),
        QueryMsg::AllAccounts { start_after, limit } => {
            to_binary(&query_all_accounts(deps, start_after, limit)?)
        }
        QueryMsg::MarketingInfo {} => to_binary(&query_marketing_info(deps)?),
        QueryMsg::DownloadLogo {} => to_binary(&query_download_logo(deps)?),

        // ======= Extend queries for cremation-coin =======
        QueryMsg::Config {} => to_binary(&query::config(deps)?),
        QueryMsg::Owner {} => to_binary(&query::owner(deps)?),
        QueryMsg::CollectTaxAddress {} => to_binary(&query::collect_tax_address(deps)?),
        QueryMsg::TaxInfo {} => to_binary(&query::tax_info(deps)?),
        QueryMsg::TaxFreeAddress { address } => to_binary(&query::tax_free_address(deps, address)?),
    }
}

pub mod query {
    use cosmwasm_std::Decimal;

    use super::*;

    pub fn config(deps: Deps) -> StdResult<ConfigResponse> {
        let config = CONFIG.load(deps.storage)?;
        Ok(ConfigResponse {
            terraswap_router: config.terraswap_router,
            terraswap_pair: config.terraswap_pair,
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
