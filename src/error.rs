use cosmwasm_std::{
    CheckedFromRatioError, CheckedMultiplyFractionError, ConversionOverflowError,
    Decimal256RangeExceeded, DivideByZeroError, OverflowError, StdError, Uint256,
};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("Unauthorized")]
    Unauthorized {},

    #[error("Payment is not the same as the price {price}")]
    IncorrectPayment { price: Uint256 },

    #[error("The reply ID is unrecognized")]
    UnrecognizedReply {},

    #[error("The NFT is not tradable.")]
    NonTradeable {},

    #[error("User is not the NFT owner")]
    NotNftOwner {},

    #[error("Type Sent is Not Supported")]
    TypeNotSupported {},

    #[error("{0}")]
    DivideByZeroError(#[from] DivideByZeroError),

    #[error("{0}")]
    OverflowError(#[from] OverflowError),

    #[error("{0}")]
    CheckedFromRatioError(#[from] CheckedFromRatioError),

    #[error("{0}")]
    Decimal256RangeExceeded(#[from] Decimal256RangeExceeded),

    #[error("{0}")]
    ConversionOverflowError(#[from] ConversionOverflowError),

    #[error("{0}")]
    CheckedMultiplyFractionError(#[from] CheckedMultiplyFractionError),
}
