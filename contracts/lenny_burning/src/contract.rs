use cosmwasm_std::{
    from_json, to_json_binary, Addr, Binary, Decimal, Deps, DepsMut, Env, MessageInfo,
    QueryRequest, Response, StdResult, Uint128, WasmMsg, WasmQuery,
};
use cw2::set_contract_version;
use cw20::{BalanceResponse as Cw20BalanceResponse, Cw20ExecuteMsg, Cw20QueryMsg, Cw20ReceiveMsg};

use crate::{error::ContractError, msg::*, state::*};

// version info for migration info
const CONTRACT_NAME: &str = "burning";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

pub fn instantiate(
    deps: DepsMut,
    env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    let owner = deps.api.addr_validate(&msg.owner)?;
    let reward_address = deps.api.addr_validate(&msg.reward_address)?;
    let reward_info = msg.reward_info;
    let burn_limit = msg.burn_limit;
    let burned_today = BurnedToday {
        amount: Uint128::zero(),
        latest_burned: env.block.time.seconds(),
    };

    OWNER.save(deps.storage, &owner)?;
    REWARD_ADDRESS.save(deps.storage, &reward_address)?;
    REWARD_INFO.save(deps.storage, &reward_info)?;
    BURNED_AMOUNT.save(deps.storage, &Uint128::zero())?;
    BURN_LIMIT.save(deps.storage, &burn_limit)?;
    TOTAL_BURNED_TODAY.save(deps.storage, &burned_today)?;

    Ok(Response::default())
}

pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::UpdateOwner { owner } => execute::update_owner(deps, env, info, owner),
        ExecuteMsg::UpdateRewardAddress { address } => {
            execute::update_reward_address(deps, env, info, address)
        }
        ExecuteMsg::UpdateRewardInfo {
            reward_ratio,
            refund_ratio,
        } => execute::update_reward_info(deps, env, info, reward_ratio, refund_ratio),

        ExecuteMsg::UpdateBurnLimit {
            total,
            per_address,
            duration,
        } => execute::update_burn_limit(deps, env, info, total, per_address, duration),

        ExecuteMsg::Receive(cw20_msg) => execute::receive_cw20(deps, env, info, cw20_msg),
    }
}

pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Owner {} => to_json_binary(&query::owner(deps)?),
        QueryMsg::RewardAddress {} => to_json_binary(&query::reward_address(deps)?),
        QueryMsg::RewardInfo {} => to_json_binary(&query::reward_info(deps)?),
        QueryMsg::BurnLimit {} => to_json_binary(&query::burn_limit(deps)?),
        QueryMsg::TotalBurnedToday {} => to_json_binary(&query::total_burned_today(deps, env)?),
        QueryMsg::BurnedTodayByAddress { address } => {
            to_json_binary(&query::burned_today_by_address(deps, env, address)?)
        }
        QueryMsg::BurnedAmount {} => to_json_binary(&query::burned_amount(deps)?),
    }
}

mod execute {
    use super::*;

    pub fn update_owner(
        deps: DepsMut,
        _env: Env,
        info: MessageInfo,
        owner: String,
    ) -> Result<Response, ContractError> {
        let owner_addr = deps.api.addr_validate(&owner)?;
        let sender = info.sender;
        let owner = OWNER.load(deps.storage)?;
        if owner != sender {
            return Err(ContractError::Unauthorized {});
        }
        OWNER.save(deps.storage, &owner_addr)?;
        Ok(Response::default())
    }

    pub fn update_reward_address(
        deps: DepsMut,
        _env: Env,
        info: MessageInfo,
        address: String,
    ) -> Result<Response, ContractError> {
        let owner = OWNER.load(deps.storage)?;
        if owner != info.sender {
            return Err(ContractError::Unauthorized {});
        }
        let reward_address = deps.api.addr_validate(&address)?;
        REWARD_ADDRESS.save(deps.storage, &reward_address)?;
        Ok(Response::default())
    }

    pub fn update_reward_info(
        deps: DepsMut,
        _env: Env,
        info: MessageInfo,
        reward_ratio: Option<Decimal>,
        refund_ratio: Option<Decimal>,
    ) -> Result<Response, ContractError> {
        let owner = OWNER.load(deps.storage)?;
        if owner != info.sender {
            return Err(ContractError::Unauthorized {});
        }
        let mut reward_info = REWARD_INFO.load(deps.storage)?;

        match reward_ratio {
            Some(reward_ratio) => {
                if reward_ratio.is_zero() {
                    return Err(ContractError::ZeroAmount {});
                }
                if reward_ratio > Decimal::one() {
                    return Err(ContractError::RatioMustBeLessThanOne {});
                }
                reward_info.reward_ratio = reward_ratio;
            }
            None => {}
        }

        match refund_ratio {
            Some(refund_ratio) => {
                if refund_ratio.is_zero() {
                    return Err(ContractError::ZeroAmount {});
                }
                if refund_ratio > Decimal::one() {
                    return Err(ContractError::RatioMustBeLessThanOne {});
                }
                reward_info.refund_ratio = refund_ratio;
            }
            None => {}
        }

        REWARD_INFO.save(deps.storage, &reward_info)?;

        Ok(Response::default())
    }

    pub fn update_burn_limit(
        deps: DepsMut,
        _env: Env,
        info: MessageInfo,
        total: Option<Uint128>,
        per_address: Option<Uint128>,
        duration: Option<u64>,
    ) -> Result<Response, ContractError> {
        let owner = OWNER.load(deps.storage)?;
        if owner != info.sender {
            return Err(ContractError::Unauthorized {});
        }
        let mut burn_limit = BURN_LIMIT.load(deps.storage)?;

        match total {
            Some(total) => {
                if total.is_zero() {
                    return Err(ContractError::ZeroAmount {});
                }
                burn_limit.total = total;
            }
            None => {}
        }

        match per_address {
            Some(per_address) => {
                if per_address.is_zero() {
                    return Err(ContractError::ZeroAmount {});
                }
                burn_limit.per_address = per_address;
            }
            None => {}
        }

        match duration {
            Some(duration) => {
                if duration == 0 {
                    return Err(ContractError::ZeroAmount {});
                }
                burn_limit.duration = duration;
            }
            None => {}
        }

        BURN_LIMIT.save(deps.storage, &burn_limit)?;

        Ok(Response::default())
    }

    pub fn receive_cw20(
        deps: DepsMut,
        env: Env,
        info: MessageInfo,
        cw20_msg: Cw20ReceiveMsg,
    ) -> Result<Response, ContractError> {
        match from_json(&cw20_msg.msg) {
            Ok(Cw20HookMsg::Burn {}) => {
                let token = info.sender;
                let burner = deps.api.addr_validate(&cw20_msg.sender)?;
                let amount = cw20_msg.amount;

                burn(deps, env, token, burner, amount)
            }
            Err(err) => Err(ContractError::StdError(err)),
        }
    }

    fn burn(
        deps: DepsMut,
        env: Env,
        token: Addr,
        burner: Addr,
        amount: Uint128,
    ) -> Result<Response, ContractError> {
        if amount.is_zero() {
            return Err(ContractError::ZeroAmount {});
        }
        let reward_address = REWARD_ADDRESS.load(deps.storage)?;
        let reward_info = REWARD_INFO.load(deps.storage)?;
        let burn_limit = BURN_LIMIT.load(deps.storage)?;

        let mut refund_amount = amount * reward_info.refund_ratio;
        let mut reward_amount = amount * reward_info.reward_ratio;

        // check exceed burn limit
        let addr_burned_today_opt =
            BURNED_TODAY_BY_ADDRESS.may_load(deps.storage, burner.clone())?;
        let mut total_burned_today = TOTAL_BURNED_TODAY.load(deps.storage)?;

        let mut addr_burned_today = match addr_burned_today_opt {
            Some(burned_today) => burned_today,
            None => BurnedToday {
                amount: Uint128::zero(),
                latest_burned: env.block.time.seconds(),
            },
        };

        if total_burned_today.latest_burned + burn_limit.duration > env.block.time.seconds() {
            if total_burned_today.amount + amount > burn_limit.total {
                return Err(ContractError::ExceedBurnLimit {});
            }

            total_burned_today.amount += amount;
        } else {
            total_burned_today.amount = amount;
            total_burned_today.latest_burned = env.block.time.seconds();
        }

        if addr_burned_today.latest_burned + burn_limit.duration > env.block.time.seconds() {
            if addr_burned_today.amount + amount > burn_limit.per_address {
                return Err(ContractError::ExceedBurnLimit {});
            }

            addr_burned_today.amount += amount;
        } else {
            addr_burned_today.amount = amount;
            addr_burned_today.latest_burned = env.block.time.seconds();
        }

        // store burned info
        BURNED_TODAY_BY_ADDRESS.save(deps.storage, burner.clone(), &addr_burned_today)?;
        TOTAL_BURNED_TODAY.save(deps.storage, &total_burned_today)?;
        BURNED_AMOUNT.update(deps.storage, |amount| -> StdResult<Uint128> {
            Ok(amount + amount)
        })?;

        // check token reward in contract
        let cw20_balance_query = WasmQuery::Smart {
            contract_addr: token.to_string(),
            msg: to_json_binary(&Cw20QueryMsg::Balance {
                address: env.contract.address.to_string(),
            })
            .unwrap(),
        };
        let reward_token_balance_res: Cw20BalanceResponse = deps
            .querier
            .query(&QueryRequest::Wasm(cw20_balance_query))?;
        let mut reward_token_balance = reward_token_balance_res.balance;

        if reward_token_balance < refund_amount {
            refund_amount = reward_token_balance;
            reward_token_balance = Uint128::zero();
        } else {
            reward_token_balance -= refund_amount;
        }

        if reward_token_balance < reward_amount {
            reward_amount = reward_token_balance;
        }

        let burn_msg = WasmMsg::Execute {
            contract_addr: token.to_string(),
            msg: to_json_binary(&Cw20ExecuteMsg::Burn {
                amount: amount.clone(),
            })
            .unwrap(),
            funds: vec![],
        };
        let refund_msg = WasmMsg::Execute {
            contract_addr: token.to_string(),
            msg: to_json_binary(&Cw20ExecuteMsg::Transfer {
                recipient: burner.to_string(),
                amount: refund_amount,
            })
            .unwrap(),
            funds: vec![],
        };
        let reward_msg = WasmMsg::Execute {
            contract_addr: token.to_string(),
            msg: to_json_binary(&Cw20ExecuteMsg::Transfer {
                recipient: reward_address.to_string(),
                amount: reward_amount,
            })
            .unwrap(),
            funds: vec![],
        };

        let mut cw20_msgs = vec![burn_msg];
        if !refund_amount.is_zero() {
            cw20_msgs.push(refund_msg);
        }
        if !refund_amount.is_zero() {
            cw20_msgs.push(reward_msg);
        }

        let res = Response::new()
            .add_messages(cw20_msgs)
            .add_attribute("action", "burn")
            .add_attribute("burner", burner.to_string())
            .add_attribute("burn_amount", amount.to_string());

        Ok(res)
    }
}

mod query {
    use super::*;

    pub fn owner(deps: Deps) -> StdResult<OwnerResponse> {
        let owner = OWNER.load(deps.storage)?;
        Ok(OwnerResponse { owner })
    }

    pub fn reward_address(deps: Deps) -> StdResult<RewardAddressResponse> {
        let reward_address = REWARD_ADDRESS.load(deps.storage)?;
        Ok(RewardAddressResponse {
            address: reward_address,
        })
    }

    pub fn reward_info(deps: Deps) -> StdResult<RewardInfoResponse> {
        let reward_info = REWARD_INFO.load(deps.storage)?;
        Ok(RewardInfoResponse(reward_info))
    }

    pub fn burn_limit(deps: Deps) -> StdResult<BurnLimitResponse> {
        let burn_limit = BURN_LIMIT.load(deps.storage)?;
        Ok(BurnLimitResponse(burn_limit))
    }

    pub fn total_burned_today(deps: Deps, env: Env) -> StdResult<TotalBurnedTodayResponse> {
        let burn_limit = BURN_LIMIT.load(deps.storage)?;
        let total_burned_today = TOTAL_BURNED_TODAY.load(deps.storage)?;

        let duration = burn_limit.duration;
        let mut amount = total_burned_today.amount;
        if total_burned_today.latest_burned + duration < env.block.time.seconds() {
            amount = Uint128::zero();
        }
        Ok(TotalBurnedTodayResponse { amount })
    }

    pub fn burned_today_by_address(
        deps: Deps,
        env: Env,
        address: String,
    ) -> StdResult<BurnedTodayByAddressResponse> {
        let address = deps.api.addr_validate(&address)?;
        let burned_today = BURNED_TODAY_BY_ADDRESS.may_load(deps.storage, address)?;

        let burn_limit = BURN_LIMIT.load(deps.storage)?;
        let duration = burn_limit.duration;
        let mut amount = Uint128::zero();
        if let Some(burned_today) = burned_today {
            if burned_today.latest_burned + duration > env.block.time.seconds() {
                amount = burned_today.amount;
            }
        }

        Ok(BurnedTodayByAddressResponse { amount })
    }

    pub fn burned_amount(deps: Deps) -> StdResult<BurnedAmountResponse> {
        let burned_amount = BURNED_AMOUNT.load(deps.storage)?;
        Ok(BurnedAmountResponse { burned_amount })
    }
}
