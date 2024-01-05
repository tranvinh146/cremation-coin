use std::vec;

use cosmwasm_std::{
    from_binary,
    testing::{mock_dependencies, mock_env, mock_info},
    Addr, Binary, CosmosMsg, Decimal, Uint128, WasmMsg,
};
use cw20::{Cw20Coin, TokenInfoResponse};
use cw20_base::{msg::InstantiateMsg as Cw20InstantiateMsg, ContractError};

use crate::{
    contract::{execute, internal, SWAP_COLLECTED_TAX_THRESHOLD},
    instantiate,
    msg::{
        CollectTaxAddressResponse, DexConfigsResponse, ExecuteMsg, InstantiateMsg, OwnerResponse,
        QueryMsg, TaxFreeAddressResponse, TaxInfoResponse,
    },
    query,
    state::{DexConfigs, FractionFormat, TaxInfo},
};

use self::helpers::get_dex_configs;

mod helpers {
    use cosmwasm_std::{testing::*, *};

    use super::*;

    pub fn initialize(
        deps: &mut OwnedDeps<MockStorage, MockApi, MockQuerier, Empty>,
        owner: &str,
        tax_info: TaxInfo,
        cw20_instantiate_msg: Cw20InstantiateMsg,
    ) -> Result<Response, ContractError> {
        let msg = InstantiateMsg {
            owner: owner.to_string(),
            tax_info,
            swap_tax_to_token: "cremat_token".to_string(),
            cw20_instantiate_msg,
        };
        instantiate(deps.as_mut(), mock_env(), mock_info("creator", &[]), msg)
    }

    pub fn mock_cw20_instantiate_msg(initial_balances: Vec<Cw20Coin>) -> Cw20InstantiateMsg {
        Cw20InstantiateMsg {
            name: "Lenny Coin".to_string(),
            symbol: "LENNY".to_string(),
            decimals: 6,
            initial_balances,
            mint: None,
            marketing: None,
        }
    }

    pub fn setup_contract(
        deps: &mut OwnedDeps<MockStorage, MockApi, MockQuerier, Empty>,
        init_msg: InstantiateMsg,
    ) -> Result<(), ContractError> {
        instantiate(
            deps.as_mut(),
            mock_env(),
            mock_info("creator", &[]),
            init_msg,
        )
        .unwrap();

        let dex_configs = get_dex_configs();
        let terraswap_router = dex_configs.terraswap_router.to_string();
        let terraswap_pairs = dex_configs
            .terraswap_pairs
            .into_iter()
            .map(|addr| addr.to_string())
            .collect::<Vec<String>>();
        let terraport_router = dex_configs.terraport_router.to_string();
        let terraport_pairs = dex_configs
            .terraport_pairs
            .into_iter()
            .map(|addr| addr.to_string())
            .collect::<Vec<String>>();

        execute(
            deps.as_mut(),
            mock_env(),
            mock_info("creator", &[]),
            ExecuteMsg::SetDexConfigs {
                terraswap_router,
                terraswap_pairs,
                terraport_router,
                terraport_pairs,
            },
        )
        .unwrap();

        Ok(())
    }

    pub fn get_dex_configs() -> DexConfigs {
        DexConfigs {
            terraswap_router: Addr::unchecked("terraswap_router"),
            terraswap_pairs: vec![Addr::unchecked("terraswap_pair")],
            terraport_router: Addr::unchecked("terraport_router"),
            terraport_pairs: vec![
                Addr::unchecked("terraport_pair"),
                Addr::unchecked("terraport_pair2"),
            ],
        }
    }

    pub fn query_balance(
        deps: &OwnedDeps<MockStorage, MockApi, MockQuerier, Empty>,
        address: &Addr,
    ) -> Uint128 {
        let balance_query = query(
            deps.as_ref(),
            mock_env(),
            QueryMsg::Balance {
                address: address.to_string(),
            },
        )
        .unwrap();
        let balance_res: cw20::BalanceResponse = from_binary(&balance_query).unwrap();
        balance_res.balance
    }
}

#[test]
fn proper_initialization() {
    let mut deps = mock_dependencies();
    let env = mock_env();
    let creator = "creator";
    let owner = "owner";
    let tax_rate = FractionFormat {
        numerator: Uint128::new(8),
        denominator: Uint128::new(100),
    };
    let total_supply = Uint128::new(1_000_000_000_000);
    let msg = InstantiateMsg {
        owner: owner.to_string(),
        swap_tax_to_token: "cremat_token".to_string(),
        tax_info: TaxInfo {
            buy_tax: Some(tax_rate.clone()),
            sell_tax: Some(tax_rate.clone()),
            transfer_tax: None,
        },
        cw20_instantiate_msg: Cw20InstantiateMsg {
            name: "Lenny Coin".to_string(),
            symbol: "CREMAT".to_string(),
            decimals: 6,
            initial_balances: vec![Cw20Coin {
                address: owner.to_string(),
                amount: total_supply,
            }],
            mint: None,
            marketing: None,
        },
    };
    instantiate(deps.as_mut(), env, mock_info(creator, &[]), msg).unwrap();

    let dex_configs = get_dex_configs();
    let terraswap_router = dex_configs.terraswap_router.to_string();
    let terraswap_pairs = dex_configs
        .terraswap_pairs
        .into_iter()
        .map(|addr| addr.to_string())
        .collect::<Vec<String>>();
    let terraport_router = dex_configs.terraport_router.to_string();
    let terraport_pairs = dex_configs
        .terraport_pairs
        .into_iter()
        .map(|addr| addr.to_string())
        .collect::<Vec<String>>();
    execute(
        deps.as_mut(),
        mock_env(),
        mock_info("creator", &[]),
        ExecuteMsg::SetDexConfigs {
            terraswap_router,
            terraswap_pairs,
            terraport_router,
            terraport_pairs,
        },
    )
    .unwrap();

    // check token_info
    let token_info_query = query(deps.as_ref(), mock_env(), QueryMsg::TokenInfo {}).unwrap();
    let token_info_res: TokenInfoResponse = from_binary(&token_info_query).unwrap();
    assert_eq!(token_info_res.name, "Lenny Coin");
    assert_eq!(token_info_res.symbol, "CREMAT");
    assert_eq!(token_info_res.decimals, 6);
    assert_eq!(token_info_res.total_supply, total_supply);

    // check owner balance
    let balance_query = query(
        deps.as_ref(),
        mock_env(),
        QueryMsg::Balance {
            address: owner.into(),
        },
    )
    .unwrap();
    let balance_res: cw20::BalanceResponse = from_binary(&balance_query).unwrap();
    assert_eq!(balance_res.balance, total_supply);

    // check owner
    let owner_query = query(deps.as_ref(), mock_env(), QueryMsg::Owner {}).unwrap();
    let owner_res: OwnerResponse = from_binary(&owner_query).unwrap();
    assert_eq!(owner_res.owner, owner);

    // check config
    let dex_configs = get_dex_configs();
    let config_query = query(deps.as_ref(), mock_env(), QueryMsg::DexConfigs {}).unwrap();
    let config_res: DexConfigsResponse = from_binary(&config_query).unwrap();
    assert_eq!(config_res.terraswap_router, dex_configs.terraswap_router);
    assert_eq!(config_res.terraswap_pairs, dex_configs.terraswap_pairs);
    assert_eq!(config_res.terraport_router, dex_configs.terraport_router);
    assert_eq!(config_res.terraport_pairs, dex_configs.terraport_pairs);

    // check tax_info
    let tax_info_query = query(deps.as_ref(), mock_env(), QueryMsg::TaxInfo {}).unwrap();
    let tax_info_res: TaxInfoResponse = from_binary(&tax_info_query).unwrap();
    assert_eq!(tax_info_res.buy_tax, Decimal::percent(8));
    assert_eq!(tax_info_res.sell_tax, Decimal::percent(8));
    assert_eq!(tax_info_res.transfer_tax, Decimal::zero());

    // check collect tax address
    let collect_tax_addr_query =
        query(deps.as_ref(), mock_env(), QueryMsg::CollectTaxAddress {}).unwrap();
    let collect_tax_addr_res: CollectTaxAddressResponse =
        from_binary(&collect_tax_addr_query).unwrap();
    assert_eq!(collect_tax_addr_res.collect_tax_address, owner);

    // check tax free address
    let tax_free_addr_query = query(
        deps.as_ref(),
        mock_env(),
        QueryMsg::TaxFreeAddress {
            address: owner.to_string(),
        },
    )
    .unwrap();
    let tax_free_addr_res: TaxFreeAddressResponse = from_binary(&tax_free_addr_query).unwrap();
    assert_eq!(tax_free_addr_res.tax_free, true);

    let tax_free_addr_query = query(
        deps.as_ref(),
        mock_env(),
        QueryMsg::TaxFreeAddress {
            address: "no_tax_free_addr".to_string(),
        },
    )
    .unwrap();
    let tax_free_addr_res: TaxFreeAddressResponse = from_binary(&tax_free_addr_query).unwrap();
    assert_eq!(tax_free_addr_res.tax_free, false);
}

// ======= test extended executes =======
// - update_owner
// - update_tax_info
// - update_collect_tax_address
// - set_tax_free_address

#[test]
fn update_owner() {
    let mut deps = mock_dependencies();
    let owner = "owner";
    let tax_info = TaxInfo {
        buy_tax: None,
        sell_tax: None,
        transfer_tax: None,
    };
    let cw20_instantiate_msg = helpers::mock_cw20_instantiate_msg(vec![]);
    helpers::initialize(&mut deps, owner, tax_info, cw20_instantiate_msg).unwrap();

    // fail to change owner with non-owner
    let non_owner = "non_owner".to_string();
    let info = mock_info(&non_owner, &[]);
    let msg = ExecuteMsg::UpdateOwner {
        new_owner: non_owner,
    };
    let err = execute(deps.as_mut(), mock_env(), info, msg).unwrap_err();
    assert_eq!(err, ContractError::Unauthorized {});

    // change owner
    let new_owner = "new_owner".to_string();
    let info = mock_info(&owner, &[]);
    let msg: ExecuteMsg = ExecuteMsg::UpdateOwner {
        new_owner: new_owner.to_string(),
    };
    let _res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();

    // check new owner
    let owner_query = query(deps.as_ref(), mock_env(), QueryMsg::Owner {}).unwrap();
    let owner_res: OwnerResponse = from_binary(&owner_query).unwrap();
    assert_eq!(owner_res.owner, new_owner);
}

#[test]
fn update_tax_info() {
    let mut deps = mock_dependencies();
    let owner = "owner";
    let tax_rate = FractionFormat {
        numerator: Uint128::new(8),
        denominator: Uint128::new(100),
    };
    let tax_info = TaxInfo {
        buy_tax: Some(tax_rate.clone()),
        sell_tax: Some(tax_rate.clone()),
        transfer_tax: None,
    };
    let cw20_instantiate_msg = helpers::mock_cw20_instantiate_msg(vec![]);
    helpers::initialize(&mut deps, owner, tax_info, cw20_instantiate_msg).unwrap();

    // fail to change tax info with non-owner
    let new_tax_rate = FractionFormat {
        numerator: Uint128::new(1),
        denominator: Uint128::new(100),
    };
    let info = mock_info("non_owner", &[]);
    let msg = ExecuteMsg::UpdateTaxInfo {
        buy_tax: Some(new_tax_rate.clone()),
        sell_tax: Some(new_tax_rate.clone()),
        transfer_tax: None,
    };
    let err = execute(deps.as_mut(), mock_env(), info, msg).unwrap_err();
    assert_eq!(err, ContractError::Unauthorized {});

    // change tax info
    let new_buy_tax_rate = tax_rate.clone(); // existed tax rate - 8%
    let new_sell_tax_rate = FractionFormat {
        numerator: Uint128::new(10),
        denominator: Uint128::new(100),
    }; // new tax rate - 10%
    let new_transfer_tax_rate = FractionFormat {
        numerator: Uint128::new(1),
        denominator: Uint128::new(100),
    }; // new tax rate - 1%
    let info = mock_info(owner, &[]);
    let msg = ExecuteMsg::UpdateTaxInfo {
        buy_tax: Some(new_buy_tax_rate.clone()),
        sell_tax: Some(new_sell_tax_rate.clone()),
        transfer_tax: Some(new_transfer_tax_rate.clone()),
    };
    let _res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();

    // check tax_info
    let tax_info_query = query(deps.as_ref(), mock_env(), QueryMsg::TaxInfo {}).unwrap();
    let tax_info_res: TaxInfoResponse = from_binary(&tax_info_query).unwrap();
    assert_eq!(tax_info_res.buy_tax, Decimal::percent(8));
    assert_eq!(tax_info_res.sell_tax, Decimal::percent(10));
    assert_eq!(tax_info_res.transfer_tax, Decimal::percent(1));
}

#[test]
fn update_collect_tax_address() {
    let mut deps = mock_dependencies();
    let owner = "owner";
    let tax_info = TaxInfo {
        buy_tax: None,
        sell_tax: None,
        transfer_tax: None,
    };
    let cw20_instantiate_msg = helpers::mock_cw20_instantiate_msg(vec![]);
    helpers::initialize(&mut deps, owner, tax_info, cw20_instantiate_msg).unwrap();

    // fail to change collect tax address with non-owner
    let non_owner = "non_owner".to_string();
    let info = mock_info(&non_owner, &[]);
    let msg = ExecuteMsg::UpdateCollectTaxAddress {
        new_collect_tax_addr: non_owner,
    };
    let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap_err();
    assert_eq!(res, ContractError::Unauthorized {});

    // change collect tax address
    let new_collect_tax_addr = "new_collect_tax_addr".to_string();
    let info = mock_info(owner, &[]);
    let msg = ExecuteMsg::UpdateCollectTaxAddress {
        new_collect_tax_addr: new_collect_tax_addr.clone(),
    };
    let _res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();

    // check collect tax address
    let collect_tax_addr_query =
        query(deps.as_ref(), mock_env(), QueryMsg::CollectTaxAddress {}).unwrap();
    let response: CollectTaxAddressResponse = from_binary(&collect_tax_addr_query).unwrap();
    assert_eq!(response.collect_tax_address, new_collect_tax_addr);
}

#[test]
fn set_tax_free_address() {
    let mut deps = mock_dependencies();
    let owner = "owner";
    let tax_info = TaxInfo {
        buy_tax: None,
        sell_tax: None,
        transfer_tax: None,
    };
    let cw20_instantiate_msg = helpers::mock_cw20_instantiate_msg(vec![]);
    helpers::initialize(&mut deps, owner, tax_info, cw20_instantiate_msg).unwrap();

    // fail to set tax free address with non-owner
    let non_owner = "non_owner".to_string();
    let info = mock_info(&non_owner, &[]);
    let msg = ExecuteMsg::SetTaxFreeAddress {
        address: non_owner,
        tax_free: true,
    };
    let err = execute(deps.as_mut(), mock_env(), info, msg).unwrap_err();
    assert_eq!(err, ContractError::Unauthorized {});

    // set tax free address
    let tax_free_addr = "tax_free_addr";
    let info = mock_info(owner, &[]);
    let msg = ExecuteMsg::SetTaxFreeAddress {
        address: tax_free_addr.to_string(),
        tax_free: true,
    };
    let _res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();

    // check set tax free address - true
    let tax_free_addr_query = query(
        deps.as_ref(),
        mock_env(),
        QueryMsg::TaxFreeAddress {
            address: tax_free_addr.to_string(),
        },
    )
    .unwrap();
    let response: TaxFreeAddressResponse = from_binary(&tax_free_addr_query).unwrap();
    assert_eq!(response.tax_free, true);

    // unset tax free address
    let info = mock_info(owner, &[]);
    let msg = ExecuteMsg::SetTaxFreeAddress {
        address: tax_free_addr.to_string(),
        tax_free: false,
    };
    let _res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();

    // check unset tax free address - false
    let tax_free_addr_query = query(
        deps.as_ref(),
        mock_env(),
        QueryMsg::TaxFreeAddress {
            address: tax_free_addr.to_string(),
        },
    )
    .unwrap();
    let response: TaxFreeAddressResponse = from_binary(&tax_free_addr_query).unwrap();
    assert_eq!(response.tax_free, false);
}

// ======= test tax =======
// - send
// - send from
// - transfer
// - transfer from

// test collect tax when execute send cw20
#[test]
fn collect_tax_when_execute_send() {
    let mut deps = mock_dependencies();
    let owner = Addr::unchecked("owner-tax-free");
    let buy_tax = FractionFormat {
        numerator: Uint128::new(7),
        denominator: Uint128::new(100),
    };
    let sell_tax = FractionFormat {
        numerator: Uint128::new(15),
        denominator: Uint128::new(100),
    };

    let user_addresses = vec![
        Addr::unchecked("user1"),
        Addr::unchecked("user2"),
        owner.clone(),
    ];
    let contract_addresses = vec![Addr::unchecked("contract1"), Addr::unchecked("contract2")];
    let dex_configs = helpers::get_dex_configs();
    let pair_addresses = vec![dex_configs.terraswap_pairs, dex_configs.terraport_pairs]
        .into_iter()
        .flatten()
        .collect::<Vec<Addr>>();
    let router_addresses = vec![dex_configs.terraswap_router, dex_configs.terraport_router];
    let addresses = vec![
        user_addresses.clone(),
        contract_addresses.clone(),
        pair_addresses.clone(),
        router_addresses.clone(),
    ]
    .into_iter()
    .flatten()
    .collect::<Vec<Addr>>();

    let init_amount = Uint128::new(1000);
    let amount_list = addresses
        .iter()
        .map(|addr| Cw20Coin {
            address: addr.to_string(),
            amount: init_amount.clone(),
        })
        .collect::<Vec<Cw20Coin>>();
    let init_msg = InstantiateMsg {
        owner: owner.to_string(),
        swap_tax_to_token: "cremat_token".to_string(),
        tax_info: TaxInfo {
            buy_tax: Some(buy_tax.clone()),
            sell_tax: Some(sell_tax.clone()),
            transfer_tax: None,
        },
        cw20_instantiate_msg: helpers::mock_cw20_instantiate_msg(amount_list),
    };
    helpers::setup_contract(&mut deps, init_msg).unwrap();

    let dex_configs = helpers::get_dex_configs();
    let collect_tax_wallet = owner.clone();
    for sender in addresses.iter() {
        for recipient in addresses.iter() {
            // `send` fn` only works with smart contract
            if user_addresses.contains(recipient) {
                continue;
            }

            let info = mock_info(&sender.to_string(), &[]);
            let send_amount = Uint128::new(100);

            let sender_balance_before = helpers::query_balance(&deps, &sender);
            let recipient_balance_before = helpers::query_balance(&deps, &recipient);
            let collect_tax_wallet_balance_before =
                helpers::query_balance(&deps, &collect_tax_wallet);

            let msg = ExecuteMsg::Send {
                contract: recipient.to_string(),
                amount: send_amount,
                msg: Binary::default(),
            };
            let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();

            let is_sell = internal::is_sell_operation(&dex_configs, &sender, &recipient);
            let is_buy = internal::is_buy_operation(&dex_configs, &sender, &recipient);
            let is_tax_free = sender == &collect_tax_wallet || recipient == &collect_tax_wallet;
            if is_sell && !is_tax_free {
                // selling by user
                assert!(user_addresses.contains(&sender) || contract_addresses.contains(&sender));

                let expected_tax_amount =
                    send_amount.multiply_ratio(sell_tax.numerator, sell_tax.denominator);
                let tax_opt = res
                    .attributes
                    .iter()
                    .find(|attr| attr.key == "cw20_tax_amount");
                assert!(tax_opt.is_some());
                assert_eq!(tax_opt.unwrap().value, expected_tax_amount.to_string());
            } else if is_buy && !is_tax_free {
                // only pair can sell for user
                assert!(pair_addresses.contains(&sender));

                let expected_tax_amount =
                    send_amount.multiply_ratio(buy_tax.numerator, buy_tax.denominator);
                let tax_opt = res
                    .attributes
                    .iter()
                    .find(|attr| attr.key == "cw20_tax_amount");
                assert!(tax_opt.is_some());
                assert_eq!(tax_opt.unwrap().value, expected_tax_amount.to_string());
            } else {
                let tax_opt = res
                    .attributes
                    .iter()
                    .find(|attr| attr.key == "cw20_tax_amount");
                assert!(tax_opt.is_none());
            }

            let sender_balance_after = helpers::query_balance(&deps, &sender);
            let recipient_balance_after = helpers::query_balance(&deps, &recipient);
            let collect_tax_wallet_balance_after =
                helpers::query_balance(&deps, &collect_tax_wallet);

            if sender == &collect_tax_wallet {
                assert_eq!(
                    sender_balance_before - sender_balance_after,
                    recipient_balance_after - recipient_balance_before
                );
            } else {
                assert_eq!(
                    sender_balance_before - sender_balance_after,
                    (recipient_balance_after - recipient_balance_before)
                        + (collect_tax_wallet_balance_after - collect_tax_wallet_balance_before)
                );
            }
        }
    }
}

// test collect tax when execute send_from cw20
// owner approves spender
// spender send cw20 from owner -> recipient
#[test]
fn collect_tax_when_execute_send_from() {
    let mut deps = mock_dependencies();
    let owner = Addr::unchecked("owner-tax-free");
    let buy_tax = FractionFormat {
        numerator: Uint128::new(7),
        denominator: Uint128::new(100),
    };
    let sell_tax = FractionFormat {
        numerator: Uint128::new(15),
        denominator: Uint128::new(100),
    };

    let user_addresses = vec![
        Addr::unchecked("user1"),
        Addr::unchecked("user2"),
        owner.clone(),
    ];
    let contract_addresses = vec![Addr::unchecked("contract1"), Addr::unchecked("contract2")];
    let pair_addresses = vec![
        Addr::unchecked("terraswap_pair"),
        Addr::unchecked("terraport_pair"),
    ];
    let router_addresses = vec![
        Addr::unchecked("terraswap_router"),
        Addr::unchecked("terraport_router"),
    ];
    let addresses = vec![
        user_addresses.clone(),
        contract_addresses.clone(),
        pair_addresses.clone(),
        router_addresses.clone(),
    ]
    .into_iter()
    .flatten()
    .collect::<Vec<Addr>>();

    let init_amount = Uint128::new(1000);
    let amount_list = addresses
        .iter()
        .map(|addr| Cw20Coin {
            address: addr.to_string(),
            amount: init_amount.clone(),
        })
        .collect::<Vec<Cw20Coin>>();

    let init_msg = InstantiateMsg {
        owner: owner.to_string(),
        swap_tax_to_token: "cremat_token".to_string(),
        tax_info: TaxInfo {
            buy_tax: Some(buy_tax.clone()),
            sell_tax: Some(sell_tax.clone()),
            transfer_tax: None,
        },
        cw20_instantiate_msg: helpers::mock_cw20_instantiate_msg(amount_list),
    };
    helpers::setup_contract(&mut deps, init_msg).unwrap();

    let dex_configs = helpers::get_dex_configs();
    let collect_tax_wallet = owner.clone();
    for sender in addresses.iter() {
        for recipient in addresses.iter() {
            // `sendFrom` fn only works with smart contract
            // sender can NOT call `increase_allowance` to himself
            if user_addresses.contains(&recipient) || &sender == &recipient {
                continue;
            }

            let info = mock_info(&sender.to_string(), &[]);
            let allow_amount = Uint128::new(100);
            let msg = ExecuteMsg::IncreaseAllowance {
                spender: recipient.to_string(),
                amount: allow_amount,
                expires: None,
            };
            execute(deps.as_mut(), mock_env(), info, msg).unwrap();

            let sender_balance_before = helpers::query_balance(&deps, &sender);
            let recipient_balance_before = helpers::query_balance(&deps, &recipient);
            let collect_tax_wallet_balance_before =
                helpers::query_balance(&deps, &collect_tax_wallet);

            let info = mock_info(&recipient.to_string(), &[]);
            let msg = ExecuteMsg::SendFrom {
                owner: sender.to_string(),
                contract: recipient.to_string(),
                amount: allow_amount,
                msg: Binary::default(),
            };
            let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();

            let is_sell = internal::is_sell_operation(&dex_configs, &sender, &recipient);
            let is_buy = internal::is_buy_operation(&dex_configs, &sender, &recipient);
            let is_tax_free = sender == &collect_tax_wallet || recipient == &collect_tax_wallet;
            if is_sell && !is_tax_free {
                // selling by user
                assert!(user_addresses.contains(&sender) || contract_addresses.contains(&sender));

                let expected_tax_amount =
                    allow_amount.multiply_ratio(sell_tax.numerator, sell_tax.denominator);
                let tax_opt = res
                    .attributes
                    .iter()
                    .find(|attr| attr.key == "cw20_tax_amount");
                assert!(tax_opt.is_some());
                assert_eq!(tax_opt.unwrap().value, expected_tax_amount.to_string());
            } else if is_buy && !is_tax_free {
                // only pair can sell for user/contract
                assert!(pair_addresses.contains(&sender));

                let expected_tax_amount =
                    allow_amount.multiply_ratio(buy_tax.numerator, buy_tax.denominator);

                let tax_opt = res
                    .attributes
                    .iter()
                    .find(|attr| attr.key == "cw20_tax_amount");
                assert!(tax_opt.is_some());
                assert_eq!(tax_opt.unwrap().value, expected_tax_amount.to_string());
            } else {
                let tax_opt = res
                    .attributes
                    .iter()
                    .find(|attr| attr.key == "cw20_tax_amount");
                assert!(tax_opt.is_none());
            }

            let sender_balance_after = helpers::query_balance(&deps, &sender);
            let recipient_balance_after = helpers::query_balance(&deps, &recipient);
            let collect_tax_wallet_balance_after =
                helpers::query_balance(&deps, &collect_tax_wallet);

            if sender == &collect_tax_wallet {
                assert_eq!(
                    sender_balance_before - sender_balance_after,
                    recipient_balance_after - recipient_balance_before
                );
            } else {
                assert_eq!(
                    sender_balance_before - sender_balance_after,
                    (recipient_balance_after - recipient_balance_before)
                        + (collect_tax_wallet_balance_after - collect_tax_wallet_balance_before)
                );
            }
        }
    }
}

// test collect tax when execute transfer cw20
#[test]
fn collect_tax_when_execute_transfer() {
    let mut deps = mock_dependencies();
    let owner = Addr::unchecked("owner-tax-free");
    let transfer_tax = FractionFormat {
        numerator: Uint128::new(7),
        denominator: Uint128::new(100),
    };

    let normal_addresses = vec![
        Addr::unchecked("user"),
        Addr::unchecked("contract"),
        owner.clone(),
    ];
    let pair_addresses = vec![
        Addr::unchecked("terraswap_pair"),
        Addr::unchecked("terraport_pair"),
    ];
    let router_addresses = vec![
        Addr::unchecked("terraswap_router"),
        Addr::unchecked("terraport_router"),
    ];
    let addresses = vec![
        normal_addresses.clone(),
        pair_addresses.clone(),
        router_addresses.clone(),
    ]
    .into_iter()
    .flatten()
    .collect::<Vec<Addr>>();

    let init_amount = Uint128::new(1000);
    let amount_list = addresses
        .iter()
        .map(|addr| Cw20Coin {
            address: addr.to_string(),
            amount: init_amount.clone(),
        })
        .collect::<Vec<Cw20Coin>>();

    let init_msg = InstantiateMsg {
        owner: owner.to_string(),
        swap_tax_to_token: "cremat_token".to_string(),
        tax_info: TaxInfo {
            buy_tax: None,
            sell_tax: None,
            transfer_tax: Some(transfer_tax.clone()),
        },
        cw20_instantiate_msg: helpers::mock_cw20_instantiate_msg(amount_list),
    };
    helpers::setup_contract(&mut deps, init_msg).unwrap();

    let collect_tax_wallet = owner.clone();
    for sender in addresses.iter() {
        for recipient in addresses.iter() {
            let sender_balance_before = helpers::query_balance(&deps, &sender);
            let recipient_balance_before = helpers::query_balance(&deps, &recipient);
            let collect_tax_wallet_balance_before =
                helpers::query_balance(&deps, &collect_tax_wallet);

            let info = mock_info(&sender.to_string(), &[]);
            let transfer_amount = Uint128::new(100);
            let msg = ExecuteMsg::Transfer {
                recipient: recipient.to_string(),
                amount: transfer_amount,
            };
            let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();

            if sender == &collect_tax_wallet || recipient == &collect_tax_wallet {
                let tax_opt = res
                    .attributes
                    .iter()
                    .find(|attr| attr.key == "cw20_tax_amount");
                assert!(tax_opt.is_none());
            } else {
                let expected_tax_amount = transfer_amount
                    .multiply_ratio(transfer_tax.numerator, transfer_tax.denominator);
                let tax_opt = res
                    .attributes
                    .iter()
                    .find(|attr| attr.key == "cw20_tax_amount");
                assert!(tax_opt.is_some());
                assert_eq!(tax_opt.unwrap().value, expected_tax_amount.to_string());
            }

            let sender_balance_after = helpers::query_balance(&deps, &sender);
            let recipient_balance_after = helpers::query_balance(&deps, &recipient);
            let collect_tax_wallet_balance_after =
                helpers::query_balance(&deps, &collect_tax_wallet);

            if sender == &collect_tax_wallet || recipient == &collect_tax_wallet {
                assert_eq!(
                    sender_balance_before - sender_balance_after,
                    recipient_balance_after - recipient_balance_before
                );
            } else if sender == recipient {
                assert_eq!(
                    sender_balance_before - sender_balance_after,
                    recipient_balance_before - recipient_balance_after
                );
                assert_eq!(
                    sender_balance_before - sender_balance_after,
                    collect_tax_wallet_balance_after - collect_tax_wallet_balance_before
                );
            } else {
                assert_eq!(
                    sender_balance_before - sender_balance_after,
                    (recipient_balance_after - recipient_balance_before)
                        + (collect_tax_wallet_balance_after - collect_tax_wallet_balance_before)
                );
            }
        }
    }
}

// test collect tax when execute transfer_from cw20
// owner approves spender
// spender transfer cw20 from owner -> recipient
#[test]
fn collect_transfer_tax_when_execute_transfer_from() {
    let mut deps = mock_dependencies();
    let owner = Addr::unchecked("owner-tax-free");
    let transfer_tax = FractionFormat {
        numerator: Uint128::new(7),
        denominator: Uint128::new(100),
    };

    let normal_addresses = vec![
        Addr::unchecked("user"),
        Addr::unchecked("contract"),
        owner.clone(),
    ];
    let pair_addresses = vec![
        Addr::unchecked("terraswap_pair"),
        Addr::unchecked("terraport_pair"),
    ];
    let router_addresses = vec![
        Addr::unchecked("terraswap_router"),
        Addr::unchecked("terraport_router"),
    ];
    let addresses = vec![
        normal_addresses.clone(),
        pair_addresses.clone(),
        router_addresses.clone(),
    ]
    .into_iter()
    .flatten()
    .collect::<Vec<Addr>>();

    let init_amount = Uint128::new(1000);
    let amount_list = addresses
        .iter()
        .map(|addr| Cw20Coin {
            address: addr.to_string(),
            amount: init_amount.clone(),
        })
        .collect::<Vec<Cw20Coin>>();

    let init_msg = InstantiateMsg {
        owner: owner.to_string(),
        swap_tax_to_token: "cremat_token".to_string(),
        tax_info: TaxInfo {
            buy_tax: None,
            sell_tax: None,
            transfer_tax: Some(transfer_tax.clone()),
        },
        cw20_instantiate_msg: helpers::mock_cw20_instantiate_msg(amount_list),
    };
    helpers::setup_contract(&mut deps, init_msg).unwrap();

    let collect_tax_wallet = owner.clone();
    for sender in addresses.iter() {
        for recipient in addresses.iter() {
            // sender can NOT call `increase_allowance` to himself
            if &sender == &recipient {
                continue;
            }

            let info = mock_info(&sender.to_string(), &[]);
            let allow_amount = Uint128::new(100);
            let msg = ExecuteMsg::IncreaseAllowance {
                spender: recipient.to_string(),
                amount: allow_amount,
                expires: None,
            };
            execute(deps.as_mut(), mock_env(), info, msg).unwrap();

            let sender_balance_before = helpers::query_balance(&deps, &sender);
            let recipient_balance_before = helpers::query_balance(&deps, &recipient);
            let collect_tax_wallet_balance_before =
                helpers::query_balance(&deps, &collect_tax_wallet);

            let info = mock_info(&sender.to_string(), &[]);
            let transfer_amount = Uint128::new(100);
            let msg = ExecuteMsg::Transfer {
                recipient: recipient.to_string(),
                amount: transfer_amount,
            };
            let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();

            if sender == &collect_tax_wallet || recipient == &collect_tax_wallet {
                let tax_opt = res
                    .attributes
                    .iter()
                    .find(|attr| attr.key == "cw20_tax_amount");
                assert!(tax_opt.is_none());
            } else {
                let expected_tax_amount = transfer_amount
                    .multiply_ratio(transfer_tax.numerator, transfer_tax.denominator);
                let tax_opt = res
                    .attributes
                    .iter()
                    .find(|attr| attr.key == "cw20_tax_amount");
                assert!(tax_opt.is_some());
                assert_eq!(tax_opt.unwrap().value, expected_tax_amount.to_string());
            }

            let sender_balance_after = helpers::query_balance(&deps, &sender);
            let recipient_balance_after = helpers::query_balance(&deps, &recipient);
            let collect_tax_wallet_balance_after =
                helpers::query_balance(&deps, &collect_tax_wallet);

            if sender == &collect_tax_wallet || recipient == &collect_tax_wallet {
                assert_eq!(
                    sender_balance_before - sender_balance_after,
                    recipient_balance_after - recipient_balance_before
                );
            } else {
                assert_eq!(
                    sender_balance_before - sender_balance_after,
                    (recipient_balance_after - recipient_balance_before)
                        + (collect_tax_wallet_balance_after - collect_tax_wallet_balance_before)
                );
            }
        }
    }
}

#[test]
fn trigger_auto_swap_collected_tax() {
    let mut deps = mock_dependencies();
    let owner = Addr::unchecked("owner-tax-free");
    let seller = Addr::unchecked("seller");
    let sell_tax = FractionFormat {
        numerator: Uint128::new(40),
        denominator: Uint128::new(100),
    };

    let init_msg = InstantiateMsg {
        owner: owner.to_string(),
        swap_tax_to_token: "cremat_token".to_string(),
        tax_info: TaxInfo {
            buy_tax: None,
            sell_tax: Some(sell_tax.clone()),
            transfer_tax: None,
        },
        cw20_instantiate_msg: helpers::mock_cw20_instantiate_msg(vec![Cw20Coin {
            address: seller.to_string(),
            amount: Uint128::MAX,
        }]),
    };
    helpers::setup_contract(&mut deps, init_msg).unwrap();

    // send from buyer to terraswap router
    let dex_configs = helpers::get_dex_configs();
    let pair_addresses = vec![
        dex_configs.terraswap_pairs.clone(),
        dex_configs.terraport_pairs.clone(),
    ]
    .into_iter()
    .flatten()
    .collect::<Vec<Addr>>();
    let router_addresses = vec![
        dex_configs.terraswap_router.clone(),
        dex_configs.terraport_router.clone(),
    ];
    let dex_addresses = vec![router_addresses, pair_addresses]
        .into_iter()
        .flatten()
        .collect::<Vec<Addr>>();

    for addr in dex_addresses.iter() {
        let sell_amount = SWAP_COLLECTED_TAX_THRESHOLD * sell_tax.denominator / sell_tax.numerator
            + Uint128::one();
        let msg = ExecuteMsg::Send {
            contract: addr.to_string(),
            amount: sell_amount,
            msg: Binary::default(),
        };
        let mut env = mock_env();
        env.contract.address = Addr::unchecked("lenny_token");
        let info = mock_info(&seller.to_string(), &[]);
        let res = execute(deps.as_mut(), env, info, msg).unwrap();
        let auto_swap_msg = res.messages.iter().find(|sub_msg| match sub_msg.msg {
            CosmosMsg::Wasm(WasmMsg::Execute {
                ref contract_addr, ..
            }) => contract_addr == "lenny_token",
            _ => false,
        });
        let collected_tax_opt = res
            .attributes
            .iter()
            .find(|attr| attr.key == "action" && attr.value == "collected_tax_swap");

        if addr == dex_configs.terraport_router || dex_configs.terraport_pairs.contains(addr) {
            assert!(auto_swap_msg.is_none());
            assert!(collected_tax_opt.is_none());
        } else {
            assert!(auto_swap_msg.is_some());
            assert!(collected_tax_opt.is_some());
        }
    }
}

// test add new pairs after migrate
#[test]
fn add_new_pairs_after_migrate() {
    let mut deps = mock_dependencies();
    let owner = Addr::unchecked("owner-tax-free");

    let init_msg = InstantiateMsg {
        owner: owner.to_string(),
        swap_tax_to_token: "cremat_token".to_string(),
        tax_info: TaxInfo {
            buy_tax: None,
            sell_tax: None,
            transfer_tax: None,
        },
        cw20_instantiate_msg: helpers::mock_cw20_instantiate_msg(vec![]),
    };
    helpers::setup_contract(&mut deps, init_msg).unwrap();

    let new_pair_addresses = vec![
        "terraswap_pair_new1".to_string(),
        "terraswap_pair_new2".to_string(),
    ];
    let msg = ExecuteMsg::AddNewPairs {
        dex: crate::msg::Dex::Terraswap,
        pair_addresses: new_pair_addresses.clone(),
    };
    let env = mock_env();
    let info = mock_info(&owner.to_string(), &[]);
    execute(deps.as_mut(), env, info, msg).unwrap();

    let config_query = query(deps.as_ref(), mock_env(), QueryMsg::DexConfigs {}).unwrap();
    let config_res: crate::msg::DexConfigsResponse = from_binary(&config_query).unwrap();

    let old_dex_config = get_dex_configs();
    let old_terraswap_pairs = old_dex_config.terraswap_pairs;
    let old_terraport_pairs = old_dex_config.terraport_pairs;
    for i in 0..old_terraport_pairs.len() {
        assert_eq!(config_res.terraport_pairs[i], old_terraport_pairs[i]);
    }
    for i in 0..old_terraswap_pairs.len() {
        assert_eq!(config_res.terraswap_pairs[i], old_terraswap_pairs[i]);
    }

    for pair in new_pair_addresses {
        assert!(config_res.terraswap_pairs.contains(&Addr::unchecked(pair)));
    }
}
