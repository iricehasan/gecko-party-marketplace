use crate::state::{Listing, Offer, Trade};
use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::Uint256;
use cw20::Cw20ReceiveMsg;
use cw721::Cw721ReceiveMsg;

#[cw_serde]
pub struct InstantiateMsg {
    pub cw721_address: String,
    pub cw20_address: String,
}

#[cw_serde]
pub enum ExecuteMsg {
    Buy {
        id: String,
    },
    Offer {
        target: String, // target id
        offered_price: Uint256,
    },
    AcceptOffer {
        id: String,
        offerer: String,
    },
    CancelOffer {
        id: String,
    },
    RejectOffer {
        id: String,
        offerer: String,
    },
    AcceptTrade {
        id: String,
        trader: String,
    },
    CancelTrade {
        id: String,
    },
    CancelListing {
        id: String,
    },
    Receive(Cw20ReceiveMsg),
    ReceiveNft(Cw721ReceiveMsg),
}

#[cw_serde]
pub enum ReceiveMsg {
    Buy {
        id: String,
    },
    Offer {
        target: String, // target id
        offered_price: Uint256,
    },
}

#[cw_serde]
pub enum ReceiveNftMsg {
    NewListing { price: Uint256, tradeable: bool },
    NewTrade { target: String },
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(Listing)]
    GetListing { id: String },
    #[returns(Vec<Listing>)]
    GetListingsBySeller {
        seller: String,
        from_index: Option<u64>,
        limit: Option<u64>,
    },
    #[returns(Vec<Listing>)]
    GetAllListings {
        from_index: Option<u64>,
        limit: Option<u64>,
    },
    #[returns(u128)]
    GetListingCount {},
    #[returns(Trade)]
    GetTrade { id: String, trader: String },
    #[returns(Vec<Trade>)]
    GetTradesByAddress {
        address: String,
        from_index: Option<u64>,
        limit: Option<u64>,
    },
    #[returns(Vec<Trade>)]
    GetTradesById {
        id: String,
        from_index: Option<u64>,
        limit: Option<u64>,
    },
    #[returns(Vec<Trade>)]
    GetAllTrades {
        from_index: Option<u64>,
        limit: Option<u64>,
    },
    #[returns(Offer)]
    GetOffer { id: String, offerer: String },
    #[returns(Vec<Offer>)]
    GetOffersByAddress {
        address: String,
        from_index: Option<u64>,
        limit: Option<u64>,
    },
    #[returns(Vec<Offer>)]
    GetOffersById {
        id: String,
        from_index: Option<u64>,
        limit: Option<u64>,
    },
    #[returns(Vec<Offer>)]
    GetAllOffers {
        from_index: Option<u64>,
        limit: Option<u64>,
    },
}

#[cw_serde]
pub struct MigrateMsg {}
