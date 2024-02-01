use cosmwasm_std::{
    to_json_binary, Attribute, BankMsg, Binary, Coin, Decimal, Deps, DepsMut, Env, Fraction,
    MessageInfo, QueryRequest, Response, StdResult, Uint128, WasmMsg, WasmQuery,
};
use cw2::set_contract_version;
use cw20::{BalanceResponse as Cw20BalanceResponse, Cw20ExecuteMsg, Cw20QueryMsg};

use crate::{error::ContractError, msg::*, state::OWNER, state::*};

// version info for migration info
const CONTRACT_NAME: &str = "burning";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

pub fn instantiate(
    deps: DepsMut,
    _env: Env,
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
        ExecuteMsg::Burn {} => execute::burn(deps, env, info),
    }
}

pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Owner {} => to_json_binary(&query::owner(deps)?),
        QueryMsg::DevelopmentConfig {} => to_json_binary(&query::development_config(deps)?),
        QueryMsg::RewardWhitelist {} => to_json_binary(&query::reward_whitelist(deps)?),
        QueryMsg::BurnedAmount {} => to_json_binary(&query::burned_amount(deps)?),
    }
}

mod execute {

    use super::*;

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

    pub fn burn(deps: DepsMut, env: Env, info: MessageInfo) -> Result<Response, ContractError> {
        // check lunc in funds
        let funds = info.funds.clone();
        let mut burn_amount = Uint128::zero();
        for coin in funds {
            if coin.denom == "uluna" {
                burn_amount = coin.amount;
                break;
            }
        }
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
                amount: development_fee * Decimal::percent(99u64), // 1% terra tax
            }],
        };

        let actual_burn_amount = burn_amount - development_fee;

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
                    recipient: info.sender.to_string(),
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
            .add_message(burn_msg)
            .add_message(fee_msg)
            .add_messages(reward_msgs)
            .add_attribute("action", "burn")
            .add_attribute("burn_amount", actual_burn_amount)
            .add_attribute("development_fee", development_fee)
            .add_attributes(attrs);
        Ok(res)
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
}
