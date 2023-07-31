use cosmwasm_std::{
    from_binary,
    testing::{mock_dependencies, mock_env, mock_info},
    Addr, Binary, Decimal, Uint128,
};
use cw20::{Cw20Coin, TokenInfoResponse};
use cw20_base::{msg::InstantiateMsg as Cw20InstantiateMsg, ContractError};

use crate::{
    execute, instantiate,
    msg::{
        CollectTaxAddressResponse, ConfigResponse, ExecuteMsg, InstantiateMsg, OwnerResponse,
        QueryMsg, TaxFreeAddressResponse, TaxInfoResponse,
    },
    query,
    state::{FractionFormat, TaxInfo},
};

fn mock_cw20_instantiate_msg(initial_balances: Vec<Cw20Coin>) -> Cw20InstantiateMsg {
    Cw20InstantiateMsg {
        name: "Cremation Coin".to_string(),
        symbol: "CREMAT".to_string(),
        decimals: 6,
        initial_balances,
        mint: None,
        marketing: None,
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
        owner: Addr::unchecked(owner),
        tax_info: TaxInfo {
            buy_tax: Some(tax_rate.clone()),
            sell_tax: Some(tax_rate.clone()),
            transfer_tax: None,
        },
        cw20_instantiate_msg: Cw20InstantiateMsg {
            name: "Cremation Coin".to_string(),
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
    // set config
    execute(
        deps.as_mut(),
        mock_env(),
        mock_info(creator, &[]),
        ExecuteMsg::SetConfig {
            terraswap_router: Addr::unchecked("terraswap_router"),
            terraswap_pair: Addr::unchecked("terraswap_pair"),
        },
    )
    .unwrap();

    // check token_info
    let token_info_query = query(deps.as_ref(), mock_env(), QueryMsg::TokenInfo {}).unwrap();
    let token_info_res: TokenInfoResponse = from_binary(&token_info_query).unwrap();
    assert_eq!(token_info_res.name, "Cremation Coin");
    assert_eq!(token_info_res.symbol, "CREMAT");
    assert_eq!(token_info_res.decimals, 6);
    assert_eq!(token_info_res.total_supply, total_supply);

    // check owner balance
    let balance_query = query(
        deps.as_ref(),
        mock_env(),
        QueryMsg::Balance {
            address: owner.clone().into(),
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
    let config_query = query(deps.as_ref(), mock_env(), QueryMsg::Config {}).unwrap();
    let config_res: ConfigResponse = from_binary(&config_query).unwrap();
    assert_eq!(config_res.terraswap_pair, Addr::unchecked("terraswap_pair"));
    assert_eq!(
        config_res.terraswap_router,
        Addr::unchecked("terraswap_router")
    );

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
    let env = mock_env();
    let creator = "creator";
    let owner = "owner";
    let info = mock_info(creator, &[]);
    let msg = InstantiateMsg {
        owner: Addr::unchecked(owner),
        tax_info: TaxInfo {
            buy_tax: None,
            sell_tax: None,
            transfer_tax: None,
        },
        cw20_instantiate_msg: mock_cw20_instantiate_msg(vec![]),
    };
    let _res = instantiate(deps.as_mut(), env, info.clone(), msg).unwrap();
    let msg: ExecuteMsg = ExecuteMsg::SetConfig {
        terraswap_router: Addr::unchecked("terraswap_router"),
        terraswap_pair: Addr::unchecked("terraswap_pair"),
    };
    let _res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();

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
    let owner_res: OwnerResponse = from_binary(&owner_query).unwrap();
    assert_eq!(owner_res.owner, new_owner);
}

#[test]
fn update_tax_info() {
    let mut deps = mock_dependencies();
    let env = mock_env();
    let owner = "owner";
    let info = mock_info("creator", &[]);
    let tax_rate = FractionFormat {
        numerator: Uint128::new(8),
        denominator: Uint128::new(100),
    };
    let msg = InstantiateMsg {
        owner: Addr::unchecked(owner),
        tax_info: TaxInfo {
            buy_tax: Some(tax_rate.clone()),
            sell_tax: Some(tax_rate.clone()),
            transfer_tax: None,
        },
        cw20_instantiate_msg: mock_cw20_instantiate_msg(vec![]),
    };
    let _res = instantiate(deps.as_mut(), env, info, msg).unwrap();

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
    let env = mock_env();
    let owner = "owner";
    let info = mock_info("creator", &[]);
    let msg = InstantiateMsg {
        owner: Addr::unchecked(owner),
        tax_info: TaxInfo {
            buy_tax: None,
            sell_tax: None,
            transfer_tax: None,
        },
        cw20_instantiate_msg: mock_cw20_instantiate_msg(vec![]),
    };
    let _res = instantiate(deps.as_mut(), env, info, msg).unwrap();

    // fail to change collect tax address with non-owner
    let non_owner = "non_owner";
    let info = mock_info(non_owner, &[]);
    let msg = ExecuteMsg::UpdateCollectTaxAddress {
        new_collect_tax_addr: Addr::unchecked(non_owner),
    };
    let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap_err();
    assert_eq!(res, ContractError::Unauthorized {});

    // change collect tax address
    let new_collect_tax_addr = Addr::unchecked("new_collect_tax_addr");
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
    let env = mock_env();
    let owner = "owner";
    let info = mock_info("creator", &[]);
    let msg = InstantiateMsg {
        owner: Addr::unchecked(owner),
        tax_info: TaxInfo {
            buy_tax: None,
            sell_tax: None,
            transfer_tax: None,
        },
        cw20_instantiate_msg: mock_cw20_instantiate_msg(vec![]),
    };
    let _res = instantiate(deps.as_mut(), env, info, msg).unwrap();

    // fail to set tax free address with non-owner
    let non_owner = "non_owner";
    let info = mock_info(non_owner, &[]);
    let msg = ExecuteMsg::SetTaxFreeAddress {
        address: Addr::unchecked(non_owner),
        tax_free: true,
    };
    let err = execute(deps.as_mut(), mock_env(), info, msg).unwrap_err();
    assert_eq!(err, ContractError::Unauthorized {});

    // set tax free address
    let tax_free_addr = "tax_free_addr";
    let info = mock_info(owner, &[]);
    let msg = ExecuteMsg::SetTaxFreeAddress {
        address: Addr::unchecked(tax_free_addr),
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
        address: Addr::unchecked(tax_free_addr),
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

// test collect sell tax when execute send cw20
// seller -> terraswap router
#[test]
fn collect_sell_tax_when_execute_send() {
    let mut deps = mock_dependencies();
    let env = mock_env();
    let creator = "creator";
    let owner = "owner";
    let terraswap_router = "terraswap_router";
    let seller = "seller";
    let seller_balance = Uint128::new(100_000);
    let tax_rate = FractionFormat {
        numerator: Uint128::new(8),
        denominator: Uint128::new(100),
    };
    let msg = InstantiateMsg {
        owner: Addr::unchecked(owner),
        tax_info: TaxInfo {
            buy_tax: None,
            sell_tax: Some(tax_rate.clone()),
            transfer_tax: None,
        },
        cw20_instantiate_msg: mock_cw20_instantiate_msg(vec![Cw20Coin {
            address: seller.to_string(),
            amount: seller_balance,
        }]),
    };
    instantiate(deps.as_mut(), env, mock_info(creator, &[]), msg).unwrap();
    // set config
    execute(
        deps.as_mut(),
        mock_env(),
        mock_info(creator, &[]),
        ExecuteMsg::SetConfig {
            terraswap_router: Addr::unchecked("terraswap_router"),
            terraswap_pair: Addr::unchecked("terraswap_pair"),
        },
    )
    .unwrap();

    // send from buyer to terraswap router
    let info = mock_info(seller, &[]);
    let send_amount = Uint128::new(100);
    let expect_tax_amount = send_amount.multiply_ratio(tax_rate.numerator, tax_rate.denominator);
    let msg = ExecuteMsg::Send {
        contract: terraswap_router.to_string(),
        amount: send_amount,
        msg: Binary::default(),
    };
    let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();
    let tax_opt = res.attributes.iter().find(|attr| attr.key == "tax_amount");
    assert!(tax_opt.is_some());
    assert_eq!(tax_opt.unwrap().value, expect_tax_amount.to_string());

    // check sender balance
    let balance_query = query(
        deps.as_ref(),
        mock_env(),
        QueryMsg::Balance {
            address: seller.to_string(),
        },
    )
    .unwrap();
    let balance_res: cw20::BalanceResponse = from_binary(&balance_query).unwrap();
    assert_eq!(balance_res.balance, seller_balance - send_amount);

    // check receiver balance
    let balance_query = query(
        deps.as_ref(),
        mock_env(),
        QueryMsg::Balance {
            address: terraswap_router.to_string(),
        },
    )
    .unwrap();
    let balance_res: cw20::BalanceResponse = from_binary(&balance_query).unwrap();
    assert_eq!(balance_res.balance, send_amount - expect_tax_amount);

    // check collect tax address balance
    let tax_addr_balance_query = query(
        deps.as_ref(),
        mock_env(),
        QueryMsg::Balance {
            address: owner.to_string(),
        },
    );
    let tax_addr_balance_res: cw20::BalanceResponse =
        from_binary(&tax_addr_balance_query.unwrap()).unwrap();
    assert_eq!(tax_addr_balance_res.balance, expect_tax_amount);

    // send from non tax-free address
    let info = mock_info(seller, &[]);
    let send_amount = Uint128::new(100);
    let expect_tax_amount = send_amount.multiply_ratio(tax_rate.numerator, tax_rate.denominator);
    let msg = ExecuteMsg::Send {
        contract: terraswap_router.to_string(),
        amount: send_amount,
        msg: Binary::default(),
    };
    let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();
    let tax_opt = res.attributes.iter().find(|attr| attr.key == "tax_amount");
    assert!(tax_opt.is_some());
    assert_eq!(tax_opt.unwrap().value, expect_tax_amount.to_string());
}

// test collect sell tax when execute send_from cw20
// seller approves spender
// spender send cw20 from seller -> terraswap router
#[test]
fn collect_sell_tax_when_execute_send_from() {
    let mut deps = mock_dependencies();
    let env = mock_env();
    let creator = "creator";
    let owner = "owner";
    let terraswap_router = "terraswap_router";
    let spender = "spender";
    let seller = "seller";
    let seller_balance = Uint128::new(100_000);
    let sell_amount = Uint128::new(100);
    let tax_rate = FractionFormat {
        numerator: Uint128::new(8),
        denominator: Uint128::new(100),
    };
    let msg = InstantiateMsg {
        owner: Addr::unchecked(owner),
        tax_info: TaxInfo {
            buy_tax: None,
            sell_tax: Some(tax_rate.clone()),
            transfer_tax: None,
        },
        cw20_instantiate_msg: mock_cw20_instantiate_msg(vec![Cw20Coin {
            address: seller.to_string(),
            amount: seller_balance,
        }]),
    };
    instantiate(deps.as_mut(), env, mock_info(creator, &[]), msg).unwrap();
    // set config
    execute(
        deps.as_mut(),
        mock_env(),
        mock_info(creator, &[]),
        ExecuteMsg::SetConfig {
            terraswap_router: Addr::unchecked("terraswap_router"),
            terraswap_pair: Addr::unchecked("terraswap_pair"),
        },
    )
    .unwrap();

    // approve from seller to spender
    let info = mock_info(seller, &[]);
    let msg = ExecuteMsg::IncreaseAllowance {
        spender: spender.to_string(),
        amount: sell_amount,
        expires: None,
    };
    let _res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();

    // send from sender to terraswap router
    let info = mock_info(spender, &[]);
    let expect_tax_amount = sell_amount.multiply_ratio(tax_rate.numerator, tax_rate.denominator);
    let msg = ExecuteMsg::SendFrom {
        owner: seller.to_string(),
        contract: terraswap_router.to_string(),
        amount: sell_amount,
        msg: Binary::default(),
    };
    let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();
    let tax_opt = res.attributes.iter().find(|attr| attr.key == "tax_amount");
    assert!(tax_opt.is_some());
    assert_eq!(tax_opt.unwrap().value, expect_tax_amount.to_string());

    // check seller balance
    let balance_query = query(
        deps.as_ref(),
        mock_env(),
        QueryMsg::Balance {
            address: seller.to_string(),
        },
    )
    .unwrap();
    let balance_res: cw20::BalanceResponse = from_binary(&balance_query).unwrap();
    assert_eq!(balance_res.balance, seller_balance - sell_amount);

    // check receiver balance
    let balance_query = query(
        deps.as_ref(),
        mock_env(),
        QueryMsg::Balance {
            address: terraswap_router.to_string(),
        },
    )
    .unwrap();
    let balance_res: cw20::BalanceResponse = from_binary(&balance_query).unwrap();
    assert_eq!(balance_res.balance, sell_amount - expect_tax_amount);

    // check collect tax address balance
    let tax_addr_balance_query = query(
        deps.as_ref(),
        mock_env(),
        QueryMsg::Balance {
            address: owner.to_string(),
        },
    );
    let tax_addr_balance_res: cw20::BalanceResponse =
        from_binary(&tax_addr_balance_query.unwrap()).unwrap();
    assert_eq!(tax_addr_balance_res.balance, expect_tax_amount);
}

// test collect buy tax when execute transfer cw20
// terraswap pair -> buyer
#[test]
fn collect_buy_tax_when_execute_transfer() {
    let mut deps = mock_dependencies();
    let env = mock_env();
    let creator = "creator";
    let owner = "owner";
    let buyer = "buyer";
    let terraswap_pair = "terraswap_pair";
    let terraswap_pair_balance = Uint128::new(100_000);
    let tax_rate = FractionFormat {
        numerator: Uint128::new(8),
        denominator: Uint128::new(100),
    };
    let msg = InstantiateMsg {
        owner: Addr::unchecked(owner),
        tax_info: TaxInfo {
            buy_tax: Some(tax_rate.clone()),
            sell_tax: None,
            transfer_tax: None,
        },
        cw20_instantiate_msg: mock_cw20_instantiate_msg(vec![Cw20Coin {
            address: terraswap_pair.to_string(),
            amount: terraswap_pair_balance,
        }]),
    };
    instantiate(deps.as_mut(), env, mock_info(creator, &[]), msg).unwrap();
    // set config
    execute(
        deps.as_mut(),
        mock_env(),
        mock_info(creator, &[]),
        ExecuteMsg::SetConfig {
            terraswap_router: Addr::unchecked("terraswap_router"),
            terraswap_pair: Addr::unchecked("terraswap_pair"),
        },
    )
    .unwrap();

    // transfer from terraswap pair to buyer
    let info = mock_info(terraswap_pair, &[]);
    let transfer_amount = Uint128::new(100);
    let expect_tax_amount =
        transfer_amount.multiply_ratio(tax_rate.numerator, tax_rate.denominator);
    let msg = ExecuteMsg::Transfer {
        recipient: buyer.to_string(),
        amount: transfer_amount,
    };
    let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();
    let tax_opt = res.attributes.iter().find(|attr| attr.key == "tax_amount");
    assert!(tax_opt.is_some());
    assert_eq!(tax_opt.unwrap().value, expect_tax_amount.to_string());

    // check sender balance
    let balance_query = query(
        deps.as_ref(),
        mock_env(),
        QueryMsg::Balance {
            address: terraswap_pair.to_string(),
        },
    )
    .unwrap();
    let balance_res: cw20::BalanceResponse = from_binary(&balance_query).unwrap();
    assert_eq!(
        balance_res.balance,
        terraswap_pair_balance - transfer_amount
    );

    // check receiver balance
    let balance_query = query(
        deps.as_ref(),
        mock_env(),
        QueryMsg::Balance {
            address: buyer.to_string(),
        },
    )
    .unwrap();
    let balance_res: cw20::BalanceResponse = from_binary(&balance_query).unwrap();
    assert_eq!(balance_res.balance, transfer_amount - expect_tax_amount);

    // check collect tax address balance
    let tax_addr_balance_query = query(
        deps.as_ref(),
        mock_env(),
        QueryMsg::Balance {
            address: owner.to_string(),
        },
    );
    let tax_addr_balance_res: cw20::BalanceResponse =
        from_binary(&tax_addr_balance_query.unwrap()).unwrap();
    assert_eq!(tax_addr_balance_res.balance, expect_tax_amount);
}

// test collect buy tax when execute transfer_from cw20
// seller approves spender
// spender transfer cw20 from seller -> terraswap pair
#[test]
fn collect_buy_tax_when_execute_transfer_from() {
    let mut deps = mock_dependencies();
    let env = mock_env();
    let creator = "creator";
    let owner = "owner";
    let spender = "spender";
    let seller = "seller";
    let terraswap_pair = "terraswap_pair";
    let seller_balance = Uint128::new(100_000);
    let transfer_amount = Uint128::new(100);
    let tax_rate = FractionFormat {
        numerator: Uint128::new(8),
        denominator: Uint128::new(100),
    };
    let msg = InstantiateMsg {
        owner: Addr::unchecked(owner),
        tax_info: TaxInfo {
            buy_tax: None,
            sell_tax: Some(tax_rate.clone()),
            transfer_tax: None,
        },
        cw20_instantiate_msg: mock_cw20_instantiate_msg(vec![Cw20Coin {
            address: seller.to_string(),
            amount: seller_balance,
        }]),
    };
    instantiate(deps.as_mut(), env, mock_info("creator", &[]), msg).unwrap();
    // set config
    execute(
        deps.as_mut(),
        mock_env(),
        mock_info(creator, &[]),
        ExecuteMsg::SetConfig {
            terraswap_router: Addr::unchecked("terraswap_router"),
            terraswap_pair: Addr::unchecked("terraswap_pair"),
        },
    )
    .unwrap();

    // approve from seller to spender
    let info = mock_info(seller, &[]);
    let msg = ExecuteMsg::IncreaseAllowance {
        spender: spender.to_string(),
        amount: transfer_amount,
        expires: None,
    };
    let _res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();

    // transfer from spender to terraswap pair
    let info = mock_info(spender, &[]);
    let expect_tax_amount =
        transfer_amount.multiply_ratio(tax_rate.numerator, tax_rate.denominator);
    let msg = ExecuteMsg::TransferFrom {
        owner: seller.to_string(),
        recipient: terraswap_pair.to_string(),
        amount: transfer_amount,
    };
    let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();
    let tax_opt = res.attributes.iter().find(|attr| attr.key == "tax_amount");
    assert!(tax_opt.is_some());
    assert_eq!(tax_opt.unwrap().value, expect_tax_amount.to_string());

    // check seller balance
    let balance_query = query(
        deps.as_ref(),
        mock_env(),
        QueryMsg::Balance {
            address: seller.to_string(),
        },
    )
    .unwrap();
    let balance_res: cw20::BalanceResponse = from_binary(&balance_query).unwrap();
    assert_eq!(balance_res.balance, seller_balance - transfer_amount);

    // check receiver balance
    let balance_query = query(
        deps.as_ref(),
        mock_env(),
        QueryMsg::Balance {
            address: terraswap_pair.to_string(),
        },
    )
    .unwrap();
    let balance_res: cw20::BalanceResponse = from_binary(&balance_query).unwrap();
    assert_eq!(balance_res.balance, transfer_amount - expect_tax_amount);

    // check collect tax address balance
    let tax_addr_balance_query = query(
        deps.as_ref(),
        mock_env(),
        QueryMsg::Balance {
            address: owner.to_string(),
        },
    );
    let tax_addr_balance_res: cw20::BalanceResponse =
        from_binary(&tax_addr_balance_query.unwrap()).unwrap();
    assert_eq!(tax_addr_balance_res.balance, expect_tax_amount);
}
