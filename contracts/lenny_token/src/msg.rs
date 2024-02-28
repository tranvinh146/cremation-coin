use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Binary, Uint128};
use cremation_token::{
    msg::Dex,
    state::{FractionFormat, TaxInfo},
};
use cw20::{Expiration, Logo};
use cw20_base::msg::InstantiateMsg as Cw20InstantiateMsg;

#[cw_serde]
pub struct MigrateMsg {}

#[cw_serde]
pub struct InstantiateMsg {
    pub owner: String,
    pub tax_info: TaxInfo,
    pub swap_tax_to_token: String,
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
    RemovePair {
        dex: Dex,
        pair_address: String,
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

pub use cremation_token::msg::QueryMsg;
