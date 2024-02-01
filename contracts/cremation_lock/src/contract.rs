use crate::{error::ContractError, msg::*, state::*};

use cosmwasm_std::{
    to_json_binary, Addr, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult, Uint128,
    WasmMsg,
};
use cw2::set_contract_version;
use cw20::{BalanceResponse, Cw20ExecuteMsg, Cw20QueryMsg};

// version info for migration info
const CONTRACT_NAME: &str = "cremation-lock";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

pub fn instantiate(
    deps: DepsMut,
    env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> StdResult<Response> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    let unlock_time = env.block.time.plus_days(365); // lock 1 year
    UNLOCK_TIME.save(deps.storage, &unlock_time)?;
    OWNER.save(deps.storage, &msg.owner)?;

    Ok(Response::default())
}

pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::UpdateOwner { new_owner } => execute::update_owner(deps, env, info, new_owner),
        ExecuteMsg::Withdraw { token_address } => execute::withdraw(deps, env, info, token_address),
    }
}

mod execute {
    use super::*;

    pub fn update_owner(
        deps: DepsMut,
        _env: Env,
        info: MessageInfo,
        new_owner: Addr,
    ) -> Result<Response, ContractError> {
        let owner = OWNER.load(deps.storage)?;
        if owner != info.sender {
            return Err(ContractError::Unauthorized {});
        }

        OWNER.save(deps.storage, &new_owner)?;

        let res = Response::new()
            .add_attribute("action", "change_owner")
            .add_attribute("owner", new_owner);
        Ok(res)
    }

    pub fn withdraw(
        deps: DepsMut,
        env: Env,
        _info: MessageInfo,
        token_address: Addr,
    ) -> Result<Response, ContractError> {
        let owner = OWNER.load(deps.storage)?;
        let unlock_time = UNLOCK_TIME.load(deps.storage)?;
        // if owner != info.sender {
        //     return Err(ContractError::Unauthorized {});
        // }
        if env.block.time < unlock_time {
            return Err(ContractError::Locked {});
        }

        let query_balance_msg = Cw20QueryMsg::Balance {
            address: env.contract.address.to_string(),
        };
        let balance_res: BalanceResponse = deps
            .querier
            .query_wasm_smart(&token_address, &query_balance_msg)
            .unwrap();
        let locked_amount = balance_res.balance;

        let withdraw_cw20_msg = Cw20ExecuteMsg::Transfer {
            recipient: owner.into(),
            amount: locked_amount,
        };
        let res: Response = Response::new()
            .add_message(WasmMsg::Execute {
                contract_addr: token_address.clone().into(),
                msg: to_json_binary(&withdraw_cw20_msg)?,
                funds: vec![],
            })
            .add_attribute("action", "withdraw")
            .add_attribute("token_address", token_address)
            .add_attribute("amount", locked_amount);
        Ok(res)
    }
}

pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::LockedTokenAmount { token_address } => {
            to_json_binary(&query::locked_token_amount(deps, env, token_address)?)
        }
        QueryMsg::Owner {} => to_json_binary(&query::owner(deps)?),
        QueryMsg::UnlockTime {} => to_json_binary(&query::unlock_time(deps)?),
    }
}

pub mod query {
    use super::*;

    pub fn locked_token_amount(
        deps: Deps,
        env: Env,
        token_address: Addr,
    ) -> StdResult<LockedTokenAmountResponse> {
        let query_balance_msg = Cw20QueryMsg::Balance {
            address: env.contract.address.into(),
        };
        let query_balance_res: StdResult<BalanceResponse> = deps
            .querier
            .query_wasm_smart(&token_address, &query_balance_msg);

        match query_balance_res {
            Ok(balance_res) => {
                let locked_amount = balance_res.balance;
                Ok(LockedTokenAmountResponse {
                    amount: locked_amount,
                })
            }
            Err(_) => Ok(LockedTokenAmountResponse {
                amount: Uint128::zero(),
            }),
        }
    }

    pub fn owner(deps: Deps) -> StdResult<OwnerResponse> {
        let owner = OWNER.load(deps.storage)?;
        Ok(OwnerResponse { owner })
    }

    pub fn unlock_time(deps: Deps) -> StdResult<UnlockTimeResponse> {
        let unlock_time = UNLOCK_TIME.load(deps.storage)?;
        Ok(UnlockTimeResponse { unlock_time })
    }
}
