use cosmwasm_std::{
    from_json, to_json_binary, Binary, Coin, Decimal, Deps, DepsMut, Env, MessageInfo,
    QueryRequest, Reply, Response, StdResult, SubMsg, WasmMsg, WasmQuery,
};
use cremation_token::msg::AssetInfo;
use cremation_token::{
    msg::{CollectTaxAddressResponse, QueryMsg as ExtendedCw20QueryMsg},
    state::FractionFormat,
};
use cw20::{BalanceResponse as Cw20BalanceResponse, Cw20ExecuteMsg, Cw20QueryMsg, Cw20ReceiveMsg};

use crate::{error::ContractError, helpers::create_swap_operations, msg::*, state::*};

pub const SWAP_REPLY_ID: u64 = 1;
pub const LUNC_TAX: Decimal = Decimal::permille(5);

pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    let owner = deps.api.addr_validate(&msg.owner)?;
    let swap_router = deps.api.addr_validate(&msg.swap_router)?;

    OWNER.save(deps.storage, &owner)?;
    SWAP_ROUTER.save(deps.storage, &swap_router)?;

    Ok(Response::default())
}

pub fn reply(deps: DepsMut, env: Env, msg: Reply) -> Result<Response, ContractError> {
    if msg.id != SWAP_REPLY_ID {
        return Err(ContractError::InvalidReplyMsg {});
    }

    let mut cached_data = CACHE.load(deps.storage)?;
    if !cached_data.locked {
        return Err(ContractError::AlreadyUnlocked {});
    }

    let token = cached_data.token_address.clone();
    cached_data.locked = false;
    if cached_data.locked {
        CACHE.save(deps.storage, &cached_data)?;
    }

    let cw20_balance_query = WasmQuery::Smart {
        contract_addr: token.to_string(),
        msg: to_json_binary(&Cw20QueryMsg::Balance {
            address: env.contract.address.to_string(),
        })
        .unwrap(),
    };
    let balance_res: Cw20BalanceResponse = deps
        .querier
        .query(&QueryRequest::Wasm(cw20_balance_query))?;
    let buy_amount = balance_res.balance;
    let token_buy_tax = TOKEN_BUY_TAX
        .load(deps.storage, token.clone())
        .unwrap_or_default();
    let tax_amount = buy_amount
        .checked_mul(token_buy_tax.numerator)
        .unwrap()
        .checked_div(token_buy_tax.denominator)
        .unwrap();

    let collect_tax_address_query = WasmQuery::Smart {
        contract_addr: token.to_string(),
        msg: to_json_binary(&ExtendedCw20QueryMsg::CollectTaxAddress {}).unwrap(),
    };
    let collect_tax_address_res: CollectTaxAddressResponse = deps
        .querier
        .query(&QueryRequest::Wasm(collect_tax_address_query))?;
    let collect_tax_address = collect_tax_address_res.collect_tax_address;

    let collect_tax_msg = WasmMsg::Execute {
        contract_addr: token.to_string(),
        msg: to_json_binary(&Cw20ExecuteMsg::Transfer {
            recipient: collect_tax_address.to_string(),
            amount: tax_amount,
        })
        .unwrap(),
        funds: vec![],
    };

    Ok(Response::new()
        .add_message(collect_tax_msg)
        .add_attribute("cw20_tax_amount", tax_amount))
}

pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::UpdateOwner { new_owner } => execute::update_owner(deps, env, info, new_owner),
        ExecuteMsg::UpdateSwapRouter { router } => {
            execute::update_swap_router(deps, env, info, router)
        }
        ExecuteMsg::UpdateTokenBuyTax {
            token_address,
            buy_tax,
        } => execute::update_token_tax_info(deps, env, info, token_address, buy_tax),
        ExecuteMsg::Swap {
            ask_asset,
            swap_paths,
        } => execute::swap(deps, env, info, ask_asset, swap_paths),
        ExecuteMsg::Receive(cw20_msg) => execute::receive_cw20(deps, env, info, cw20_msg),
    }
}

pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Owner {} => to_json_binary(&query::owner(deps)?),
        QueryMsg::SwapRouter {} => to_json_binary(&query::swap_router(deps)?),
        QueryMsg::TokenTaxInfo { token_address } => {
            to_json_binary(&query::token_tax_info(deps, token_address)?)
        }
    }
}

pub mod execute {
    use super::*;

    pub fn update_owner(
        deps: DepsMut,
        _env: Env,
        info: MessageInfo,
        new_owner: String,
    ) -> Result<Response, ContractError> {
        let owner = OWNER.load(deps.storage)?;
        if owner != info.sender {
            return Err(ContractError::Unauthorized {});
        }

        let new_owner = deps.api.addr_validate(&new_owner)?;
        OWNER.save(deps.storage, &new_owner)?;

        Ok(Response::new().add_attribute("owner", new_owner))
    }

    pub fn update_swap_router(
        deps: DepsMut,
        _env: Env,
        info: MessageInfo,
        router: String,
    ) -> Result<Response, ContractError> {
        let owner = OWNER.load(deps.storage)?;
        if owner != info.sender {
            return Err(ContractError::Unauthorized {});
        }

        let swap_router = deps.api.addr_validate(&router)?;
        SWAP_ROUTER.save(deps.storage, &swap_router)?;

        Ok(Response::new().add_attribute("swap_router", swap_router))
    }

    pub fn update_token_tax_info(
        deps: DepsMut,
        _env: Env,
        info: MessageInfo,
        token_address: String,
        buy_tax: FractionFormat,
    ) -> Result<Response, ContractError> {
        let owner = OWNER.load(deps.storage)?;
        if owner != info.sender {
            return Err(ContractError::Unauthorized {});
        }

        let token_address = deps.api.addr_validate(&token_address)?;
        TOKEN_BUY_TAX.save(deps.storage, token_address.clone(), &buy_tax)?;

        let buy_tax = Decimal::from_ratio(buy_tax.numerator, buy_tax.denominator);

        Ok(Response::new()
            .add_attribute("token", token_address)
            .add_attribute("buy_tax", buy_tax.to_string()))
    }

    pub fn swap(
        deps: DepsMut,
        _env: Env,
        info: MessageInfo,
        ask_asset: AssetInfo,
        swap_paths: Vec<AssetInfo>,
    ) -> Result<Response, ContractError> {
        if info.funds.len() == 1 {
            return Err(ContractError::ExpectOnlyOneCoin {});
        }
        let denom = info.funds[0].denom.clone();
        let swap_amount = info.funds[0].amount;
        if swap_amount.is_zero() {
            return Err(ContractError::ZeroAmount {});
        }

        let offer_asset = AssetInfo::NativeToken {
            denom: denom.clone(),
        };
        let swap_operations = create_swap_operations(offer_asset, ask_asset, swap_paths);
        let swap_router = SWAP_ROUTER.load(deps.storage)?;

        let tax = swap_amount * LUNC_TAX;
        let actual_swap_amount = swap_amount - tax;

        let swap_msg = WasmMsg::Execute {
            contract_addr: swap_router.to_string(),
            msg: to_json_binary(&swap_operations).unwrap(),
            funds: vec![Coin {
                denom,
                amount: actual_swap_amount,
            }],
        };
        let swap_submsg = SubMsg::reply_on_success(swap_msg, SWAP_REPLY_ID);

        Ok(Response::new().add_submessage(swap_submsg))
    }

    pub fn receive_cw20(
        deps: DepsMut,
        _env: Env,
        info: MessageInfo,
        cw20_msg: Cw20ReceiveMsg,
    ) -> Result<Response, ContractError> {
        let token_in = info.sender;
        let amount = cw20_msg.amount;

        let buyer = deps.api.addr_validate(&cw20_msg.sender)?;
        let cached_data = CACHE.load(deps.storage)?;
        if cached_data.locked {
            return Err(ContractError::Locked {});
        }
        CACHE.save(
            deps.storage,
            &CacheData {
                locked: true,
                buyer,
                token_address: token_in.clone(),
            },
        )?;

        match from_json(&cw20_msg.msg) {
            Ok(Cw20HookMsg::Swap {
                ask_asset,
                swap_paths,
            }) => {
                let offer_asset = AssetInfo::Token {
                    contract_addr: token_in.to_string(),
                };
                let swap_operations = create_swap_operations(offer_asset, ask_asset, swap_paths);
                let swap_router = SWAP_ROUTER.load(deps.storage)?;
                let cw20_send_msg = WasmMsg::Execute {
                    contract_addr: token_in.to_string(),
                    msg: to_json_binary(&Cw20ExecuteMsg::Send {
                        contract: swap_router.to_string(),
                        amount,
                        msg: to_json_binary(&swap_operations).unwrap(),
                    })?,
                    funds: vec![],
                };
                let cw20_send_submsg = SubMsg::reply_on_success(cw20_send_msg, SWAP_REPLY_ID);

                Ok(Response::new().add_submessage(cw20_send_submsg))
            }
            Err(err) => Err(ContractError::StdError(err)),
        }
    }
}

pub mod query {
    use super::*;

    pub fn owner(deps: Deps) -> StdResult<OwnerResponse> {
        let owner = OWNER.load(deps.storage)?;
        Ok(OwnerResponse { owner })
    }

    pub fn token_tax_info(deps: Deps, token_address: String) -> StdResult<TokenBuyTaxResponse> {
        let token_address = deps.api.addr_validate(&token_address)?;
        let buy_tax = TOKEN_BUY_TAX
            .load(deps.storage, token_address.clone())
            .unwrap_or_default();
        Ok(TokenBuyTaxResponse {
            token_address,
            buy_tax,
        })
    }

    pub fn swap_router(deps: Deps) -> StdResult<SwapRouterResponse> {
        let router = SWAP_ROUTER.load(deps.storage)?;
        Ok(SwapRouterResponse { router })
    }
}
