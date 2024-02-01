use cosmwasm_std::{
    attr, entry_point, from_json, to_json_binary, Addr, Binary, Decimal, Deps, DepsMut, Env,
    MessageInfo, QueryRequest, Response, StdResult, Uint128, WasmMsg, WasmQuery,
};
use cw2::set_contract_version;
use cw20::{Cw20ExecuteMsg, Cw20QueryMsg, Cw20ReceiveMsg, MinterResponse, TokenInfoResponse};

use crate::{error::ContractError, msg::*, state::*};

// version info for migration info
const CONTRACT_NAME: &str = "cremation-stake";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

pub const REWARD_INFO: [RewardInfoItem; 3] = [
    RewardInfoItem {
        staking_period: StakingPeriod::Short,
        staking_days: 30,
        reward_rate: Decimal::raw(3 * 10_000_000_000_000_000), // Decimal Places 18
    },
    RewardInfoItem {
        staking_period: StakingPeriod::Medium,
        staking_days: 90,
        reward_rate: Decimal::raw(10 * 10_000_000_000_000_000),
    },
    RewardInfoItem {
        staking_period: StakingPeriod::Long,
        staking_days: 180,
        reward_rate: Decimal::raw(225 * 1_000_000_000_000_000),
    },
];

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> StdResult<Response> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    let minter_query = WasmQuery::Smart {
        contract_addr: msg.token_address.to_string(),
        msg: to_json_binary(&Cw20QueryMsg::Minter {}).unwrap(),
    };
    let minter_res: MinterResponse = deps.querier.query(&QueryRequest::Wasm(minter_query))?;
    let token_info_query = WasmQuery::Smart {
        contract_addr: msg.token_address.to_string(),
        msg: to_json_binary(&Cw20QueryMsg::TokenInfo {}).unwrap(),
    };
    let token_info_res: TokenInfoResponse =
        deps.querier.query(&QueryRequest::Wasm(token_info_query))?;

    let maximum_supply = minter_res.cap.unwrap();
    let current_supply = token_info_res.total_supply;
    let mintable_amount = maximum_supply - current_supply;

    TOTAL_STAKED_AMOUNT.save(deps.storage, &Uint128::zero())?;
    REMAINING_REWARDS.save(deps.storage, &mintable_amount)?;
    TOTAL_PENDING_REWARDS.save(deps.storage, &Uint128::zero())?;
    TOKEN_ADDRESS.save(deps.storage, &msg.token_address)?;

    Ok(Response::default())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::Receive(msg) => execute::receive_cw20(deps, env, info, msg),
        ExecuteMsg::Unstake {} => execute::unstake(deps, env, info),
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::TotalStaked {} => to_json_binary(&query::total_staked(deps)?),
        QueryMsg::Staked { address } => to_json_binary(&query::staked(deps, address)?),
        QueryMsg::RewardInfo {} => to_json_binary(&query::reward_info(deps)?),
        QueryMsg::CanStake {} => to_json_binary(&query::can_stake(deps)?),
        QueryMsg::RemainingRewards {} => to_json_binary(&query::remaining_rewards(deps)?),
        QueryMsg::TotalPendingRewards {} => to_json_binary(&query::total_pending_rewards(deps)?),
    }
}

pub mod execute {
    use super::*;

    pub fn receive_cw20(
        deps: DepsMut,
        env: Env,
        info: MessageInfo,
        cw20_msg: Cw20ReceiveMsg,
    ) -> Result<Response, ContractError> {
        let token_address = TOKEN_ADDRESS.load(deps.storage)?;
        if info.sender != token_address {
            return Err(ContractError::UnsupportedToken {});
        }

        let sender = deps.api.addr_validate(&cw20_msg.sender)?;
        let amount = cw20_msg.amount;

        if amount.is_zero() {
            return Err(ContractError::InvalidStakeAmount {});
        }

        match from_json(&cw20_msg.msg)? {
            Cw20HookMsg::Stake { staking_period } => {
                let is_staked = STAKE.has(deps.storage, &sender);
                if is_staked {
                    return Err(ContractError::AlreadyStaked {});
                }

                let remaining_rewards = REMAINING_REWARDS.load(deps.storage)?;
                let total_pending_rewards = TOTAL_PENDING_REWARDS.load(deps.storage)?;
                let reward_info = REWARD_INFO
                    .iter()
                    .find(|item| item.staking_period == staking_period)
                    .unwrap();
                let pending_reward = amount * reward_info.reward_rate;
                if remaining_rewards < total_pending_rewards + pending_reward {
                    return Err(ContractError::InsufficientRewards {});
                }
                TOTAL_PENDING_REWARDS
                    .save(deps.storage, &(total_pending_rewards + pending_reward))?;
                let staked = Staked {
                    staked_amount: amount,
                    start_time: env.block.time,
                    period: staking_period,
                };
                STAKE.save(deps.storage, &sender, &staked)?;
                TOTAL_STAKED_AMOUNT.update(deps.storage, |total| -> StdResult<_> {
                    Ok(total + staked.staked_amount)
                })?;
                Ok(Response::new()
                    .add_attribute("action", "stake")
                    .add_attribute("staker", &sender)
                    .add_attribute("staked_amount", amount)
                    .add_attribute("period", reward_info.staking_days.to_string()))
            }
        }
    }

    pub fn unstake(deps: DepsMut, env: Env, info: MessageInfo) -> Result<Response, ContractError> {
        let sender = info.sender;
        let staked_opt = STAKE.may_load(deps.storage, &sender)?;
        match staked_opt {
            Some(staked) => {
                let token_address = TOKEN_ADDRESS.load(deps.storage)?;
                let reward_info = REWARD_INFO
                    .iter()
                    .find(|item| item.staking_period == staked.period)
                    .unwrap();
                let total_pending_rewards = TOTAL_PENDING_REWARDS.load(deps.storage)?;
                let reward = staked.staked_amount * reward_info.reward_rate;

                TOTAL_PENDING_REWARDS.save(deps.storage, &(total_pending_rewards - reward))?;
                STAKE.remove(deps.storage, &sender);

                let mut messages = vec![WasmMsg::Execute {
                    contract_addr: token_address.to_string(),
                    msg: to_json_binary(&Cw20ExecuteMsg::Transfer {
                        recipient: sender.to_string(),
                        amount: staked.staked_amount,
                    })?,
                    funds: vec![],
                }];
                let mut attrs = vec![
                    attr("action", "unstake"),
                    attr("staker", &sender),
                    attr("unstaked_amount", staked.staked_amount),
                ];
                TOTAL_STAKED_AMOUNT.update(deps.storage, |total| -> StdResult<_> {
                    Ok(total - staked.staked_amount)
                })?;

                let claim_reward_at = staked.start_time.plus_days(reward_info.staking_days);

                if env.block.time >= claim_reward_at {
                    let remaining_rewards = REMAINING_REWARDS.load(deps.storage)?;

                    REMAINING_REWARDS.save(deps.storage, &(remaining_rewards - reward))?;

                    let mint_reward_msg = Cw20ExecuteMsg::Mint {
                        recipient: sender.to_string(),
                        amount: reward,
                    };
                    messages.push(WasmMsg::Execute {
                        contract_addr: token_address.to_string(),
                        msg: to_json_binary(&mint_reward_msg).unwrap(),
                        funds: vec![],
                    });
                    attrs.push(attr("reward", reward));
                }

                Ok(Response::new().add_messages(messages).add_attributes(attrs))
            }
            None => Err(ContractError::NotStaked {}),
        }
    }
}

pub mod query {
    use super::*;

    pub fn total_staked(deps: Deps) -> StdResult<TotalStakedResponse> {
        let total_staked_amount = TOTAL_STAKED_AMOUNT.load(deps.storage)?;
        Ok(TotalStakedResponse {
            total_staked_amount,
        })
    }

    pub fn staked(deps: Deps, address: Addr) -> StdResult<StakedResponse> {
        let staked_opt = STAKE.may_load(deps.storage, &address)?;
        match staked_opt {
            Some(info) => {
                let reward_info = REWARD_INFO
                    .iter()
                    .find(|item| item.staking_period == info.period)
                    .unwrap();
                let pending_reward = info.staked_amount * reward_info.reward_rate;
                let claim_reward_at = info
                    .start_time
                    .plus_days(reward_info.staking_days)
                    .seconds();
                Ok(StakedResponse {
                    staked_amount: info.staked_amount,
                    pending_reward,
                    claim_reward_at,
                })
            }
            None => Ok(StakedResponse {
                staked_amount: Uint128::zero(),
                pending_reward: Uint128::zero(),
                claim_reward_at: 0,
            }),
        }
    }

    pub fn reward_info(deps: Deps) -> StdResult<RewardInfoResponse> {
        Ok(RewardInfoResponse {
            token_reward: TOKEN_ADDRESS.load(deps.storage)?,
            reward_info: REWARD_INFO,
        })
    }

    pub fn can_stake(deps: Deps) -> StdResult<CanStakeResponse> {
        let remaining_rewards = REMAINING_REWARDS.load(deps.storage)?;
        let total_pending_rewards = TOTAL_PENDING_REWARDS.load(deps.storage)?;
        let can_stake = remaining_rewards > total_pending_rewards;
        Ok(CanStakeResponse { can_stake })
    }

    pub fn remaining_rewards(deps: Deps) -> StdResult<RemainingRewardsResponse> {
        let remaining_rewards = REMAINING_REWARDS.load(deps.storage)?;
        Ok(RemainingRewardsResponse { remaining_rewards })
    }

    pub fn total_pending_rewards(deps: Deps) -> StdResult<TotalPendingRewardsResponse> {
        let total_pending_rewards = TOTAL_PENDING_REWARDS.load(deps.storage)?;
        Ok(TotalPendingRewardsResponse {
            total_pending_rewards,
        })
    }
}
