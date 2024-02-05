use cosmwasm_std::{
    coin, from_json,
    testing::{mock_dependencies, mock_env, mock_info},
    Addr, Coin, Decimal, Uint128,
};
use cw20::{BalanceResponse as Cw20BalanceResponse, Cw20Coin, Cw20QueryMsg};
use cw20_base::contract::{
    execute as cw20_execute, instantiate as cw20_instantiate, query as cw20_query,
};
use cw_multi_test::{App, ContractWrapper, Executor};
use std::time::SystemTime;

use crate::{contract::LUNC_TAX, error::ContractError, execute, instantiate, msg::*, query};

mod helpers {
    use cosmwasm_std::{
        testing::{MockApi, MockQuerier, MockStorage},
        Empty, OwnedDeps,
    };

    use super::*;

    pub fn psuedo_rand(range: u64) -> usize {
        let now = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap();
        (now.as_secs() % range) as usize
    }

    pub fn mock_reward_list(len: u64) -> Vec<RewardInfo> {
        let mut reward_list = Vec::new();
        for i in 0..len {
            let token = format!("token{}", i);
            let reward_ratio = Decimal::percent((i + 1) * 10);
            let reward_info = RewardInfo {
                token: token.to_string(),
                reward_ratio,
            };
            reward_list.push(reward_info);
        }

        reward_list
    }

    pub fn execute_add_multi_rewards_to_whitelist(
        deps: &mut OwnedDeps<MockStorage, MockApi, MockQuerier, Empty>,
        owner: &str,
        rewards_info: Vec<RewardInfo>,
    ) {
        let env = mock_env();
        let info = mock_info(owner, &[]);

        for reward_info in rewards_info {
            let msg = ExecuteMsg::AddToRewardWhitelist { reward_info };
            execute(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();
        }
    }

    pub struct SetupContractsRes {
        pub burning_addr: Addr,
        pub reward_list: Vec<RewardInfo>,
    }

    pub fn setup_multi_test_contracts(
        app: &mut App,
        owner: &str,
        init_cw20_reward: u128,
        development_config: Option<DevelopmentConfig>,
    ) -> SetupContractsRes {
        let burning_code = ContractWrapper::new(execute, instantiate, query);
        let burning_code_id = app.store_code(Box::new(burning_code));
        let development_config = development_config.unwrap_or(DevelopmentConfig {
            fee_ratio: Decimal::percent(2),
            beneficiary: "beneficiary".to_string(),
        });
        let burning_addr = app
            .instantiate_contract(
                burning_code_id,
                Addr::unchecked("deployer"),
                &InstantiateMsg {
                    owner: owner.to_string(),
                    development_config,
                },
                &[],
                "BurningContract",
                None,
            )
            .unwrap();

        // deploy reward tokens
        let mut token_addrs = vec![];

        for i in 0..10 {
            let token_code = ContractWrapper::new(cw20_execute, cw20_instantiate, cw20_query);
            let token_code_id = app.store_code(Box::new(token_code));
            let token = format!("token{}", i);
            let token_addr = app
                .instantiate_contract(
                    token_code_id,
                    Addr::unchecked("deployer"),
                    &cw20_base::msg::InstantiateMsg {
                        name: token.clone(),
                        symbol: "TOKEN".to_string(),
                        decimals: 6,
                        initial_balances: vec![Cw20Coin {
                            address: burning_addr.to_string(),
                            amount: Uint128::new(init_cw20_reward),
                        }],
                        mint: None,
                        marketing: None,
                    },
                    &[],
                    token,
                    None,
                )
                .unwrap();

            token_addrs.push(token_addr);
        }

        // add reward tokens to whitelist
        let mut reward_list = vec![];
        for i in 0..10 {
            let token_addr = token_addrs[i].clone();
            let reward_ratio = Decimal::percent((i + 1) as u64 * 10);
            let reward_info = RewardInfo {
                token: token_addr.to_string(),
                reward_ratio,
            };
            reward_list.push(reward_info.clone());

            app.execute_contract(
                Addr::unchecked(owner),
                Addr::unchecked(burning_addr.clone()),
                &ExecuteMsg::AddToRewardWhitelist { reward_info },
                &[],
            )
            .unwrap();
        }

        SetupContractsRes {
            burning_addr,
            reward_list,
        }
    }
}

// 1. init properly
#[test]
fn init_properly() {
    let mut deps = mock_dependencies();
    let env = mock_env();
    let info = mock_info("deployer", &[]);
    let owner = "owner";
    let development_config = DevelopmentConfig {
        fee_ratio: Decimal::percent(2),
        beneficiary: "beneficiary".to_string(),
    };

    let init_msg = InstantiateMsg {
        owner: owner.to_string(),
        development_config: development_config.clone(),
    };
    instantiate(deps.as_mut(), env, info, init_msg).unwrap();

    let owner_query = query(deps.as_ref(), mock_env(), QueryMsg::Owner {}).unwrap();
    let owner_res: OwnerResponse = from_json(&owner_query).unwrap();
    assert_eq!(owner_res.owner, owner.to_string());

    let reward_whitelist_query =
        query(deps.as_ref(), mock_env(), QueryMsg::RewardWhitelist {}).unwrap();
    let reward_whitelist_res: RewardWhitelistResponse = from_json(&reward_whitelist_query).unwrap();
    assert_eq!(reward_whitelist_res.reward_whitelist.len(), 0);

    let burned_amount_query = query(deps.as_ref(), mock_env(), QueryMsg::BurnedAmount {}).unwrap();
    let burned_amount_res: BurnedAmountResponse = from_json(&burned_amount_query).unwrap();
    assert_eq!(burned_amount_res.burned_amount, Uint128::zero());

    let development_config_query =
        query(deps.as_ref(), mock_env(), QueryMsg::DevelopmentConfig {}).unwrap();
    let development_config_res: DevelopmentConfigResponse =
        from_json(&development_config_query).unwrap();
    assert_eq!(development_config_res.0, development_config);
}

// ============= add_to_reward_whitelist =============
// 2a. add reward to whitelist properly
#[test]
fn add_to_reward_whitelist_properly() {
    let mut deps = mock_dependencies();
    let env = mock_env();
    let info = mock_info("deployer", &[]);
    let owner = "owner";

    let init_msg = InstantiateMsg {
        owner: owner.to_string(),
        development_config: DevelopmentConfig {
            fee_ratio: Decimal::percent(2),
            beneficiary: "beneficiary".to_string(),
        },
    };
    instantiate(deps.as_mut(), env, info, init_msg).unwrap();

    let env = mock_env();
    let info = mock_info(owner, &[]);
    let token = "token";
    let reward_ratio = Decimal::percent(10);
    let reward_info = RewardInfo {
        token: token.to_string(),
        reward_ratio,
    };
    let msg = ExecuteMsg::AddToRewardWhitelist { reward_info };
    let res = execute(deps.as_mut(), env, info, msg).unwrap();

    // check attrs exists
    assert_eq!(3, res.attributes.len());
    assert_eq!(res.attributes[0].key, "action");
    assert_eq!(res.attributes[0].value, "add_to_reward_whitelist");
    assert_eq!(res.attributes[1].key, "token");
    assert_eq!(res.attributes[1].value, token);
    assert_eq!(res.attributes[2].key, "reward_ratio");
    assert_eq!(res.attributes[2].value, reward_ratio.to_string());

    let reward_whitelist_query =
        query(deps.as_ref(), mock_env(), QueryMsg::RewardWhitelist {}).unwrap();
    let reward_whitelist_res: RewardWhitelistResponse = from_json(&reward_whitelist_query).unwrap();
    assert_eq!(reward_whitelist_res.reward_whitelist.len(), 1);
    assert_eq!(
        reward_whitelist_res.reward_whitelist[0].token,
        token.to_string()
    );
    assert_eq!(
        reward_whitelist_res.reward_whitelist[0].reward_ratio,
        reward_ratio
    );

    // execute add multi rewards to whitelist
    let reward_list_len = 10;
    let rewards_info = helpers::mock_reward_list(reward_list_len);
    helpers::execute_add_multi_rewards_to_whitelist(&mut deps, owner, rewards_info);
}

// 2b. add reward to whitelist not authorized
#[test]
fn fail_to_add_to_reward_whitelist_not_authorized() {
    let mut deps = mock_dependencies();
    let env = mock_env();
    let info = mock_info("deployer", &[]);
    let owner = "owner";

    let init_msg = InstantiateMsg {
        owner: owner.to_string(),
        development_config: DevelopmentConfig {
            fee_ratio: Decimal::percent(2),
            beneficiary: "beneficiary".to_string(),
        },
    };
    instantiate(deps.as_mut(), env, info, init_msg).unwrap();

    let env = mock_env();
    let info = mock_info("anyone", &[]);
    let token = "token";
    let reward_ratio = Decimal::percent(10);
    let reward_info = RewardInfo {
        token: token.to_string(),
        reward_ratio,
    };
    let msg = ExecuteMsg::AddToRewardWhitelist { reward_info };
    let res = execute(deps.as_mut(), env, info, msg);
    assert!(res.is_err());

    let err = res.unwrap_err();
    assert_eq!(err, ContractError::Unauthorized {});
}

// 2c. add reward to whitelist with zero ratio
#[test]
fn fail_to_add_to_reward_whitelist_with_zero_ratio() {
    let mut deps = mock_dependencies();
    let env = mock_env();
    let info = mock_info("deployer", &[]);
    let owner = "owner";

    let init_msg = InstantiateMsg {
        owner: owner.to_string(),
        development_config: DevelopmentConfig {
            fee_ratio: Decimal::percent(2),
            beneficiary: "beneficiary".to_string(),
        },
    };
    instantiate(deps.as_mut(), env, info, init_msg).unwrap();

    let env = mock_env();
    let info = mock_info(owner, &[]);
    let token = "token";
    let reward_ratio = Decimal::zero();
    let reward_info = RewardInfo {
        token: token.to_string(),
        reward_ratio,
    };
    let msg = ExecuteMsg::AddToRewardWhitelist { reward_info };
    let res = execute(deps.as_mut(), env, info, msg);
    assert!(res.is_err());

    let err = res.unwrap_err();
    assert_eq!(err, ContractError::ZeroRatio {});
}

// 2d. add reward already in whitelist
#[test]
fn fail_to_add_to_reward_whitelist_already_in_whitelist() {
    let mut deps = mock_dependencies();
    let env = mock_env();
    let info = mock_info("deployer", &[]);
    let owner = "owner";

    let init_msg = InstantiateMsg {
        owner: owner.to_string(),
        development_config: DevelopmentConfig {
            fee_ratio: Decimal::percent(2),
            beneficiary: "beneficiary".to_string(),
        },
    };
    instantiate(deps.as_mut(), env, info, init_msg).unwrap();

    let reward_list_len = 10;
    let reward_list = helpers::mock_reward_list(reward_list_len);
    helpers::execute_add_multi_rewards_to_whitelist(&mut deps, owner, reward_list.clone());

    let env = mock_env();
    let rand_id = helpers::psuedo_rand(reward_list_len);
    let info = mock_info(owner, &[]);
    let reward_info = reward_list[rand_id].to_owned();
    let msg = ExecuteMsg::AddToRewardWhitelist { reward_info };
    let res = execute(deps.as_mut(), env, info, msg);
    assert!(res.is_err());

    let err = res.unwrap_err();
    assert_eq!(err, ContractError::AlreadyExists {});
}

// ============= remove_from_reward_whitelist =============
// 3a. remove reward from whitelist properly
#[test]
fn remove_from_reward_whitelist_properly() {
    let mut deps = mock_dependencies();
    let env = mock_env();
    let info = mock_info("deployer", &[]);
    let owner = "owner";

    let init_msg = InstantiateMsg {
        owner: owner.to_string(),
        development_config: DevelopmentConfig {
            fee_ratio: Decimal::percent(2),
            beneficiary: "beneficiary".to_string(),
        },
    };
    instantiate(deps.as_mut(), env, info, init_msg).unwrap();

    let env = mock_env();
    let info = mock_info(owner, &[]);
    let reward_list_len = 10;
    let reward_list = helpers::mock_reward_list(reward_list_len);
    helpers::execute_add_multi_rewards_to_whitelist(&mut deps, owner, reward_list.clone());

    let rand_id = helpers::psuedo_rand(reward_list_len);
    let reward_info = reward_list[rand_id].to_owned();
    let token = reward_info.token.to_string();
    let msg = ExecuteMsg::RemoveFromRewardWhitelist {
        token: token.clone(),
    };
    let res = execute(deps.as_mut(), env, info, msg).unwrap();
    assert_eq!(2, res.attributes.len());
    assert_eq!(res.attributes[0].key, "action");
    assert_eq!(res.attributes[0].value, "remove_from_reward_whitelist");
    assert_eq!(res.attributes[1].key, "token");
    assert_eq!(res.attributes[1].value, token);

    let reward_whitelist_query =
        query(deps.as_ref(), mock_env(), QueryMsg::RewardWhitelist {}).unwrap();
    let reward_whitelist_res: RewardWhitelistResponse = from_json(&reward_whitelist_query).unwrap();
    assert_eq!(
        reward_whitelist_res.reward_whitelist.len(),
        (reward_list_len - 1) as usize
    );

    // check reward not in whitelist
    for reward_info in reward_whitelist_res.reward_whitelist {
        assert_ne!(reward_info.token, token);
    }
}

// 3b. remove reward from whitelist not authorized
#[test]
fn fail_to_remove_from_reward_whitelist_not_authorized() {
    let mut deps = mock_dependencies();
    let env = mock_env();
    let info = mock_info("deployer", &[]);
    let owner = "owner";

    let init_msg = InstantiateMsg {
        owner: owner.to_string(),
        development_config: DevelopmentConfig {
            fee_ratio: Decimal::percent(2),
            beneficiary: "beneficiary".to_string(),
        },
    };
    instantiate(deps.as_mut(), env.clone(), info.clone(), init_msg).unwrap();

    let info = mock_info(owner, &[]);
    let token = "token";
    let reward_ratio = Decimal::percent(10);
    let reward_info = RewardInfo {
        token: token.to_string(),
        reward_ratio,
    };
    let msg = ExecuteMsg::AddToRewardWhitelist { reward_info };
    let res = execute(deps.as_mut(), env.clone(), info, msg).unwrap();
    assert_eq!(3, res.attributes.len());

    let info = mock_info("anyone", &[]);
    let msg = ExecuteMsg::RemoveFromRewardWhitelist {
        token: token.to_string(),
    };
    let res = execute(deps.as_mut(), env, info, msg);
    assert!(res.is_err());

    let err = res.unwrap_err();
    assert_eq!(err, ContractError::Unauthorized {});
}

// 3c. remove reward from whitelist not in whitelist
#[test]
fn fail_to_remove_from_reward_whitelist_not_in_whitelist() {
    let mut deps = mock_dependencies();
    let env = mock_env();
    let info = mock_info("deployer", &[]);
    let owner = "owner";

    let init_msg = InstantiateMsg {
        owner: owner.to_string(),
        development_config: DevelopmentConfig {
            fee_ratio: Decimal::percent(2),
            beneficiary: "beneficiary".to_string(),
        },
    };
    instantiate(deps.as_mut(), env.clone(), info.clone(), init_msg).unwrap();

    let reward_list = helpers::mock_reward_list(5);
    helpers::execute_add_multi_rewards_to_whitelist(&mut deps, owner, reward_list);

    let info = mock_info(owner, &[]);
    let token = "token-not-in-whitelist";
    let msg = ExecuteMsg::RemoveFromRewardWhitelist {
        token: token.to_string(),
    };
    let res = execute(deps.as_mut(), env, info, msg);
    assert!(res.is_err());

    let err = res.unwrap_err();
    assert_eq!(err, ContractError::NotInWhitelist {});
}

// ============= update_reward_info =============
// 4a. update reward info properly
#[test]
fn update_reward_info_properly() {
    let mut deps = mock_dependencies();
    let env = mock_env();
    let info = mock_info("deployer", &[]);
    let owner = "owner";

    let init_msg = InstantiateMsg {
        owner: owner.to_string(),
        development_config: DevelopmentConfig {
            fee_ratio: Decimal::percent(2),
            beneficiary: "beneficiary".to_string(),
        },
    };
    instantiate(deps.as_mut(), env, info, init_msg).unwrap();

    let reward_list_len = 10;
    let reward_list = helpers::mock_reward_list(10);
    helpers::execute_add_multi_rewards_to_whitelist(&mut deps, owner, reward_list.clone());

    let env = mock_env();
    let info = mock_info(owner, &[]);
    let rand_id = helpers::psuedo_rand(reward_list_len);
    let mut reward_info = reward_list[rand_id].to_owned();
    reward_info.reward_ratio += Decimal::percent(helpers::psuedo_rand(100) as u64);
    let msg = ExecuteMsg::UpdateRewardInfo {
        reward_info: reward_info.clone(),
    };
    let res = execute(deps.as_mut(), env, info, msg).unwrap();
    assert_eq!(3, res.attributes.len());

    let reward_whitelist_query =
        query(deps.as_ref(), mock_env(), QueryMsg::RewardWhitelist {}).unwrap();
    let reward_whitelist_res: RewardWhitelistResponse = from_json(&reward_whitelist_query).unwrap();
    assert_eq!(
        reward_whitelist_res.reward_whitelist.len(),
        reward_list_len as usize
    );
    assert_eq!(
        reward_whitelist_res.reward_whitelist[rand_id].token,
        reward_info.token.to_string()
    );
    assert_eq!(
        reward_whitelist_res.reward_whitelist[rand_id].reward_ratio,
        reward_info.reward_ratio
    );
}

// 4b. update reward info not authorized
#[test]
fn fail_to_update_reward_info_not_authorized() {
    let mut deps = mock_dependencies();
    let env = mock_env();
    let info = mock_info("deployer", &[]);
    let owner = "owner";

    let init_msg = InstantiateMsg {
        owner: owner.to_string(),
        development_config: DevelopmentConfig {
            fee_ratio: Decimal::percent(2),
            beneficiary: "beneficiary".to_string(),
        },
    };
    instantiate(deps.as_mut(), env.clone(), info.clone(), init_msg).unwrap();

    let reward_list_len = 10;
    let reward_list = helpers::mock_reward_list(10);
    helpers::execute_add_multi_rewards_to_whitelist(&mut deps, owner, reward_list.clone());

    let env = mock_env();
    let info = mock_info("anyone", &[]);
    let rand_id = helpers::psuedo_rand(reward_list_len);
    let reward_info = reward_list[rand_id].to_owned();
    let msg = ExecuteMsg::UpdateRewardInfo {
        reward_info: reward_info.clone(),
    };
    let res = execute(deps.as_mut(), env, info, msg);
    assert!(res.is_err());

    let err = res.unwrap_err();
    assert_eq!(err, ContractError::Unauthorized {});
}

// 4c. update reward info with zero ratio
#[test]
fn fail_to_update_reward_info_with_zero_ratio() {
    let mut deps = mock_dependencies();
    let env = mock_env();
    let info = mock_info("deployer", &[]);
    let owner = "owner";
    let init_msg = InstantiateMsg {
        owner: owner.to_string(),
        development_config: DevelopmentConfig {
            fee_ratio: Decimal::percent(2),
            beneficiary: "beneficiary".to_string(),
        },
    };
    instantiate(deps.as_mut(), env.clone(), info.clone(), init_msg).unwrap();

    let reward_list_len = 10;
    let reward_list = helpers::mock_reward_list(10);
    helpers::execute_add_multi_rewards_to_whitelist(&mut deps, owner, reward_list.clone());

    let env = mock_env();
    let info = mock_info(owner, &[]);
    let rand_id = helpers::psuedo_rand(reward_list_len);
    let mut reward_info = reward_list[rand_id].to_owned();
    reward_info.reward_ratio = Decimal::zero();
    let msg = ExecuteMsg::UpdateRewardInfo {
        reward_info: reward_info.clone(),
    };
    let res = execute(deps.as_mut(), env, info, msg);
    assert!(res.is_err());

    let err = res.unwrap_err();
    assert_eq!(err, ContractError::ZeroRatio {});
}

// 4d. update reward info not in whitelist
#[test]
fn fail_to_update_reward_info_not_in_whitelist() {
    let mut deps = mock_dependencies();
    let env = mock_env();
    let info = mock_info("deployer", &[]);
    let owner = "owner";
    let init_msg = InstantiateMsg {
        owner: owner.to_string(),
        development_config: DevelopmentConfig {
            fee_ratio: Decimal::percent(2),
            beneficiary: "beneficiary".to_string(),
        },
    };
    instantiate(deps.as_mut(), env.clone(), info.clone(), init_msg).unwrap();

    let reward_list = helpers::mock_reward_list(10);
    helpers::execute_add_multi_rewards_to_whitelist(&mut deps, owner, reward_list.clone());

    let env = mock_env();
    let info = mock_info(owner, &[]);
    let token = "token-not-in-whitelist";
    let reward_ratio = Decimal::percent(10);
    let reward_info = RewardInfo {
        token: token.to_string(),
        reward_ratio,
    };
    let msg = ExecuteMsg::UpdateRewardInfo {
        reward_info: reward_info.clone(),
    };
    let res = execute(deps.as_mut(), env, info, msg);
    assert!(res.is_err());

    let err = res.unwrap_err();
    assert_eq!(err, ContractError::NotInWhitelist {});
}

// ============= burn =============
// 5a. burn properly
#[test]
fn burn_properly() {
    let owner = "owner";
    let burner = "burner";

    let burner_funds = vec![coin(1_000_000, "uluna")];
    let mut app = App::new(|router, _, storage| {
        // initialization moved to App construction
        router
            .bank
            .init_balance(storage, &Addr::unchecked(burner), burner_funds.clone())
            .unwrap();
    });

    let cw20_reward = 1_000_000_000;
    let development_config = DevelopmentConfig {
        fee_ratio: Decimal::percent(2),
        beneficiary: "beneficiary".to_string(),
    };
    let setup_res = helpers::setup_multi_test_contracts(
        &mut app,
        owner,
        cw20_reward,
        Some(development_config.clone()),
    );
    let burning_addr = setup_res.burning_addr;
    let reward_list = setup_res.reward_list;

    // burn
    let burn_amount = burner_funds[0].amount;
    let _res = app
        .execute_contract(
            Addr::unchecked(burner),
            burning_addr.clone(),
            &ExecuteMsg::Burn {},
            &vec![Coin {
                denom: "uluna".to_string(),
                amount: burn_amount,
            }],
        )
        .unwrap();

    // check development fee is collected
    let beneficiary_balance = app
        .wrap()
        .query_balance(development_config.beneficiary, "uluna")
        .unwrap();
    let development_fee = burn_amount * development_config.fee_ratio;
    assert_eq!(beneficiary_balance.amount, development_fee);
    let send_tax = development_fee * LUNC_TAX;
    let actual_burned_amount = burn_amount - development_fee - send_tax;

    println!("burned amount: {}", burn_amount);
    println!("send tax: {}", send_tax);
    println!("development fee: {}", development_fee);
    println!("actual burned amount: {}", actual_burned_amount);

    // check burned amount
    let burned_amount_query: BurnedAmountResponse = app
        .wrap()
        .query_wasm_smart(burning_addr.clone(), &QueryMsg::BurnedAmount {})
        .unwrap();
    assert_eq!(burned_amount_query.burned_amount, actual_burned_amount);

    // contract not hold any burned token
    let burn_token_balance = app
        .wrap()
        .query_balance(Addr::unchecked(burning_addr), "uluna")
        .unwrap();
    assert_eq!(burn_token_balance, coin(send_tax.u128(), "uluna")); // tax will be sent in real transaction

    // check reward tokens of burner
    for reward in reward_list {
        let reward_token_balance: Cw20BalanceResponse = app
            .wrap()
            .query_wasm_smart(
                Addr::unchecked(reward.token),
                &Cw20QueryMsg::Balance {
                    address: burner.to_string(),
                },
            )
            .unwrap();
        let expected_reward_amount = actual_burned_amount * reward.reward_ratio;
        assert_eq!(reward_token_balance.balance, expected_reward_amount);
    }
}

// 5b. burn zero amount
#[test]
fn fail_to_burn_zero_amount() {
    let owner = "owner";
    let burner = "burner";

    let mut app = App::default();
    let setup_res = helpers::setup_multi_test_contracts(&mut app, owner, 1000, None);
    let burning_addr = setup_res.burning_addr;

    // burn
    let res = app.execute_contract(
        Addr::unchecked(burner),
        burning_addr.clone(),
        &ExecuteMsg::Burn {},
        &vec![],
    );
    assert!(res.is_err());

    let err = res.unwrap_err();
    assert_eq!(ContractError::ZeroAmount {}, err.downcast().unwrap());
}

// 5c. burn not enough rewards
#[test]
fn burn_not_enough_rewards() {
    let owner = "owner";
    let burner = "burner";

    let burner_funds = vec![coin(1_000_000, "uluna")];
    let mut app = App::new(|router, _, storage| {
        // initialization moved to App construction
        router
            .bank
            .init_balance(storage, &Addr::unchecked(burner), burner_funds.clone())
            .unwrap();
    });

    let cw20_reward = 0;
    let development_config = DevelopmentConfig {
        fee_ratio: Decimal::percent(2),
        beneficiary: "beneficiary".to_string(),
    };
    let setup_res = helpers::setup_multi_test_contracts(
        &mut app,
        owner,
        cw20_reward,
        Some(development_config.clone()),
    );
    let burning_addr = setup_res.burning_addr;
    let reward_list = setup_res.reward_list;

    // burn
    let burn_amount = burner_funds[0].amount;
    app.execute_contract(
        Addr::unchecked(burner),
        burning_addr.clone(),
        &ExecuteMsg::Burn {},
        &vec![Coin {
            denom: "uluna".to_string(),
            amount: burn_amount,
        }],
    )
    .unwrap();

    // check development fee is collected
    let beneficiary_balance = app
        .wrap()
        .query_balance(development_config.beneficiary, "uluna")
        .unwrap();
    let development_fee = burn_amount * development_config.fee_ratio;
    assert_eq!(beneficiary_balance.amount, development_fee);

    let send_tax = development_fee * LUNC_TAX;
    let actual_burned_amount = burn_amount - development_fee - send_tax;

    // check burned amount
    let burned_amount_query: BurnedAmountResponse = app
        .wrap()
        .query_wasm_smart(burning_addr.clone(), &QueryMsg::BurnedAmount {})
        .unwrap();
    assert_eq!(burned_amount_query.burned_amount, actual_burned_amount);

    // check reward tokens of burner
    for reward in reward_list {
        let reward_token_balance: Cw20BalanceResponse = app
            .wrap()
            .query_wasm_smart(
                Addr::unchecked(reward.token),
                &Cw20QueryMsg::Balance {
                    address: burner.to_string(),
                },
            )
            .unwrap();
        assert_eq!(reward_token_balance.balance, Uint128::zero());
    }
}

// 5d. burned amount increase
#[test]
fn burned_amount_should_increase() {
    let owner = "owner";
    let burner = "burner";

    let burner_funds = vec![coin(1_000_000, "uluna")];
    let mut app = App::new(|router, _, storage| {
        // initialization moved to App construction
        router
            .bank
            .init_balance(storage, &Addr::unchecked(burner), burner_funds.clone())
            .unwrap();
    });

    let cw20_reward = 1_000_000_000;
    let development_config = DevelopmentConfig {
        fee_ratio: Decimal::percent(2),
        beneficiary: "beneficiary".to_string(),
    };
    let setup_res = helpers::setup_multi_test_contracts(
        &mut app,
        owner,
        cw20_reward,
        Some(development_config.clone()),
    );
    let burning_addr = setup_res.burning_addr;

    // burn
    let mut total_burned = Uint128::zero();
    let mut total_development_fee = Uint128::zero();
    for _ in 0..10 {
        let burn_amount = Uint128::new(1000);
        let development_fee = burn_amount * development_config.fee_ratio;
        total_burned += burn_amount - development_fee;
        total_development_fee += development_fee;

        app.execute_contract(
            Addr::unchecked(burner),
            burning_addr.clone(),
            &ExecuteMsg::Burn {},
            &vec![Coin {
                denom: "uluna".to_string(),
                amount: burn_amount,
            }],
        )
        .unwrap();
        let burned_amount_query: BurnedAmountResponse = app
            .wrap()
            .query_wasm_smart(burning_addr.clone(), &QueryMsg::BurnedAmount {})
            .unwrap();
        assert_eq!(burned_amount_query.burned_amount, total_burned);

        // check development fee is collected
        let beneficiary_balance = app
            .wrap()
            .query_balance(development_config.beneficiary.clone(), "uluna")
            .unwrap();
        assert_eq!(beneficiary_balance.amount, total_development_fee);
    }
}
