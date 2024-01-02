use cosmwasm_std::{entry_point, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult};
use cw20_base::ContractError;
use msg::{ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg};

pub mod contract;
pub mod msg;
pub mod state;

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(deps: DepsMut, _env: Env, msg: MigrateMsg) -> StdResult<Response> {
    let config = state::CONFIG.load(deps.storage)?;
    let dex_configs = state::DexConfigs {
        terraswap_pairs: vec![config.terraswap_pair],
        terraswap_router: config.terraswap_router,
        terraport_pairs: msg.terraport_pairs,
        terraport_router: msg.terraport_router,
    };
    state::DEX_CONFIGS.save(deps.storage, &dex_configs)?;

    Ok(Response::default())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    contract::instantiate(deps, env, info, msg)
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> StdResult<Binary> {
    contract::query(deps, env, msg)
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    contract::execute(deps, env, info, msg)
}

#[cfg(test)]
mod testing;
