use cosmwasm_std::{to_json_binary, Addr, Uint128};
use cw20::{BalanceResponse, Cw20Coin, MinterResponse};
use cw20_base::{
    contract::{execute as cw20_execute, instantiate as cw20_instantiate, query as cw20_query},
    msg::{
        ExecuteMsg as Cw20ExecuteMsg, InstantiateMsg as Cw20InstantiateMsg,
        QueryMsg as Cw20QueryMsg,
    },
};
use cw_multi_test::{App, ContractWrapper, Executor};

use crate::{
    contract::{execute, instantiate, query, REWARD_INFO},
    error::ContractError,
    msg::{
        CanStakeResponse, Cw20HookMsg, ExecuteMsg, InstantiateMsg, QueryMsg,
        RemainingRewardsResponse, RewardInfoResponse, StakedResponse, StakingPeriod,
        TotalPendingRewardsResponse,
    },
};

const TOTAL_AMOUNT: Uint128 = Uint128::new(1_000_000_000_000_000_000);
const INIT_AMOUNT: Uint128 = Uint128::new(850_000_000_000_000_000);
const MINTABLE_AMOUNT: Uint128 = Uint128::new(150_000_000_000_000_000);

struct SetupContractRes {
    token_address: Addr,
    staking_address: Addr,
}

fn setup_contracts(app: &mut App, sender: &str, owner: &str) -> SetupContractRes {
    let token_code = ContractWrapper::new(cw20_execute, cw20_instantiate, cw20_query);
    let token_code_id = app.store_code(Box::new(token_code));
    let token_address = app
        .instantiate_contract(
            token_code_id,
            Addr::unchecked(sender),
            &Cw20InstantiateMsg {
                name: "name".to_string(),
                symbol: "symbol".to_string(),
                decimals: 6,
                initial_balances: vec![Cw20Coin {
                    address: owner.to_string(),
                    amount: INIT_AMOUNT,
                }],
                mint: Some(MinterResponse {
                    minter: sender.to_string(),
                    cap: Some(TOTAL_AMOUNT),
                }),
                marketing: None,
            },
            &[],
            "Cw20Contract",
            None,
        )
        .unwrap();

    let staking_code = ContractWrapper::new(execute, instantiate, query);
    let staking_code_id = app.store_code(Box::new(staking_code));
    let staking_address = app
        .instantiate_contract(
            staking_code_id,
            Addr::unchecked(sender),
            &InstantiateMsg {
                token_address: token_address.to_owned(),
            },
            &[],
            "StakingContract",
            None,
        )
        .unwrap();

    // update minter
    app.execute_contract(
        Addr::unchecked(sender),
        token_address.clone(),
        &Cw20ExecuteMsg::UpdateMinter {
            new_minter: Some(staking_address.to_string()),
        },
        &[],
    )
    .unwrap();

    SetupContractRes {
        token_address,
        staking_address,
    }
}

#[test]
fn proper_initialization() {
    let mut app = App::default();
    let sender = "sender";
    let owner = "owner";

    let cw20_code = ContractWrapper::new(cw20_execute, cw20_instantiate, cw20_query);
    let cw20_code_id = app.store_code(Box::new(cw20_code));
    let cw20_address = app
        .instantiate_contract(
            cw20_code_id,
            Addr::unchecked(sender),
            &Cw20InstantiateMsg {
                name: "name".to_string(),
                symbol: "symbol".to_string(),
                decimals: 6,
                initial_balances: vec![Cw20Coin {
                    address: owner.to_string(),
                    amount: INIT_AMOUNT,
                }],
                mint: Some(MinterResponse {
                    minter: sender.to_string(),
                    cap: Some(TOTAL_AMOUNT),
                }),
                marketing: None,
            },
            &[],
            "Cw20Contract",
            None,
        )
        .unwrap();

    let staking_code = ContractWrapper::new(execute, instantiate, query);
    let staking_code_id = app.store_code(Box::new(staking_code));
    let staking_address = app
        .instantiate_contract(
            staking_code_id,
            Addr::unchecked(sender),
            &InstantiateMsg {
                token_address: cw20_address.to_owned(),
            },
            &[],
            "StakingContract",
            None,
        )
        .unwrap();

    // check staking contract
    // check reward info
    let reward_info_res: RewardInfoResponse = app
        .wrap()
        .query_wasm_smart(staking_address.clone(), &QueryMsg::RewardInfo {})
        .unwrap();
    assert_eq!(reward_info_res.reward_info, REWARD_INFO);
    assert_eq!(reward_info_res.token_reward, cw20_address);

    // check staking status
    let can_stake_res: CanStakeResponse = app
        .wrap()
        .query_wasm_smart(staking_address.clone(), &QueryMsg::CanStake {})
        .unwrap();
    assert_eq!(can_stake_res.can_stake, true);

    // check remaining rewards
    let remaining_rewards_res: RemainingRewardsResponse = app
        .wrap()
        .query_wasm_smart(staking_address.clone(), &QueryMsg::RemainingRewards {})
        .unwrap();
    assert_eq!(remaining_rewards_res.remaining_rewards, MINTABLE_AMOUNT);

    // check total pending rewards
    let total_pending_rewards_res: TotalPendingRewardsResponse = app
        .wrap()
        .query_wasm_smart(staking_address.clone(), &QueryMsg::TotalPendingRewards {})
        .unwrap();
    assert_eq!(
        total_pending_rewards_res.total_pending_rewards,
        Uint128::zero()
    );
}

#[test]
fn stake_token() {
    let mut app = App::default();
    let sender = "sender";
    let owner = "owner";

    let SetupContractRes {
        token_address,
        staking_address,
    } = setup_contracts(&mut app, sender, owner);

    // before staking owner balance
    let owner_balance_res: BalanceResponse = app
        .wrap()
        .query_wasm_smart(
            token_address.clone(),
            &Cw20QueryMsg::Balance {
                address: owner.to_string(),
            },
        )
        .unwrap();
    let before_staking_owner_balance = owner_balance_res.balance;

    // stake token
    let staking_period = StakingPeriod::Short;
    app.execute_contract(
        Addr::unchecked(owner),
        token_address.clone(),
        &Cw20ExecuteMsg::Send {
            contract: staking_address.to_string(),
            amount: Uint128::new(10_000),
            msg: to_json_binary(&Cw20HookMsg::Stake {
                staking_period: staking_period.clone(),
            })
            .unwrap(),
        },
        &[],
    )
    .unwrap();

    let after_staking_owner_balance_res: BalanceResponse = app
        .wrap()
        .query_wasm_smart(
            token_address.clone(),
            &Cw20QueryMsg::Balance {
                address: owner.to_string(),
            },
        )
        .unwrap();
    let after_staking_owner_balance = after_staking_owner_balance_res.balance;

    // check staking status
    let staked_res: StakedResponse = app
        .wrap()
        .query_wasm_smart(
            staking_address.clone(),
            &QueryMsg::Staked {
                address: Addr::unchecked(owner),
            },
        )
        .unwrap();
    let reward_info = REWARD_INFO
        .iter()
        .find(|x| x.staking_period == staking_period)
        .unwrap();
    let expect_staked_amount = before_staking_owner_balance - after_staking_owner_balance;
    assert_eq!(staked_res.staked_amount, expect_staked_amount);
    assert_eq!(
        staked_res.pending_reward,
        expect_staked_amount * reward_info.reward_rate
    );
    assert_eq!(
        staked_res.claim_reward_at,
        app.block_info()
            .time
            .plus_days(reward_info.staking_days)
            .seconds()
    );

    // check remaining rewards
    let remaining_rewards_res: RemainingRewardsResponse = app
        .wrap()
        .query_wasm_smart(staking_address.clone(), &QueryMsg::RemainingRewards {})
        .unwrap();
    assert_eq!(remaining_rewards_res.remaining_rewards, MINTABLE_AMOUNT);

    // check total pending rewards
    let total_pending_rewards_res: TotalPendingRewardsResponse = app
        .wrap()
        .query_wasm_smart(staking_address.clone(), &QueryMsg::TotalPendingRewards {})
        .unwrap();
    assert_eq!(
        total_pending_rewards_res.total_pending_rewards,
        staked_res.pending_reward
    );
}

#[test]
fn unstake_token_without_reward() {
    let mut app = App::default();
    let sender = "sender";
    let owner = "owner";

    let SetupContractRes {
        token_address,
        staking_address,
    } = setup_contracts(&mut app, sender, owner);

    // before staking owner balance
    let owner_balance_res: BalanceResponse = app
        .wrap()
        .query_wasm_smart(
            token_address.clone(),
            &Cw20QueryMsg::Balance {
                address: owner.to_string(),
            },
        )
        .unwrap();
    let before_staking_owner_balance = owner_balance_res.balance;

    // stake token
    let staking_period = StakingPeriod::Medium;
    app.execute_contract(
        Addr::unchecked(owner),
        token_address.clone(),
        &Cw20ExecuteMsg::Send {
            contract: staking_address.to_string(),
            amount: Uint128::new(10_000),
            msg: to_json_binary(&Cw20HookMsg::Stake { staking_period }).unwrap(),
        },
        &[],
    )
    .unwrap();

    // unstake token
    app.execute_contract(
        Addr::unchecked(owner),
        staking_address.clone(),
        &ExecuteMsg::Unstake {},
        &[],
    )
    .unwrap();

    // check staking status
    let staked_res: StakedResponse = app
        .wrap()
        .query_wasm_smart(
            staking_address.clone(),
            &QueryMsg::Staked {
                address: Addr::unchecked(owner),
            },
        )
        .unwrap();
    assert_eq!(staked_res.staked_amount, Uint128::zero());
    assert_eq!(staked_res.pending_reward, Uint128::zero());
    assert_eq!(staked_res.claim_reward_at, 0);

    // check remaining rewards
    let remaining_rewards_res: RemainingRewardsResponse = app
        .wrap()
        .query_wasm_smart(staking_address.clone(), &QueryMsg::RemainingRewards {})
        .unwrap();
    assert_eq!(remaining_rewards_res.remaining_rewards, MINTABLE_AMOUNT);

    // check total pending rewards
    let total_pending_rewards_res: TotalPendingRewardsResponse = app
        .wrap()
        .query_wasm_smart(staking_address.clone(), &QueryMsg::TotalPendingRewards {})
        .unwrap();
    assert_eq!(
        total_pending_rewards_res.total_pending_rewards,
        Uint128::zero()
    );

    // check staker balance
    let owner_balance_res: BalanceResponse = app
        .wrap()
        .query_wasm_smart(
            token_address.clone(),
            &Cw20QueryMsg::Balance {
                address: owner.to_string(),
            },
        )
        .unwrap();
    assert_eq!(owner_balance_res.balance, before_staking_owner_balance);
}

#[test]
fn unstake_token_with_reward() {
    let mut app = App::default();
    let sender = "sender";
    let owner = "owner";

    let SetupContractRes {
        token_address,
        staking_address,
    } = setup_contracts(&mut app, sender, owner);

    // before staking owner balance
    let owner_balance_res: BalanceResponse = app
        .wrap()
        .query_wasm_smart(
            token_address.clone(),
            &Cw20QueryMsg::Balance {
                address: owner.to_string(),
            },
        )
        .unwrap();
    let before_staking_owner_balance = owner_balance_res.balance;

    // stake token
    let staking_period = StakingPeriod::Long;
    app.execute_contract(
        Addr::unchecked(owner),
        token_address.clone(),
        &Cw20ExecuteMsg::Send {
            contract: staking_address.to_string(),
            amount: Uint128::new(10_000),
            msg: to_json_binary(&Cw20HookMsg::Stake {
                staking_period: staking_period.clone(),
            })
            .unwrap(),
        },
        &[],
    )
    .unwrap();

    // update block
    let reward_info = REWARD_INFO
        .iter()
        .find(|x| x.staking_period == staking_period)
        .unwrap();
    app.update_block(|block| {
        block.time = block.time.plus_days(reward_info.staking_days);
    });

    // check staking status before unstaking
    let staked_res: StakedResponse = app
        .wrap()
        .query_wasm_smart(
            staking_address.clone(),
            &QueryMsg::Staked {
                address: Addr::unchecked(owner),
            },
        )
        .unwrap();
    let reward = staked_res.pending_reward;

    // unstake token
    app.execute_contract(
        Addr::unchecked(owner),
        staking_address.clone(),
        &ExecuteMsg::Unstake {},
        &[],
    )
    .unwrap();

    // check staking status after unstaking
    let staked_res: StakedResponse = app
        .wrap()
        .query_wasm_smart(
            staking_address.clone(),
            &QueryMsg::Staked {
                address: Addr::unchecked(owner),
            },
        )
        .unwrap();
    assert_eq!(staked_res.staked_amount, Uint128::zero());
    assert_eq!(staked_res.pending_reward, Uint128::zero());
    assert_eq!(staked_res.claim_reward_at, 0);

    // check remaining rewards
    let remaining_rewards_res: RemainingRewardsResponse = app
        .wrap()
        .query_wasm_smart(staking_address.clone(), &QueryMsg::RemainingRewards {})
        .unwrap();
    assert_eq!(
        remaining_rewards_res.remaining_rewards,
        MINTABLE_AMOUNT - reward
    );

    // check total pending rewards
    let total_pending_rewards_res: TotalPendingRewardsResponse = app
        .wrap()
        .query_wasm_smart(staking_address.clone(), &QueryMsg::TotalPendingRewards {})
        .unwrap();
    assert_eq!(
        total_pending_rewards_res.total_pending_rewards,
        Uint128::zero()
    );

    // check staker balance
    let owner_balance_res: BalanceResponse = app
        .wrap()
        .query_wasm_smart(
            token_address.clone(),
            &Cw20QueryMsg::Balance {
                address: owner.to_string(),
            },
        )
        .unwrap();
    assert_eq!(
        owner_balance_res.balance,
        before_staking_owner_balance + reward
    );
}

#[test]
fn error_when_insufficient_reward() {
    let mut app = App::default();
    let sender = "sender";
    let owner = "owner";

    let SetupContractRes {
        token_address,
        staking_address,
    } = setup_contracts(&mut app, sender, owner);

    // stake token
    let staking_period = StakingPeriod::Long;
    let err = app
        .execute_contract(
            Addr::unchecked(owner),
            token_address.clone(),
            &Cw20ExecuteMsg::Send {
                contract: staking_address.to_string(),
                amount: INIT_AMOUNT,
                msg: to_json_binary(&Cw20HookMsg::Stake { staking_period }).unwrap(),
            },
            &[],
        )
        .unwrap_err();
    assert_eq!(
        ContractError::InsufficientRewards {},
        err.downcast().unwrap(),
    );
}

#[test]
fn unstake_and_stake_again() {
    let mut app = App::default();
    let sender = "sender";
    let owner = "owner";

    let SetupContractRes {
        token_address,
        staking_address,
    } = setup_contracts(&mut app, sender, owner);

    // before staking owner balance
    let before_owner_balance_res: BalanceResponse = app
        .wrap()
        .query_wasm_smart(
            token_address.clone(),
            &Cw20QueryMsg::Balance {
                address: owner.to_string(),
            },
        )
        .unwrap();
    let before_owner_balance = before_owner_balance_res.balance;

    // stake token
    let staking_period = StakingPeriod::Short;
    app.execute_contract(
        Addr::unchecked(owner),
        token_address.clone(),
        &Cw20ExecuteMsg::Send {
            contract: staking_address.to_string(),
            amount: Uint128::new(10_000),
            msg: to_json_binary(&Cw20HookMsg::Stake { staking_period }).unwrap(),
        },
        &[],
    )
    .unwrap();

    // unstake token before claim reward date
    app.execute_contract(
        Addr::unchecked(owner),
        staking_address.clone(),
        &ExecuteMsg::Unstake {},
        &[],
    )
    .unwrap();

    // stake token again
    let staking_period = StakingPeriod::Medium;
    app.execute_contract(
        Addr::unchecked(owner),
        token_address.clone(),
        &Cw20ExecuteMsg::Send {
            contract: staking_address.to_string(),
            amount: Uint128::new(100_000),
            msg: to_json_binary(&Cw20HookMsg::Stake {
                staking_period: staking_period.clone(),
            })
            .unwrap(),
        },
        &[],
    )
    .unwrap();

    // update block
    let reward_info = REWARD_INFO
        .iter()
        .find(|x| x.staking_period == staking_period)
        .unwrap();
    app.update_block(|block| {
        block.time = block.time.plus_days(reward_info.staking_days);
    });

    // check staking status before unstaking
    let staked_res: StakedResponse = app
        .wrap()
        .query_wasm_smart(
            staking_address.clone(),
            &QueryMsg::Staked {
                address: Addr::unchecked(owner),
            },
        )
        .unwrap();
    let reward = staked_res.pending_reward;

    // unstake token at claim reward date
    app.execute_contract(
        Addr::unchecked(owner),
        staking_address.clone(),
        &ExecuteMsg::Unstake {},
        &[],
    )
    .unwrap();

    // check remaining rewards
    let remaining_rewards_res: RemainingRewardsResponse = app
        .wrap()
        .query_wasm_smart(staking_address.clone(), &QueryMsg::RemainingRewards {})
        .unwrap();
    assert_eq!(
        remaining_rewards_res.remaining_rewards,
        MINTABLE_AMOUNT - reward
    );

    // check total pending rewards
    let total_pending_rewards_res: TotalPendingRewardsResponse = app
        .wrap()
        .query_wasm_smart(staking_address.clone(), &QueryMsg::TotalPendingRewards {})
        .unwrap();
    assert_eq!(
        total_pending_rewards_res.total_pending_rewards,
        Uint128::zero()
    );

    // check staker balance
    let owner_balance_res: BalanceResponse = app
        .wrap()
        .query_wasm_smart(
            token_address.clone(),
            &Cw20QueryMsg::Balance {
                address: owner.to_string(),
            },
        )
        .unwrap();
    assert_eq!(owner_balance_res.balance, before_owner_balance + reward);
}

#[test]
fn multiple_staker() {
    let mut app = App::default();
    let sender = "sender";
    let owner = "owner";
    let stakers = ["staker1", "staker2", "staker3"];
    let staker_balances = [
        Uint128::new(100_000),
        Uint128::new(1_000_000),
        Uint128::new(10_000_000),
    ];

    let SetupContractRes {
        token_address,
        staking_address,
    } = setup_contracts(&mut app, sender, owner);

    // transfer token from owner to stakers
    for index in 0..stakers.len() {
        app.execute_contract(
            Addr::unchecked(owner),
            token_address.clone(),
            &Cw20ExecuteMsg::Transfer {
                recipient: stakers[index].to_string(),
                amount: staker_balances[index],
            },
            &[],
        )
        .unwrap();
    }

    // staking
    let staking_periods = [
        StakingPeriod::Long,
        StakingPeriod::Medium,
        StakingPeriod::Short,
    ];
    for index in 0..stakers.len() {
        app.execute_contract(
            Addr::unchecked(stakers[index]),
            token_address.clone(),
            &Cw20ExecuteMsg::Send {
                contract: staking_address.to_string(),
                amount: staker_balances[index],
                msg: to_json_binary(&Cw20HookMsg::Stake {
                    staking_period: staking_periods[index].clone(),
                })
                .unwrap(),
            },
            &[],
        )
        .unwrap();
    }

    // check staking status
    let mut expect_total_rewards = Uint128::zero();
    let mut rewards = vec![];
    for index in 0..stakers.len() {
        let staked_res: StakedResponse = app
            .wrap()
            .query_wasm_smart(
                staking_address.clone(),
                &QueryMsg::Staked {
                    address: Addr::unchecked(stakers[index]),
                },
            )
            .unwrap();
        let reward_info = REWARD_INFO
            .iter()
            .find(|x| x.staking_period == staking_periods[index])
            .unwrap();
        let expect_staked_amount = staker_balances[index];
        assert_eq!(staked_res.staked_amount, expect_staked_amount);
        assert_eq!(
            staked_res.pending_reward,
            expect_staked_amount * reward_info.reward_rate
        );
        assert_eq!(
            staked_res.claim_reward_at,
            app.block_info()
                .time
                .plus_days(reward_info.staking_days)
                .seconds()
        );

        expect_total_rewards += staked_res.pending_reward;
        rewards.push(staked_res.pending_reward);
    }

    // check total pending rewards before unstaking
    let total_pending_rewards_res: TotalPendingRewardsResponse = app
        .wrap()
        .query_wasm_smart(staking_address.clone(), &QueryMsg::TotalPendingRewards {})
        .unwrap();
    assert_eq!(
        total_pending_rewards_res.total_pending_rewards,
        expect_total_rewards
    );

    // unstake 1 account before claim reward date
    app.execute_contract(
        Addr::unchecked(stakers[0]),
        staking_address.clone(),
        &ExecuteMsg::Unstake {},
        &[],
    )
    .unwrap();

    // update block
    let reward_info = REWARD_INFO
        .iter()
        .find(|x| x.staking_period == StakingPeriod::Long)
        .unwrap();
    app.update_block(|block| {
        block.time = block.time.plus_days(reward_info.staking_days);
    });

    // unstake remaining accounts after claim reward date
    for index in 1..stakers.len() {
        app.execute_contract(
            Addr::unchecked(stakers[index]),
            staking_address.clone(),
            &ExecuteMsg::Unstake {},
            &[],
        )
        .unwrap();
    }

    // check remaining rewards
    let remaining_rewards_res: RemainingRewardsResponse = app
        .wrap()
        .query_wasm_smart(staking_address.clone(), &QueryMsg::RemainingRewards {})
        .unwrap();
    assert_eq!(
        remaining_rewards_res.remaining_rewards,
        MINTABLE_AMOUNT - rewards[1] - rewards[2]
    );

    // check total pending rewards
    let total_pending_rewards_res: TotalPendingRewardsResponse = app
        .wrap()
        .query_wasm_smart(staking_address.clone(), &QueryMsg::TotalPendingRewards {})
        .unwrap();
    assert_eq!(
        total_pending_rewards_res.total_pending_rewards,
        Uint128::zero()
    );

    // check staker balances
    for index in 0..stakers.len() {
        let staker_balance_res: BalanceResponse = app
            .wrap()
            .query_wasm_smart(
                token_address.clone(),
                &Cw20QueryMsg::Balance {
                    address: stakers[index].to_string(),
                },
            )
            .unwrap();
        // first staker unstake before claim reward date
        if index == 0 {
            assert_eq!(staker_balance_res.balance, staker_balances[index]);
        } else {
            assert_eq!(
                staker_balance_res.balance,
                staker_balances[index] + rewards[index]
            );
        }
    }
}
