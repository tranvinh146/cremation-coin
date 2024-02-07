use cosmwasm_std::{
    from_json,
    testing::{mock_dependencies, mock_env, mock_info},
    to_json_binary, Addr, Coin, CosmosMsg, Reply, SubMsgResponse, SubMsgResult, SystemResult,
    Uint128, WasmMsg, WasmQuery,
};
use cremation_token::{
    msg::{AssetInfo, CollectTaxAddressResponse, QueryMsg as ExtendedCw20QueryMsg},
    state::FractionFormat,
};
use cw20::{BalanceResponse as Cw20BalanceResponse, Cw20ExecuteMsg, Cw20ReceiveMsg};

use crate::{
    contract::{execute, instantiate, query, reply, SWAP_REPLY_ID},
    error::ContractError,
    msg::{
        Cw20HookMsg, ExecuteMsg, InstantiateMsg, OwnerResponse, QueryMsg, SwapRouterResponse,
        TokenBuyTaxResponse,
    },
};

#[test]
fn init_properly() {
    let mut deps = mock_dependencies();
    let env = mock_env();
    let info = mock_info("deployer", &[]);
    let owner = String::from("owner");
    let swap_router = String::from("router");
    let init_msg = InstantiateMsg {
        owner: owner.clone(),
        swap_router: swap_router.clone(),
    };
    instantiate(deps.as_mut(), env.clone(), info, init_msg).unwrap();

    let owner_query = query(deps.as_ref(), env.clone(), QueryMsg::Owner {}).unwrap();
    let owner_res: OwnerResponse = from_json(&owner_query).unwrap();
    assert_eq!(owner_res.owner, owner.to_string());

    let swap_router_query = query(deps.as_ref(), env.clone(), QueryMsg::SwapRouter {}).unwrap();
    let swap_router_res: SwapRouterResponse = from_json(&swap_router_query).unwrap();
    assert_eq!(swap_router_res.router, swap_router);
}

#[test]
fn update_owner_properly() {
    let mut deps = mock_dependencies();
    let env = mock_env();
    let info = mock_info("deployer", &[]);
    let owner = String::from("owner");
    let swap_router = String::from("router");
    let new_owner = String::from("new_owner");
    let init_msg = InstantiateMsg {
        owner: owner.clone(),
        swap_router: swap_router.clone(),
    };
    instantiate(deps.as_mut(), env.clone(), info, init_msg).unwrap();

    let update_owner_info = mock_info(owner.as_str(), &[]);
    let update_owner_msg = ExecuteMsg::UpdateOwner {
        new_owner: new_owner.clone(),
    };
    execute(
        deps.as_mut(),
        env.clone(),
        update_owner_info,
        update_owner_msg,
    )
    .unwrap();

    let owner_query = query(deps.as_ref(), env.clone(), QueryMsg::Owner {}).unwrap();
    let owner_res: OwnerResponse = from_json(&owner_query).unwrap();
    assert_eq!(owner_res.owner, new_owner);
}

#[test]
fn update_owner_without_authorized() {
    let mut deps = mock_dependencies();
    let env = mock_env();
    let info = mock_info("deployer", &[]);
    let owner = String::from("owner");
    let swap_router = String::from("router");
    let new_owner = String::from("new_owner");
    let init_msg = InstantiateMsg {
        owner: owner.clone(),
        swap_router: swap_router.clone(),
    };
    instantiate(deps.as_mut(), env.clone(), info, init_msg).unwrap();

    let update_owner_info = mock_info("random", &[]);
    let update_owner_msg = ExecuteMsg::UpdateOwner {
        new_owner: new_owner.clone(),
    };
    let res = execute(
        deps.as_mut(),
        env.clone(),
        update_owner_info,
        update_owner_msg,
    );
    assert_eq!(res.unwrap_err(), ContractError::Unauthorized {});
}

#[test]
fn update_swap_router_properly() {
    let mut deps = mock_dependencies();
    let env = mock_env();
    let info = mock_info("deployer", &[]);
    let owner = String::from("owner");
    let swap_router = String::from("router");
    let new_swap_router = String::from("new_router");
    let init_msg = InstantiateMsg {
        owner: owner.clone(),
        swap_router: swap_router.clone(),
    };
    instantiate(deps.as_mut(), env.clone(), info, init_msg).unwrap();

    let update_swap_router_info = mock_info(owner.as_str(), &[]);
    let update_swap_router_msg = ExecuteMsg::UpdateSwapRouter {
        router: new_swap_router.clone(),
    };
    execute(
        deps.as_mut(),
        env.clone(),
        update_swap_router_info,
        update_swap_router_msg,
    )
    .unwrap();

    let swap_router_query = query(deps.as_ref(), env.clone(), QueryMsg::SwapRouter {}).unwrap();
    let swap_router_res: SwapRouterResponse = from_json(&swap_router_query).unwrap();
    assert_eq!(swap_router_res.router, new_swap_router);
}

#[test]
fn update_swap_router_without_authorized() {
    let mut deps = mock_dependencies();
    let env = mock_env();
    let info = mock_info("deployer", &[]);
    let owner = String::from("owner");
    let swap_router = String::from("router");
    let new_swap_router = String::from("new_router");
    let init_msg = InstantiateMsg {
        owner: owner.clone(),
        swap_router: swap_router.clone(),
    };
    instantiate(deps.as_mut(), env.clone(), info, init_msg).unwrap();

    let update_swap_router_info = mock_info("random", &[]);
    let update_swap_router_msg = ExecuteMsg::UpdateSwapRouter {
        router: new_swap_router.clone(),
    };
    let res = execute(
        deps.as_mut(),
        env.clone(),
        update_swap_router_info,
        update_swap_router_msg,
    );
    assert_eq!(res.unwrap_err(), ContractError::Unauthorized {});
}

#[test]
fn set_token_buy_tax_properly() {
    let mut deps = mock_dependencies();
    let env = mock_env();
    let info = mock_info("deployer", &[]);
    let owner = String::from("owner");
    let swap_router = String::from("router");
    let token_address = String::from("token_address");
    let buy_tax = FractionFormat {
        numerator: Uint128::from(1u64),
        denominator: Uint128::from(100u64),
    };
    let init_msg = InstantiateMsg {
        owner: owner.clone(),
        swap_router: swap_router.clone(),
    };
    instantiate(deps.as_mut(), env.clone(), info, init_msg).unwrap();

    let set_token_buy_tax_info = mock_info(owner.as_str(), &[]);
    let set_token_buy_tax_msg = ExecuteMsg::SetTokenBuyTax {
        token_address: token_address.clone(),
        buy_tax: buy_tax.clone(),
    };
    execute(
        deps.as_mut(),
        env.clone(),
        set_token_buy_tax_info,
        set_token_buy_tax_msg,
    )
    .unwrap();

    let token_buy_tax_query = query(
        deps.as_ref(),
        env.clone(),
        QueryMsg::TokenTaxInfo {
            token_address: token_address.clone(),
        },
    )
    .unwrap();
    let token_buy_tax_res: TokenBuyTaxResponse = from_json(&token_buy_tax_query).unwrap();
    assert_eq!(token_buy_tax_res.buy_tax, buy_tax);
}

#[test]
fn set_token_buy_tax_without_authorized() {
    let mut deps = mock_dependencies();
    let env = mock_env();
    let info = mock_info("deployer", &[]);
    let owner = String::from("owner");
    let swap_router = String::from("router");
    let token_address = String::from("token_address");
    let buy_tax = FractionFormat {
        numerator: Uint128::from(1u64),
        denominator: Uint128::from(100u64),
    };
    let init_msg = InstantiateMsg {
        owner: owner.clone(),
        swap_router: swap_router.clone(),
    };
    instantiate(deps.as_mut(), env.clone(), info, init_msg).unwrap();

    let set_token_buy_tax_info = mock_info("random", &[]);
    let set_token_buy_tax_msg = ExecuteMsg::SetTokenBuyTax {
        token_address: token_address.clone(),
        buy_tax: buy_tax.clone(),
    };
    let res = execute(
        deps.as_mut(),
        env.clone(),
        set_token_buy_tax_info,
        set_token_buy_tax_msg,
    );
    assert_eq!(res.unwrap_err(), ContractError::Unauthorized {});
}

#[test]
fn swap_native_properly() {
    let mut deps = mock_dependencies();
    let env = mock_env();
    let info = mock_info("deployer", &[]);
    let owner = "owner";
    let swap_router = "router";
    let token_address = "token_address";
    let buyer = "buyer";
    let collect_tax_address = "collect_tax_address";

    let init_msg = InstantiateMsg {
        owner: owner.to_string(),
        swap_router: swap_router.to_string(),
    };
    instantiate(deps.as_mut(), env.clone(), info, init_msg).unwrap();

    let buy_tax = FractionFormat {
        numerator: Uint128::from(50u64),
        denominator: Uint128::from(100u64),
    };
    let set_token_buy_tax_info = mock_info(owner, &[]);
    let set_token_buy_tax_msg = ExecuteMsg::SetTokenBuyTax {
        token_address: token_address.to_string(),
        buy_tax: buy_tax.clone(),
    };
    execute(
        deps.as_mut(),
        env.clone(),
        set_token_buy_tax_info,
        set_token_buy_tax_msg,
    )
    .unwrap();

    let swap_info = mock_info(
        buyer,
        &[Coin {
            denom: "uluna".to_string(),
            amount: Uint128::from(100u128),
        }],
    );
    let swap_msg = ExecuteMsg::Swap {
        ask_asset: AssetInfo::Token {
            contract_addr: token_address.to_string(),
        },
        swap_paths: vec![],
    };
    let res = execute(deps.as_mut(), env.clone(), swap_info, swap_msg).unwrap();
    assert_eq!(res.messages.len(), 1);
    let submsg = res.messages[0].clone();
    assert_eq!(submsg.id, SWAP_REPLY_ID);

    // expect submsg call to router contract
    match submsg.msg {
        CosmosMsg::Wasm(wasm_msg) => match wasm_msg {
            WasmMsg::Execute {
                contract_addr,
                funds,
                ..
            } => {
                assert_eq!(contract_addr, swap_router);
                assert_eq!(funds.len(), 1);
                assert_eq!(
                    funds[0],
                    Coin {
                        denom: "uluna".to_string(),
                        amount: Uint128::from(100u128),
                    }
                );
            }
            _ => panic!("Unexpected wasm message"),
        },
        _ => panic!("Unexpected sub message"),
    }

    let swapped_amount = Uint128::from(1_000_000u128);

    // Mock query
    let query_env = env.clone();
    let query_swapped_amount = swapped_amount.clone();
    deps.querier.update_wasm(move |query| match query {
        WasmQuery::Smart { contract_addr, msg } => match from_json(&msg).unwrap() {
            ExtendedCw20QueryMsg::Balance { address } => {
                assert_eq!(contract_addr, token_address);
                assert_eq!(address, query_env.contract.address);
                let res = Cw20BalanceResponse {
                    balance: query_swapped_amount,
                };
                SystemResult::Ok((to_json_binary(&res)).into())
            }
            ExtendedCw20QueryMsg::CollectTaxAddress {} => {
                assert_eq!(contract_addr, "token_address");
                let res = CollectTaxAddressResponse {
                    collect_tax_address: Addr::unchecked(collect_tax_address),
                };
                SystemResult::Ok((to_json_binary(&res)).into())
            }
            _ => panic!("DO NOT ENTER HERE"),
        },
        _ => panic!("DO NOT ENTER HERE"),
    });

    // call reply
    let res = reply(
        deps.as_mut(),
        env.clone(),
        Reply {
            id: submsg.id,
            result: SubMsgResult::Ok(SubMsgResponse {
                events: vec![],
                data: None,
            }),
        },
    )
    .unwrap();

    // expect transfer to collect tax address
    let tax_amount = swapped_amount * buy_tax.numerator / buy_tax.denominator;
    match res.messages[0].msg.clone() {
        CosmosMsg::Wasm(wasm_msg) => match wasm_msg {
            WasmMsg::Execute {
                contract_addr,
                msg,
                funds,
            } => {
                assert_eq!(contract_addr, token_address);
                assert_eq!(funds.len(), 0);
                let msg: Cw20ExecuteMsg = from_json(&msg).unwrap();
                match msg {
                    Cw20ExecuteMsg::Transfer { recipient, amount } => {
                        assert_eq!(recipient, buyer);
                        assert_eq!(amount, swapped_amount - tax_amount);
                    }
                    _ => panic!("Unexpected cw20 message"),
                }
            }
            _ => panic!("Unexpected wasm message"),
        },
        _ => panic!("Unexpected sub message"),
    }
    match res.messages[1].msg.clone() {
        CosmosMsg::Wasm(wasm_msg) => match wasm_msg {
            WasmMsg::Execute {
                contract_addr,
                msg,
                funds,
            } => {
                assert_eq!(contract_addr, token_address);
                assert_eq!(funds.len(), 0);
                let msg: Cw20ExecuteMsg = from_json(&msg).unwrap();
                match msg {
                    Cw20ExecuteMsg::Transfer { recipient, amount } => {
                        assert_eq!(recipient, collect_tax_address);
                        assert_eq!(amount, tax_amount);
                    }
                    _ => panic!("Unexpected cw20 message"),
                }
            }
            _ => panic!("Unexpected wasm message"),
        },
        _ => panic!("Unexpected sub message"),
    }

    let attrs = res
        .attributes
        .iter()
        .find(|attr| attr.key == "cw20_tax_amount")
        .unwrap();
    assert_eq!(attrs.value, tax_amount.to_string());
}

#[test]
fn swap_cw20_properly() {
    let mut deps = mock_dependencies();
    let env = mock_env();
    let info = mock_info("deployer", &[]);
    let owner = "owner";
    let swap_router = "router";
    let buyer = "buyer";
    let offer_token_address = "offer_token_address";
    let ask_token_address = "ask_token_address";
    let collect_tax_address = "collect_tax_address";

    let init_msg = InstantiateMsg {
        owner: owner.to_string(),
        swap_router: swap_router.to_string(),
    };
    instantiate(deps.as_mut(), env.clone(), info, init_msg).unwrap();

    let buy_tax = FractionFormat {
        numerator: Uint128::from(50u64),
        denominator: Uint128::from(100u64),
    };
    let set_token_buy_tax_info = mock_info(owner, &[]);
    let set_token_buy_tax_msg = ExecuteMsg::SetTokenBuyTax {
        token_address: ask_token_address.to_string(),
        buy_tax: buy_tax.clone(),
    };
    execute(
        deps.as_mut(),
        env.clone(),
        set_token_buy_tax_info,
        set_token_buy_tax_msg,
    )
    .unwrap();

    let swap_msg = ExecuteMsg::Receive(Cw20ReceiveMsg {
        amount: Uint128::from(100u128),
        msg: to_json_binary(&Cw20HookMsg::Swap {
            ask_asset: AssetInfo::Token {
                contract_addr: ask_token_address.to_string(),
            },
            swap_paths: vec![],
        })
        .unwrap(),
        sender: buyer.to_string(),
    });
    let swap_info = mock_info(offer_token_address, &[]);
    let res = execute(deps.as_mut(), env.clone(), swap_info, swap_msg).unwrap();
    assert_eq!(res.messages.len(), 1);
    let submsg = res.messages[0].clone();
    assert_eq!(submsg.id, SWAP_REPLY_ID);

    // expect submsg call to router contract
    match submsg.msg {
        CosmosMsg::Wasm(wasm_msg) => match wasm_msg {
            WasmMsg::Execute {
                contract_addr,
                funds,
                ..
            } => {
                assert_eq!(contract_addr, offer_token_address);
                assert_eq!(funds.len(), 0);
            }
            _ => panic!("Unexpected wasm message"),
        },
        _ => panic!("Unexpected sub message"),
    }

    let swapped_amount = Uint128::from(1_000_000u128);

    // Mock query
    let query_env = env.clone();
    let query_swapped_amount = swapped_amount.clone();
    deps.querier.update_wasm(move |query| match query {
        WasmQuery::Smart { contract_addr, msg } => match from_json(&msg).unwrap() {
            ExtendedCw20QueryMsg::Balance { address } => {
                assert_eq!(contract_addr, ask_token_address);
                assert_eq!(address, query_env.contract.address);
                let res = Cw20BalanceResponse {
                    balance: query_swapped_amount,
                };
                SystemResult::Ok((to_json_binary(&res)).into())
            }
            ExtendedCw20QueryMsg::CollectTaxAddress {} => {
                assert_eq!(contract_addr, ask_token_address);
                let res = CollectTaxAddressResponse {
                    collect_tax_address: Addr::unchecked(collect_tax_address),
                };
                SystemResult::Ok((to_json_binary(&res)).into())
            }
            _ => panic!("DO NOT ENTER HERE"),
        },
        _ => panic!("DO NOT ENTER HERE"),
    });

    // call reply
    let res = reply(
        deps.as_mut(),
        env.clone(),
        Reply {
            id: submsg.id,
            result: SubMsgResult::Ok(SubMsgResponse {
                events: vec![],
                data: None,
            }),
        },
    )
    .unwrap();

    // expect transfer to collect tax address
    let tax_amount = swapped_amount * buy_tax.numerator / buy_tax.denominator;
    match res.messages[0].msg.clone() {
        CosmosMsg::Wasm(wasm_msg) => match wasm_msg {
            WasmMsg::Execute {
                contract_addr,
                msg,
                funds,
            } => {
                assert_eq!(contract_addr, ask_token_address);
                assert_eq!(funds.len(), 0);
                let msg: Cw20ExecuteMsg = from_json(&msg).unwrap();
                match msg {
                    Cw20ExecuteMsg::Transfer { recipient, amount } => {
                        assert_eq!(recipient, buyer);
                        assert_eq!(amount, swapped_amount - tax_amount);
                    }
                    _ => panic!("Unexpected cw20 message"),
                }
            }
            _ => panic!("Unexpected wasm message"),
        },
        _ => panic!("Unexpected sub message"),
    }
    match res.messages[1].msg.clone() {
        CosmosMsg::Wasm(wasm_msg) => match wasm_msg {
            WasmMsg::Execute {
                contract_addr,
                msg,
                funds,
            } => {
                assert_eq!(contract_addr, ask_token_address);
                assert_eq!(funds.len(), 0);
                let msg: Cw20ExecuteMsg = from_json(&msg).unwrap();
                match msg {
                    Cw20ExecuteMsg::Transfer { recipient, amount } => {
                        assert_eq!(recipient, collect_tax_address);
                        assert_eq!(amount, tax_amount);
                    }
                    _ => panic!("Unexpected cw20 message"),
                }
            }
            _ => panic!("Unexpected wasm message"),
        },
        _ => panic!("Unexpected sub message"),
    }

    let attrs = res
        .attributes
        .iter()
        .find(|attr| attr.key == "cw20_tax_amount")
        .unwrap();
    assert_eq!(attrs.value, tax_amount.to_string());
}
