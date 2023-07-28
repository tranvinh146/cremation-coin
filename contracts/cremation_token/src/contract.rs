use crate::{msg::*, state::*};

use cosmwasm_std::{
    attr, to_binary, Addr, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult, Storage,
    Uint128,
};
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

pub fn instantiate(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    // Terraswap config (Router and Pair contract addresses)
    CONFIG.save(deps.storage, &msg.config)?;
    OWNER.save(deps.storage, &msg.owner)?;

    // Buy - Sell - Transfer Taxes
    TAX_INFO.save(deps.storage, &msg.tax_info)?;
    COLLECTING_TAX_ADDRESS.save(deps.storage, &msg.owner)?;

    cw20_instantiate(deps, env, info, msg.cw20_instantiate_msg)
}

pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
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

        // ======= Extend executes for cremation-coin =======
        ExecuteMsg::ChangeOwner { owner } => execute::change_owner(deps, env, info, owner),
        ExecuteMsg::ChangeCollectingTaxAddress { address } => {
            execute::change_collecting_tax_address(deps, env, info, address)
        }
        ExecuteMsg::ChangeTaxInfo {
            buy_tax,
            sell_tax,
            transfer_tax,
        } => execute::change_tax_info(deps, env, info, buy_tax, sell_tax, transfer_tax),
        ExecuteMsg::SetTaxFreeAddress { address, tax_free } => {
            execute::set_tax_free_address(deps, env, info, address, tax_free)
        }
    }
}

pub mod execute {
    use super::*;

    pub fn send(
        deps: DepsMut,
        _env: Env,
        info: MessageInfo,
        contract: String,
        amount: Uint128,
        msg: Binary,
    ) -> Result<Response, ContractError> {
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
        let msg = Cw20ReceiveMsg {
            sender: sender_addr.into(),
            amount,
            msg,
        }
        .into_cosmos_msg(contract)?;

        let res = Response::new().add_message(msg).add_attributes(attrs);
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
        let msg = Cw20ReceiveMsg {
            sender: info.sender.into(),
            amount,
            msg,
        }
        .into_cosmos_msg(contract)?;

        let res = Response::new().add_message(msg).add_attributes(attrs);
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

        let res = Response::new().add_attributes(attrs);
        Ok(res)
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

        let res = Response::new().add_attributes(attrs);
        Ok(res)
    }

    fn update_balance_with_tax(
        storage: &mut dyn Storage,
        from: &Addr,
        to: &Addr,
        amount: Uint128,
        tax_amount: Option<Uint128>,
    ) -> StdResult<()> {
        BALANCES.update(storage, &from, |balance: Option<Uint128>| -> StdResult<_> {
            Ok(balance.unwrap_or_default().checked_sub(amount)?)
        })?;

        match tax_amount {
            Some(tax) => {
                let received_amount = amount.checked_sub(tax)?;
                let collecting_tax_addr = COLLECTING_TAX_ADDRESS.load(storage)?;
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

        match (
            from == &config.terraswap_pair, // receive token from terraswap pair -> buy
            to == &config.terraswap_router, // send token to terraswap router -> sell
        ) {
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

    pub fn change_owner(
        deps: DepsMut,
        _env: Env,
        info: MessageInfo,
        owner: Addr,
    ) -> Result<Response, ContractError> {
        let owner_addr = OWNER.load(deps.storage)?;
        if info.sender != owner_addr {
            return Err(ContractError::Unauthorized {});
        }
        OWNER.save(deps.storage, &owner)?;
        Ok(Response::new())
    }

    pub fn change_collecting_tax_address(
        deps: DepsMut,
        _env: Env,
        info: MessageInfo,
        collecting_tax_address: Addr,
    ) -> Result<Response, ContractError> {
        let owner_addr = OWNER.load(deps.storage)?;
        if info.sender != owner_addr {
            return Err(ContractError::Unauthorized {});
        }
        COLLECTING_TAX_ADDRESS.save(deps.storage, &collecting_tax_address)?;
        Ok(Response::new())
    }

    pub fn change_tax_info(
        deps: DepsMut,
        _env: Env,
        info: MessageInfo,
        buy_tax: Option<FractionFormat>,
        sell_tax: Option<FractionFormat>,
        transfer_tax: Option<FractionFormat>,
    ) -> Result<Response, ContractError> {
        let owner_addr = OWNER.load(deps.storage)?;
        if info.sender != owner_addr {
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
        let owner_addr = OWNER.load(deps.storage)?;
        if info.sender != owner_addr {
            return Err(ContractError::Unauthorized {});
        }
        TAX_FREE_ADDRESSES.save(deps.storage, address, &tax_free)?;
        Ok(Response::new())
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
        QueryMsg::CollectingTaxAddress {} => to_binary(&query::collecting_tax_address(deps)?),
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

    pub fn collecting_tax_address(deps: Deps) -> StdResult<CollectingTaxAddressResponse> {
        let collect_tax_address = COLLECTING_TAX_ADDRESS.load(deps.storage)?;
        Ok(CollectingTaxAddressResponse {
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
        let tax_free = TAX_FREE_ADDRESSES.load(deps.storage, addr.clone())?;
        Ok(TaxFreeAddressResponse {
            address: addr,
            tax_free,
        })
    }
}
