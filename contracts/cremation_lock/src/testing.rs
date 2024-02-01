use cosmwasm_std::{
    from_json,
    testing::{mock_dependencies, mock_env, mock_info, MockQuerier},
    to_json_binary, Addr, ContractResult, SystemResult, Uint128, WasmQuery,
};
use cw20::{BalanceResponse as Cw20BalanceResponse, Cw20QueryMsg};

use crate::{
    error::ContractError,
    msg::{ExecuteMsg, InstantiateMsg, OwnerResponse, QueryMsg, UnlockTimeResponse},
    {execute, instantiate, query},
};

#[test]
fn proper_initialization() {
    let mut deps = mock_dependencies();
    let env = mock_env();
    let owner = "owner";

    let msg = InstantiateMsg {
        owner: Addr::unchecked(owner),
    };
    let info = mock_info("deployer", &[]);
    let _res = instantiate(deps.as_mut(), env.clone(), info, msg).unwrap();

    // check owner
    let owner_query = query(deps.as_ref(), env.clone(), QueryMsg::Owner {}).unwrap();
    let owner_res: OwnerResponse = from_json(&owner_query).unwrap();
    assert_eq!(owner_res.owner, owner);

    // check unlock_time
    let unlock_time_query = query(deps.as_ref(), env.clone(), QueryMsg::UnlockTime {}).unwrap();
    let unlock_time_res: UnlockTimeResponse = from_json(&unlock_time_query).unwrap();
    assert_eq!(unlock_time_res.unlock_time, env.block.time.plus_days(365));
}

#[test]
fn update_owner() {
    let mut deps = mock_dependencies();
    let owner = "owner";

    let msg = InstantiateMsg {
        owner: Addr::unchecked(owner),
    };
    let info = mock_info("deployer", &[]);
    let _res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

    // fail to change owner with non-owner
    let non_owner = "non_owner";
    let info = mock_info(non_owner, &[]);
    let msg = ExecuteMsg::UpdateOwner {
        new_owner: Addr::unchecked(non_owner),
    };
    let err = execute(deps.as_mut(), mock_env(), info, msg).unwrap_err();
    assert_eq!(err, ContractError::Unauthorized {});

    // change owner
    let new_owner = "new_owner";
    let info = mock_info(owner, &[]);
    let msg: ExecuteMsg = ExecuteMsg::UpdateOwner {
        new_owner: Addr::unchecked(new_owner),
    };
    let _res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();

    // check new owner
    let owner_query = query(deps.as_ref(), mock_env(), QueryMsg::Owner {}).unwrap();
    let owner_res: OwnerResponse = from_json(&owner_query).unwrap();
    assert_eq!(owner_res.owner, new_owner);
}

#[test]
fn fail_to_withdraw_when_in_lock_time() {
    let mut deps = mock_dependencies();
    let owner = "owner";

    let msg = InstantiateMsg {
        owner: Addr::unchecked(owner),
    };
    let info = mock_info("deployer", &[]);
    let _res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

    // fail to withdraw when in-lock time
    let info = mock_info(owner, &[]);
    let msg = ExecuteMsg::Withdraw {
        token_address: Addr::unchecked("token_addr"),
    };
    let err = execute(deps.as_mut(), mock_env(), info, msg).unwrap_err();
    assert_eq!(err, ContractError::Locked {});
}

#[test]
fn withdraw() {
    let mut deps = mock_dependencies();
    let owner = "owner";
    let lock_contract_addr = "lock_contract_addr";
    let token_addr = "token_addr";
    let locked_amount = Uint128::new(100_000);

    let msg = InstantiateMsg {
        owner: Addr::unchecked(owner),
    };
    let info = mock_info("deployer", &[]);
    let _res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

    // withdraw
    let mut custom_querier = MockQuerier::default();
    custom_querier.update_wasm(|query| match query {
        WasmQuery::Smart { contract_addr, msg } => {
            assert_eq!(contract_addr, "token_addr");
            match from_json(msg).unwrap() {
                Cw20QueryMsg::Balance { address } => {
                    assert_eq!(address, "lock_contract_addr");

                    let balance = Uint128::new(100_000);

                    SystemResult::Ok(ContractResult::Ok(
                        to_json_binary(&Cw20BalanceResponse { balance }).unwrap(),
                    ))
                }
                _ => panic!("DO NOT ENTER HERE"),
            }
        }
        _ => panic!("DO NOT ENTER HERE"),
    });
    deps.querier = custom_querier;
    let info = mock_info("someone", &[]); // anyone can call withdraw
    let mut env = mock_env();
    env.block.time = env.block.time.plus_days(365);
    env.contract.address = Addr::unchecked(lock_contract_addr);
    let msg = ExecuteMsg::Withdraw {
        token_address: Addr::unchecked(token_addr),
    };
    let res = execute(deps.as_mut(), env, info, msg).unwrap();
    assert_eq!(
        res.attributes
            .iter()
            .find(|attr| attr.key == "amount")
            .unwrap()
            .value,
        locked_amount.to_string()
    )
}
