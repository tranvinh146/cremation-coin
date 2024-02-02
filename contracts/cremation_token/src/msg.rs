use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Addr, Binary, Decimal, Uint128};
use cw20::{Expiration, Logo};
use cw20_base::msg::InstantiateMsg as Cw20InstantiateMsg;

use crate::state::{FractionFormat, TaxInfo};

pub use classic_terraswap::asset::AssetInfo;
#[cw_serde]
pub struct MigrateMsg {}

#[cw_serde]
pub enum Dex {
    Terraswap,
    Terraport,
}

#[cw_serde]
pub enum SwapOperation {
    NativeSwap {
        offer_denom: String,
        ask_denom: String,
    },
    TerraSwap {
        offer_asset_info: AssetInfo,
        ask_asset_info: AssetInfo,
    },
    TerraPort {
        offer_asset_info: AssetInfo,
        ask_asset_info: AssetInfo,
    },
    Loop {
        offer_asset_info: AssetInfo,
        ask_asset_info: AssetInfo,
    },
    Astroport {
        offer_asset_info: AssetInfo,
        ask_asset_info: AssetInfo,
    },
}

#[cw_serde]
pub enum RouterExecuteMsg {
    /// Execute multiple BuyOperation
    ExecuteSwapOperations {
        operations: Vec<SwapOperation>,
        minimum_receive: Option<Uint128>,
        to: Option<String>,
        deadline: Option<u64>,
    },
}

#[cw_serde]
pub struct InstantiateMsg {
    pub owner: String,
    pub tax_info: TaxInfo,
    pub cw20_instantiate_msg: Cw20InstantiateMsg,
}

#[cw_serde]
pub enum ExecuteMsg {
    // ======= Extend executes for cremation-coin =======
    SetDexConfigs {
        terraswap_router: String,
        terraswap_pairs: Vec<String>,
        terraport_router: String,
        terraport_pairs: Vec<String>,
    },
    UpdateOwner {
        new_owner: String,
    },
    AddNewPairs {
        dex: Dex,
        pair_addresses: Vec<String>,
    },
    UpdateCollectTaxAddress {
        new_collect_tax_addr: String,
    },
    UpdateTaxInfo {
        buy_tax: Option<FractionFormat>,
        sell_tax: Option<FractionFormat>,
        transfer_tax: Option<FractionFormat>,
    },
    SetTaxFreeAddress {
        address: String,
        tax_free: bool,
    },

    // ======= Existed executes from cw20-base =======
    /// Transfer is a base message to move tokens to another account without triggering actions
    Transfer {
        recipient: String,
        amount: Uint128,
    },
    /// Burn is a base message to destroy tokens forever
    Burn {
        amount: Uint128,
    },
    /// Send is a base message to transfer tokens to a contract and trigger an action
    /// on the receiving contract.
    Send {
        contract: String,
        amount: Uint128,
        msg: Binary,
    },
    /// Only with "approval" extension. Allows spender to access an additional amount tokens
    /// from the owner's (env.sender) account. If expires is Some(), overwrites current allowance
    /// expiration with this one.
    IncreaseAllowance {
        spender: String,
        amount: Uint128,
        expires: Option<Expiration>,
    },
    /// Only with "approval" extension. Lowers the spender's access of tokens
    /// from the owner's (env.sender) account by amount. If expires is Some(), overwrites current
    /// allowance expiration with this one.
    DecreaseAllowance {
        spender: String,
        amount: Uint128,
        expires: Option<Expiration>,
    },
    /// Only with "approval" extension. Transfers amount tokens from owner -> recipient
    /// if `env.sender` has sufficient pre-approval.
    TransferFrom {
        owner: String,
        recipient: String,
        amount: Uint128,
    },
    /// Only with "approval" extension. Sends amount tokens from owner -> contract
    /// if `env.sender` has sufficient pre-approval.
    SendFrom {
        owner: String,
        contract: String,
        amount: Uint128,
        msg: Binary,
    },
    /// Only with "approval" extension. Destroys tokens forever
    BurnFrom {
        owner: String,
        amount: Uint128,
    },
    /// Only with the "mintable" extension. If authorized, creates amount new tokens
    /// and adds to the recipient balance.
    Mint {
        recipient: String,
        amount: Uint128,
    },
    /// Only with the "mintable" extension. The current minter may set
    /// a new minter. Setting the minter to None will remove the
    /// token's minter forever.
    UpdateMinter {
        new_minter: Option<String>,
    },
    /// Only with the "marketing" extension. If authorized, updates marketing metadata.
    /// Setting None/null for any of these will leave it unchanged.
    /// Setting Some("") will clear this field on the contract storage
    UpdateMarketing {
        /// A URL pointing to the project behind this token.
        project: Option<String>,
        /// A longer description of the token and it's utility. Designed for tooltips or such
        description: Option<String>,
        /// The address (if any) who can update this data structure
        marketing: Option<String>,
    },
    /// If set as the "marketing" role on the contract, upload a new URL, SVG, or PNG for the token
    UploadLogo(Logo),
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    // ======= Extended queries from cremation-coin =======
    /// Returns the dex contracts.
    /// - terraswap_router: Terraswap Router contract address
    /// - terraswap_pair: Terraswap Pair contract address
    /// - terraport_router: Terraport Router contract address
    /// - terraport_pair: Terraport Pair contract address
    #[returns(DexConfigsResponse)]
    DexConfigs {},
    #[returns(OwnerResponse)]
    Owner {},
    #[returns(CollectTaxAddressResponse)]
    CollectTaxAddress {},
    /// Returns the current tax info of the contract.
    /// - buy_tax: Tax rate for buy
    /// - sell_tax: Tax rate for sell
    /// - transfer_tax: Tax rate for transfer
    #[returns(TaxInfoResponse)]
    TaxInfo {},
    #[returns(TaxFreeAddressResponse)]
    TaxFreeAddress { address: String },

    // ======= Existed queries from cw20-base =======
    /// Returns the current balance of the given address, 0 if unset.
    #[returns(cw20::BalanceResponse)]
    Balance { address: String },
    /// Returns metadata on the contract - name, decimals, supply, etc.
    #[returns(cw20::TokenInfoResponse)]
    TokenInfo {},
    /// Only with "mintable" extension.
    /// Returns who can mint and the hard cap on maximum tokens after minting.
    #[returns(cw20::MinterResponse)]
    Minter {},
    /// Only with "allowance" extension.
    /// Returns how much spender can use from owner account, 0 if unset.
    #[returns(cw20::AllowanceResponse)]
    Allowance { owner: String, spender: String },
    /// Only with "enumerable" extension (and "allowances")
    /// Returns all allowances this owner has approved. Supports pagination.
    #[returns(cw20::AllAllowancesResponse)]
    AllAllowances {
        owner: String,
        start_after: Option<String>,
        limit: Option<u32>,
    },
    /// Only with "enumerable" extension (and "allowances")
    /// Returns all allowances this spender has been granted. Supports pagination.
    #[returns(cw20::AllSpenderAllowancesResponse)]
    AllSpenderAllowances {
        spender: String,
        start_after: Option<String>,
        limit: Option<u32>,
    },
    /// Only with "enumerable" extension
    /// Returns all accounts that have balances. Supports pagination.
    #[returns(cw20::AllAccountsResponse)]
    AllAccounts {
        start_after: Option<String>,
        limit: Option<u32>,
    },
    /// Only with "marketing" extension
    /// Returns more metadata on the contract to display in the client:
    /// - description, logo, project url, etc.
    #[returns(cw20::MarketingInfoResponse)]
    MarketingInfo {},
    /// Only with "marketing" extension
    /// Downloads the embedded logo data (if stored on chain). Errors if no logo data is stored for this
    /// contract.
    #[returns(cw20::DownloadLogoResponse)]
    DownloadLogo {},
}

#[cw_serde]
pub struct DexConfigsResponse {
    pub terraswap_router: Addr,
    pub terraswap_pairs: Vec<Addr>,
    pub terraport_router: Addr,
    pub terraport_pairs: Vec<Addr>,
}

#[cw_serde]
pub struct TaxInfoResponse {
    pub buy_tax: Decimal,
    pub sell_tax: Decimal,
    pub transfer_tax: Decimal,
}

#[cw_serde]
pub struct OwnerResponse {
    pub owner: Addr,
}

#[cw_serde]
pub struct CollectTaxAddressResponse {
    pub collect_tax_address: Addr,
}

#[cw_serde]
pub struct TaxFreeAddressResponse {
    pub tax_free: bool,
}
