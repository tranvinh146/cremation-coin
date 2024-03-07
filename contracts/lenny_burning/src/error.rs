use cosmwasm_std::StdError;
use thiserror::Error;

#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
    #[error("{0}")]
    StdError(#[from] StdError),
    #[error("Unauthorized")]
    Unauthorized {},
    #[error("Ratio must be less than 1")]
    RatioMustBeLessThanOne {},
    #[error("Already exists")]
    AlreadyExists {},
    #[error("Zero amount")]
    ZeroAmount {},
    #[error("Zero ratio")]
    ZeroRatio {},
    #[error("Exceed burn limit")]
    ExceedBurnLimit {},
}
