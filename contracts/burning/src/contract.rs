use cosmwasm_std::{
    from_json, to_json_binary, Addr, Attribute, BankMsg, Binary, Coin, Decimal, Deps, DepsMut, Env,
    Fraction, MessageInfo, QueryRequest, Reply, Response, StdResult, SubMsg, Uint128, WasmMsg,
    WasmQuery,
};
use cremation_token::msg::{AssetInfo, RouterExecuteMsg, SwapOperation};
use cw2::set_contract_version;
use cw20::{BalanceResponse as Cw20BalanceResponse, Cw20ExecuteMsg, Cw20QueryMsg, Cw20ReceiveMsg};

use crate::{error::ContractError, msg::*, state::OWNER, state::*};

// version info for migration info
const CONTRACT_NAME: &str = "burning";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

const SWAP_REPLY_ID: u64 = 1;
pub const LUNC_TAX: Decimal = Decimal::permille(5);

pub fn instantiate(
    deps: DepsMut,
    env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    let owner = deps.api.addr_validate(&msg.owner)?;
    OWNER.save(deps.storage, &owner)?;
    BURNED_AMOUNT.save(deps.storage, &Uint128::zero())?;

    let fee_ratio = msg.development_config.fee_ratio;
    if fee_ratio >= Decimal::one() {
        return Err(ContractError::FeeRatioMustBeLessThanOne {});
    }
    DEVELOPMENT_FEE_RATIO.save(deps.storage, &fee_ratio)?;

    let beneficiary = deps
        .api
        .addr_validate(&msg.development_config.beneficiary)?;
    DEVELOPMENT_FEE_BENEFICIARY.save(deps.storage, &beneficiary)?;

    CACHE.save(
        deps.storage,
        &CachedData {
            locked: false,
            burner: env.contract.address,
        },
    )?;

    Ok(Response::default())
}

pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::UpdateDevelopmentConfig {
            fee_ratio,
            beneficiary,
        } => execute::update_development_config(deps, env, info, fee_ratio, beneficiary),
        ExecuteMsg::AddToRewardWhitelist { reward_info } => {
            execute::add_to_reward_whitelist(deps, env, info, reward_info)
        }
        ExecuteMsg::RemoveFromRewardWhitelist { token } => {
            execute::remove_from_reward_whitelist(deps, env, info, token)
        }
        ExecuteMsg::UpdateRewardInfo { reward_info } => {
            execute::update_reward_info(deps, env, info, reward_info)
        }
        ExecuteMsg::Burn {} => {
            // check lunc in funds
            let funds = info.funds.clone();
            let mut burn_amount = Uint128::zero();
            for coin in funds {
                if coin.denom == "uluna" {
                    burn_amount = coin.amount;
                    break;
                }
            }

            execute::burn(deps, env, info.sender, burn_amount)
        }
        ExecuteMsg::SetSwapRouter { router } => execute::set_swap_router(deps, env, info, router),
        ExecuteMsg::SwapAndBurn { denom, swap_paths } => {
            execute::swap_and_burn(deps, env, info, denom, swap_paths)
        }
        ExecuteMsg::Receive(cw20_msg) => execute::receive_cw20(deps, env, info, cw20_msg),
    }
}

pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Owner {} => to_json_binary(&query::owner(deps)?),
        QueryMsg::DevelopmentConfig {} => to_json_binary(&query::development_config(deps)?),
        QueryMsg::RewardWhitelist {} => to_json_binary(&query::reward_whitelist(deps)?),
        QueryMsg::BurnedAmount {} => to_json_binary(&query::burned_amount(deps)?),
        QueryMsg::SwapRouter {} => to_json_binary(&query::swap_router(deps)?),
    }
}

pub fn reply(deps: DepsMut, env: Env, msg: Reply) -> Result<Response, ContractError> {
    if msg.id != SWAP_REPLY_ID {
        return Err(ContractError::InvalidReplyMsg {});
    }

    let burn_amount = deps
        .querier
        .query_balance(&env.contract.address, "uluna")
        .unwrap()
        .amount;
    let mut cached_data = CACHE.load(deps.storage)?;
    let burner = cached_data.burner.clone();
    cached_data.locked = false;
    CACHE.save(deps.storage, &cached_data)?;

    execute::burn(deps, env.clone(), burner, burn_amount)
}

mod execute {
    use super::*;

    pub fn set_swap_router(
        deps: DepsMut,
        _env: Env,
        info: MessageInfo,
        router: String,
    ) -> Result<Response, ContractError> {
        let owner = OWNER.load(deps.storage)?;
        if owner != info.sender {
            return Err(ContractError::Unauthorized {});
        }
        let router = deps.api.addr_validate(&router)?;

        SWAP_ROUTER.save(deps.storage, &router)?;

        let res = Response::new()
            .add_attribute("action", "set_swap_router")
            .add_attribute("router", router);
        Ok(res)
    }

    pub fn update_development_config(
        deps: DepsMut,
        _env: Env,
        info: MessageInfo,
        fee_ratio: Option<Decimal>,
        beneficiary: Option<String>,
    ) -> Result<Response, ContractError> {
        let owner = OWNER.load(deps.storage)?;
        if owner != info.sender {
            return Err(ContractError::Unauthorized {});
        }

        let mut attrs = vec![];
        if let Some(fee_ratio) = fee_ratio {
            if fee_ratio >= Decimal::one() {
                return Err(ContractError::FeeRatioMustBeLessThanOne {});
            }
            DEVELOPMENT_FEE_RATIO.save(deps.storage, &fee_ratio)?;
            attrs.push(Attribute {
                key: "fee_ratio".to_string(),
                value: fee_ratio.to_string(),
            });
        }
        if let Some(beneficiary) = beneficiary {
            let beneficiary = deps.api.addr_validate(&beneficiary)?;
            DEVELOPMENT_FEE_BENEFICIARY.save(deps.storage, &beneficiary)?;
            attrs.push(Attribute {
                key: "beneficiary".to_string(),
                value: beneficiary.to_string(),
            });
        }

        let res = Response::new()
            .add_attribute("action", "update_development_config")
            .add_attributes(attrs);
        Ok(res)
    }

    pub fn add_to_reward_whitelist(
        deps: DepsMut,
        _env: Env,
        info: MessageInfo,
        reward_info: RewardInfo,
    ) -> Result<Response, ContractError> {
        let owner = OWNER.load(deps.storage)?;
        if owner != info.sender {
            return Err(ContractError::Unauthorized {});
        }

        if reward_info.reward_ratio.is_zero() {
            return Err(ContractError::ZeroRatio {});
        }

        let token = deps.api.addr_validate(&reward_info.token)?;
        let existed = REWARD_WHITELIST.has(deps.storage, token.clone());
        if existed {
            return Err(ContractError::AlreadyExists {});
        } else {
            REWARD_WHITELIST.save(deps.storage, token, &reward_info.reward_ratio)?;
        }

        let res = Response::new()
            .add_attribute("action", "add_to_reward_whitelist")
            .add_attribute("token", reward_info.token)
            .add_attribute("reward_ratio", reward_info.reward_ratio.to_string());
        Ok(res)
    }

    pub fn remove_from_reward_whitelist(
        deps: DepsMut,
        _env: Env,
        info: MessageInfo,
        token: String,
    ) -> Result<Response, ContractError> {
        let owner = OWNER.load(deps.storage)?;
        if owner != info.sender {
            return Err(ContractError::Unauthorized {});
        }

        let token = deps.api.addr_validate(&token)?;

        let existed = REWARD_WHITELIST.has(deps.storage, token.clone());
        if !existed {
            return Err(ContractError::NotInWhitelist {});
        } else {
            REWARD_WHITELIST.remove(deps.storage, token.clone());
        }

        let res = Response::new()
            .add_attribute("action", "remove_from_reward_whitelist")
            .add_attribute("token", token);
        Ok(res)
    }

    pub fn update_reward_info(
        deps: DepsMut,
        _env: Env,
        info: MessageInfo,
        reward_info: RewardInfo,
    ) -> Result<Response, ContractError> {
        let owner = OWNER.load(deps.storage)?;
        if owner != info.sender {
            return Err(ContractError::Unauthorized {});
        }

        if reward_info.reward_ratio.is_zero() {
            return Err(ContractError::ZeroRatio {});
        }

        let token = deps.api.addr_validate(&reward_info.token)?;
        let existed = REWARD_WHITELIST.has(deps.storage, token.clone());
        if !existed {
            return Err(ContractError::NotInWhitelist {});
        } else {
            REWARD_WHITELIST.save(deps.storage, token, &reward_info.reward_ratio)?;
        }

        let res = Response::new()
            .add_attribute("action", "update_reward_info")
            .add_attribute("token", reward_info.token)
            .add_attribute("reward_ratio", reward_info.reward_ratio.to_string());
        Ok(res)
    }

    pub fn burn(
        deps: DepsMut,
        env: Env,
        recipient: Addr,
        burn_amount: Uint128,
    ) -> Result<Response, ContractError> {
        if burn_amount.is_zero() {
            return Err(ContractError::ZeroAmount {});
        }

        let fee_beneficiary = DEVELOPMENT_FEE_BENEFICIARY.load(deps.storage)?;
        let fee_ratio = DEVELOPMENT_FEE_RATIO.load(deps.storage)?;
        let development_fee = burn_amount * fee_ratio.numerator() / fee_ratio.denominator();
        let fee_msg = BankMsg::Send {
            to_address: fee_beneficiary.to_string(),
            amount: vec![Coin {
                denom: "uluna".to_string(),
                amount: development_fee,
            }],
        };

        let send_tax = development_fee * LUNC_TAX;
        let actual_burn_amount = burn_amount - (development_fee + send_tax);

        let rewards = REWARD_WHITELIST
            .range(deps.storage, None, None, cosmwasm_std::Order::Ascending)
            .map(|item| {
                item.map(|(token, reward_ratio)| {
                    let reward_amount = actual_burn_amount * reward_ratio;
                    (token, reward_amount)
                })
            })
            .collect::<StdResult<Vec<_>>>()?;

        let mut reward_msgs = vec![];
        let mut attrs = vec![];
        for reward in rewards {
            let (token, mut reward_amount) = reward;
            if reward_amount.is_zero() {
                continue;
            }

            let cw20_balance_query = WasmQuery::Smart {
                contract_addr: token.to_string(),
                msg: to_json_binary(&Cw20QueryMsg::Balance {
                    address: env.contract.address.to_string(),
                })
                .unwrap(),
            };
            let reward_token_balance: Cw20BalanceResponse = deps
                .querier
                .query(&QueryRequest::Wasm(cw20_balance_query))?;

            if reward_token_balance.balance.is_zero() {
                continue;
            }

            if reward_amount > reward_token_balance.balance {
                reward_amount = reward_token_balance.balance;
            }

            reward_msgs.push(WasmMsg::Execute {
                contract_addr: token.to_string(),
                msg: to_json_binary(&Cw20ExecuteMsg::Transfer {
                    recipient: recipient.to_string(),
                    amount: reward_amount,
                })
                .unwrap(),
                funds: vec![],
            });

            attrs.push(Attribute {
                key: "token".to_string(),
                value: token.to_string(),
            });
            attrs.push(Attribute {
                key: "reward".to_string(),
                value: reward_amount.to_string(),
            });
        }

        BURNED_AMOUNT.update(deps.storage, |burned_amount: Uint128| -> StdResult<_> {
            Ok(burned_amount + actual_burn_amount)
        })?;

        let burn_msg = BankMsg::Burn {
            amount: vec![Coin {
                denom: "uluna".to_string(),
                amount: actual_burn_amount,
            }],
        };

        let res = Response::new()
            .add_message(fee_msg)
            .add_message(burn_msg)
            .add_messages(reward_msgs)
            .add_attribute("action", "burn")
            .add_attribute("development_fee", development_fee)
            .add_attribute("burn_amount", actual_burn_amount)
            .add_attributes(attrs);
        Ok(res)
    }

    pub fn swap_and_burn(
        deps: DepsMut,
        _env: Env,
        info: MessageInfo,
        denom: String,
        swap_paths: Vec<AssetInfo>,
    ) -> Result<Response, ContractError> {
        let funds = info.funds.clone();
        let mut swap_amount = Uint128::zero();
        for coin in funds {
            if coin.denom == denom {
                swap_amount = coin.amount;
                break;
            }
        }
        if swap_amount.is_zero() {
            return Err(ContractError::ZeroAmount {});
        }

        let cached_data = CACHE.load(deps.storage)?;
        if cached_data.locked {
            return Err(ContractError::Locked {});
        }
        CACHE.save(
            deps.storage,
            &CachedData {
                locked: true,
                burner: info.sender,
            },
        )?;

        let tax = swap_amount * LUNC_TAX;
        let actual_swap_amount = swap_amount - tax;

        let mut operations = vec![];
        for i in 0..=swap_paths.len() {
            let offer_asset_info = if i == 0 {
                AssetInfo::NativeToken {
                    denom: denom.clone(),
                }
            } else {
                swap_paths[i - 1].clone()
            };

            let ask_asset_info = if i == swap_paths.len() {
                AssetInfo::NativeToken {
                    denom: "uluna".to_string(),
                }
            } else {
                swap_paths[i].clone()
            };

            operations.push(SwapOperation::TerraPort {
                offer_asset_info,
                ask_asset_info,
            });
        }
        let swap_operations = RouterExecuteMsg::ExecuteSwapOperations {
            operations,
            to: None,
            minimum_receive: None,
            deadline: None,
        };

        let swap_router = SWAP_ROUTER.load(deps.storage)?;
        let swap_wasm_msg = WasmMsg::Execute {
            contract_addr: swap_router.to_string(),
            msg: to_json_binary(&swap_operations).unwrap(),
            funds: vec![Coin {
                denom,
                amount: actual_swap_amount,
            }],
        };
        let swap_submsg = SubMsg::reply_on_success(swap_wasm_msg, SWAP_REPLY_ID);
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

        let burner = deps.api.addr_validate(&cw20_msg.sender)?;
        let cached_data = CACHE.load(deps.storage)?;
        if cached_data.locked {
            return Err(ContractError::Locked {});
        }
        CACHE.save(
            deps.storage,
            &CachedData {
                locked: true,
                burner,
            },
        )?;

        match from_json(&cw20_msg.msg) {
            Ok(Cw20HookMsg::SwapAndBurn { swap_paths }) => {
                let mut operations = vec![];
                for i in 0..=swap_paths.len() {
                    let offer_asset_info = if i == 0 {
                        AssetInfo::Token {
                            contract_addr: token_in.to_string(),
                        }
                    } else {
                        swap_paths[i - 1].clone()
                    };

                    let ask_asset_info = if i == swap_paths.len() {
                        AssetInfo::NativeToken {
                            denom: "uluna".to_string(),
                        }
                    } else {
                        swap_paths[i].clone()
                    };

                    operations.push(SwapOperation::TerraPort {
                        offer_asset_info,
                        ask_asset_info,
                    });
                }
                let swap_operations = RouterExecuteMsg::ExecuteSwapOperations {
                    operations,
                    to: None,
                    minimum_receive: None,
                    deadline: None,
                };
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

mod query {
    use super::*;

    pub fn owner(deps: Deps) -> StdResult<OwnerResponse> {
        let owner = OWNER.load(deps.storage)?;
        Ok(OwnerResponse { owner })
    }

    pub fn development_config(deps: Deps) -> StdResult<DevelopmentConfigResponse> {
        let fee_ratio = DEVELOPMENT_FEE_RATIO.load(deps.storage)?;
        let fee_beneficiary = DEVELOPMENT_FEE_BENEFICIARY.load(deps.storage)?;
        let development_config = DevelopmentConfig {
            fee_ratio,
            beneficiary: fee_beneficiary.to_string(),
        };
        Ok(DevelopmentConfigResponse(development_config))
    }

    pub fn reward_whitelist(deps: Deps) -> StdResult<RewardWhitelistResponse> {
        let reward_whitelist = REWARD_WHITELIST
            .range(deps.storage, None, None, cosmwasm_std::Order::Ascending)
            .map(|item| {
                item.map(|(token, reward_ratio)| RewardInfo {
                    token: token.to_string(),
                    reward_ratio,
                })
            })
            .collect::<StdResult<Vec<_>>>()?;
        Ok(RewardWhitelistResponse { reward_whitelist })
    }

    pub fn burned_amount(deps: Deps) -> StdResult<BurnedAmountResponse> {
        let burned_amount = BURNED_AMOUNT.load(deps.storage)?;
        Ok(BurnedAmountResponse { burned_amount })
    }

    pub fn swap_router(deps: Deps) -> StdResult<SwapRouterResponse> {
        let swap_router = SWAP_ROUTER.load(deps.storage);
        match swap_router {
            Ok(router) => Ok(SwapRouterResponse {
                swap_router: Some(router),
            }),
            Err(_) => Ok(SwapRouterResponse { swap_router: None }),
        }
    }
}
