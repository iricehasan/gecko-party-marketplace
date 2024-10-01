#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    coins, from_json, to_json_binary, Addr, BankMsg, Binary, Coin, CosmosMsg, Deps, DepsMut, Empty,
    Env, MessageInfo, Order, Reply, Response, StdResult, SubMsg, Uint128, Uint256, WasmMsg,
};
use cw2::set_contract_version;
use cw20::{Cw20Contract, Cw20ExecuteMsg, Cw20ReceiveMsg};
use cw721::{Cw721ExecuteMsg, Cw721QueryMsg, Cw721ReceiveMsg, OwnerOfResponse};

use crate::error::ContractError;
use crate::msg::{ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg, ReceiveMsg, ReceiveNftMsg};
use crate::state::{
    CoinType, Config, Listing, Offer, Trade, CONFIG, LISTINGS, LISTING_COUNTER, OFFERS, TRADES,
};

pub const CONTRACT_NAME: &str = "gecko-party-marketplace";
pub const CONTRACT_VERSION: &str = "0.1.0";

pub const LISTING_REPLY: u64 = 1;
pub const TRADE_REPLY: u64 = 2;
pub const OFFER_REPLY: u64 = 3;

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    let config = Config {
        cw721_address: deps.api.addr_validate(&msg.cw721_address)?,
        cw20_address: deps.api.addr_validate(&msg.cw20_address)?,
    };

    CONFIG.save(deps.storage, &config)?;
    LISTING_COUNTER.save(deps.storage, &0u128)?;

    Ok(Response::new()
        .add_attribute("action", "instantiate")
        .add_attribute("NFT", config.cw721_address)
        .add_attribute("Cw20 Token", config.cw20_address))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::Buy { id } => execute_buy(deps, info, id),
        ExecuteMsg::Offer {
            target,
            offered_price,
        } => execute_offer(deps, info, env, target, offered_price),
        ExecuteMsg::CancelOffer { id } => execute_cancel_offer(deps, info, id),
        ExecuteMsg::AcceptOffer { id, offerer } => {
            execute_accept_offer(deps, info, env, id, offerer)
        }
        ExecuteMsg::RejectOffer { id, offerer } => execute_reject_offer(deps, info, id, offerer),
        ExecuteMsg::AcceptTrade { id, trader } => execute_accept_trade(deps, info, id, trader),
        ExecuteMsg::CancelTrade { id } => execute_cancel_trade(deps, info, id),
        ExecuteMsg::CancelListing { id } => execute_cancel_listing(deps, info, id),
        ExecuteMsg::Receive(msg) => execute_receive(deps, env, info, msg),
        ExecuteMsg::ReceiveNft(msg) => execute_receive_nft(deps, env, info, msg),
    }
}

pub fn execute_buy(
    deps: DepsMut,
    info: MessageInfo,
    id: String,
) -> Result<Response, ContractError> {
    let listing = LISTINGS.load(deps.storage, id.clone())?;

    if Uint256::from_uint128(info.funds[0].amount) != listing.price {
        return Err(ContractError::IncorrectPayment {
            price: listing.price,
        });
    }

    let config = CONFIG.load(deps.storage)?;

    let submsg = SubMsg::reply_on_success(
        WasmMsg::Execute {
            contract_addr: config.cw721_address.to_string(),
            msg: to_json_binary(&Cw721ExecuteMsg::TransferNft {
                recipient: info.sender.to_string().clone(),
                token_id: listing.nft_id.clone(),
            })?,
            funds: vec![],
        },
        LISTING_REPLY,
    );

    let mut res = Response::new()
        .add_attribute("action", "receive_buy")
        .add_attribute("NFT", listing.nft_id)
        .add_attribute("seller", listing.owner.clone().into_string())
        .add_attribute("buyer", info.sender.to_string())
        .add_submessage(submsg);

    let payment = CosmosMsg::Bank(BankMsg::Send {
        to_address: listing.owner.into_string().clone(),
        amount: coins(info.funds[0].amount.into(), "uxion"),
    });

    LISTINGS.remove(deps.storage, id);
    let _ = LISTING_COUNTER.update(deps.storage, |counter: u128| -> StdResult<u128> {
        Ok(counter.checked_sub(1u128).unwrap())
    });
    res = res.add_message(payment);

    Ok(res)
}

pub fn execute_offer(
    deps: DepsMut,
    info: MessageInfo,
    env: Env,
    asked_id: String,
    amount_offered: Uint256,
) -> Result<Response, ContractError> {
    // check funds
    if Uint256::from_uint128(info.funds[0].amount) != amount_offered {
        return Err(ContractError::IncorrectPayment {
            price: amount_offered,
        });
    }

    let payment: CosmosMsg = CosmosMsg::Bank(BankMsg::Send {
        to_address: env.contract.address.to_string().clone(),
        amount: vec![Coin {
            denom: "uxion".to_string(),
            amount: amount_offered.try_into()?,
        }],
    });

    let new_offer = Offer {
        asked_id: asked_id.clone(),
        offerer: info.sender,
        amount_offered: amount_offered.clone(),
        amount_type: CoinType::Native,
    };

    OFFERS.save(
        deps.storage,
        (asked_id.clone(), new_offer.offerer.to_string()),
        &new_offer,
    )?;

    Ok(Response::new()
        .add_attribute("action", "offer")
        .add_attribute("NFT", asked_id)
        .add_message(payment))
}

pub fn execute_accept_offer(
    deps: DepsMut,
    info: MessageInfo,
    _env: Env,
    asked_id: String,
    offerer: String,
) -> Result<Response, ContractError> {
    let offer = OFFERS.load(deps.storage, (asked_id.clone(), offerer))?;
    let listing = LISTINGS.load(deps.storage, asked_id.clone())?;
    let config = CONFIG.load(deps.storage)?;

    if listing.owner != info.sender {
        return Err(ContractError::Unauthorized {});
    }

    // payment from the contract
    let payment: CosmosMsg = match offer.amount_type {
        CoinType::Cw20 => {
            let cw20 = Cw20Contract(config.cw20_address);

            // send payment to the contract
            cw20.call(Cw20ExecuteMsg::Transfer {
                recipient: info.sender.to_string().clone(),
                amount: offer.amount_offered.try_into()?,
            })?
        }
        CoinType::Native => CosmosMsg::Bank(BankMsg::Send {
            to_address: info.sender.to_string().clone(),
            amount: vec![Coin {
                denom: "uxion".to_string(),
                amount: offer.amount_offered.try_into()?,
            }],
        }),
    };

    // Asked
    let submsgs: Vec<SubMsg> = vec![SubMsg::reply_on_success(
        WasmMsg::Execute {
            contract_addr: config.cw721_address.to_string(),
            msg: to_json_binary(&Cw721ExecuteMsg::TransferNft {
                recipient: offer.offerer.to_string(),
                token_id: offer.asked_id.clone(),
            })?,
            funds: vec![],
        },
        OFFER_REPLY,
    )];

    OFFERS.remove(
        deps.storage,
        (offer.asked_id.clone(), offer.offerer.to_string()),
    );

    LISTINGS.remove(deps.storage, offer.asked_id.clone());
    let _ = LISTING_COUNTER.update(deps.storage, |counter: u128| -> StdResult<u128> {
        Ok(counter.checked_sub(1u128).unwrap())
    });

    Ok(Response::new()
        .add_attribute("action", "accept offer")
        .add_attribute("NFT", asked_id)
        .add_message(payment)
        .add_submessages(submsgs))
}

pub fn execute_cancel_offer(
    deps: DepsMut,
    info: MessageInfo,
    asked_id: String,
) -> Result<Response, ContractError> {
    let offer = OFFERS.load(deps.storage, (asked_id.clone(), info.sender.to_string()))?;
    let config = CONFIG.load(deps.storage)?;

    if offer.offerer != info.sender {
        return Err(ContractError::Unauthorized {});
    }

    // payment from the contract
    let payment: CosmosMsg = match offer.amount_type {
        CoinType::Cw20 => {
            let cw20 = Cw20Contract(config.cw20_address);

            // send payment to the contract
            cw20.call(Cw20ExecuteMsg::Transfer {
                recipient: offer.offerer.to_string().clone(),
                amount: offer.amount_offered.try_into()?,
            })?
        }
        CoinType::Native => CosmosMsg::Bank(BankMsg::Send {
            to_address: offer.offerer.to_string().clone(),
            amount: vec![Coin {
                denom: "uxion".to_string(),
                amount: offer.amount_offered.try_into()?,
            }],
        }),
    };

    OFFERS.remove(
        deps.storage,
        (offer.asked_id.clone(), offer.offerer.to_string()),
    );

    Ok(Response::new()
        .add_attribute("action", "cancel offer")
        .add_attribute("NFT", asked_id)
        .add_message(payment))
}

pub fn execute_reject_offer(
    deps: DepsMut,
    info: MessageInfo,
    asked_id: String,
    offerer: String,
) -> Result<Response, ContractError> {
    let offer = OFFERS.load(deps.storage, (asked_id.clone(), offerer.to_string()))?;
    let listing = LISTINGS.load(deps.storage, asked_id.clone())?;

    let config = CONFIG.load(deps.storage)?;

    if listing.owner != info.sender {
        return Err(ContractError::Unauthorized {});
    }

    // payment from the contract
    let payment: CosmosMsg = match offer.amount_type {
        CoinType::Cw20 => {
            let cw20 = Cw20Contract(config.cw20_address);

            // send payment to the contract
            cw20.call(Cw20ExecuteMsg::Transfer {
                recipient: offer.offerer.to_string().clone(),
                amount: offer.amount_offered.try_into()?,
            })?
        }
        CoinType::Native => CosmosMsg::Bank(BankMsg::Send {
            to_address: offer.offerer.to_string().clone(),
            amount: vec![Coin {
                denom: "uxion".to_string(),
                amount: offer.amount_offered.try_into()?,
            }],
        }),
    };

    OFFERS.remove(
        deps.storage,
        (offer.asked_id.clone(), offer.offerer.to_string()),
    );

    Ok(Response::new()
        .add_attribute("action", "reject offer")
        .add_attribute("NFT", asked_id)
        .add_message(payment))
}

pub fn execute_accept_trade(
    deps: DepsMut,
    info: MessageInfo,
    asked_id: String,
    trader: String,
) -> Result<Response, ContractError> {
    let trade = TRADES.load(deps.storage, (asked_id.clone(), trader))?;
    let listing = LISTINGS.load(deps.storage, asked_id.clone())?;

    if listing.owner != info.sender {
        return Err(ContractError::Unauthorized {});
    }

    let config = CONFIG.load(deps.storage)?;

    // Asked
    let mut submsgs: Vec<SubMsg> = vec![SubMsg::reply_on_success(
        WasmMsg::Execute {
            contract_addr: config.cw721_address.to_string(),
            msg: to_json_binary(&Cw721ExecuteMsg::TransferNft {
                recipient: trade.trader.to_string(),
                token_id: trade.asked_id.clone(),
            })?,
            funds: vec![],
        },
        TRADE_REPLY,
    )];

    // Offered
    submsgs.push(SubMsg::reply_on_success(
        WasmMsg::Execute {
            contract_addr: config.cw721_address.to_string(),
            msg: to_json_binary(&Cw721ExecuteMsg::TransferNft {
                recipient: listing.owner.to_string(),
                token_id: trade.to_trade_id.clone(),
            })?,
            funds: vec![],
        },
        TRADE_REPLY,
    ));

    TRADES.remove(
        deps.storage,
        (trade.asked_id.clone(), trade.trader.to_string()),
    );

    LISTINGS.remove(deps.storage, trade.asked_id.clone());
    let _ = LISTING_COUNTER.update(deps.storage, |counter: u128| -> StdResult<u128> {
        Ok(counter.checked_sub(1u128).unwrap())
    });

    if LISTINGS
        .may_load(deps.storage, trade.to_trade_id.clone())?
        .is_some()
    {
        LISTINGS.remove(deps.storage, trade.to_trade_id.clone());
        let _ = LISTING_COUNTER.update(deps.storage, |counter: u128| -> StdResult<u128> {
            Ok(counter.checked_sub(1u128).unwrap())
        });
    }

    Ok(Response::new()
        .add_attribute("action", "NFT traded")
        .add_attribute("NFT asked", trade.asked_id)
        .add_attribute("NFT offered", trade.to_trade_id)
        .add_submessages(submsgs))
}

pub fn execute_cancel_listing(
    deps: DepsMut,
    info: MessageInfo,
    id: String,
) -> Result<Response, ContractError> {
    let listing = LISTINGS.load(deps.storage, id.clone())?;

    if listing.owner != info.sender {
        return Err(ContractError::Unauthorized {});
    }

    let config = CONFIG.load(deps.storage)?;

    let msg = CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: config.cw721_address.to_string(),
        msg: to_json_binary(&Cw721ExecuteMsg::TransferNft {
            recipient: listing.owner.to_string(),
            token_id: listing.nft_id.clone(),
        })?,
        funds: vec![],
    });

    LISTINGS.remove(deps.storage, id.clone());

    let _ = LISTING_COUNTER.update(deps.storage, |counter: u128| -> StdResult<u128> {
        Ok(counter.checked_sub(1u128).unwrap())
    });

    Ok(Response::new()
        .add_attribute("action", "cancel listing")
        .add_attribute("NFT", listing.nft_id)
        .add_message(msg))
}

pub fn execute_cancel_trade(
    deps: DepsMut,
    info: MessageInfo,
    asked_id: String,
) -> Result<Response, ContractError> {
    let target = TRADES.load(deps.storage, (asked_id.clone(), info.sender.to_string()))?;

    if target.trader != info.sender {
        return Err(ContractError::Unauthorized {});
    }

    let config = CONFIG.load(deps.storage)?;

    let msg = CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: config.cw721_address.to_string(),
        msg: to_json_binary(&Cw721ExecuteMsg::TransferNft {
            recipient: target.trader.to_string(),
            token_id: target.to_trade_id,
        })?,
        funds: vec![],
    });

    TRADES.remove(deps.storage, (target.asked_id, target.trader.to_string()));

    Ok(Response::new()
        .add_attribute("action", "cancel trade")
        .add_attribute("NFT", asked_id)
        .add_message(msg))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn reply(_deps: DepsMut, _env: Env, reply: Reply) -> Result<Response, ContractError> {
    match reply.id {
        LISTING_REPLY => Ok(Response::new().add_attribute("Operation", "NFT Listing")),
        TRADE_REPLY => Ok(Response::new().add_attribute("Operation", "NFT Trade")),
        OFFER_REPLY => Ok(Response::new().add_attribute("Operation", "NFT Price Offer")),
        _ => Err(ContractError::UnrecognizedReply {}),
    }
}

pub fn execute_receive(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    cw20_receive_msg: Cw20ReceiveMsg,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    if config.cw20_address != info.sender {
        return Err(ContractError::Unauthorized {});
    }

    let msg: ReceiveMsg = from_json(&cw20_receive_msg.msg)?;
    match msg {
        ReceiveMsg::Buy { id } => receive_buy(
            deps,
            id,
            cw20_receive_msg.sender,
            cw20_receive_msg.amount,
            info.sender,
        ),
        ReceiveMsg::Offer {
            target,
            offered_price,
        } => receive_offer(
            deps,
            env,
            target,
            offered_price,
            cw20_receive_msg.sender,
            cw20_receive_msg.amount,
            info.sender,
        ),
    }
}

pub fn receive_buy(
    deps: DepsMut,
    id: String,
    sender: String,
    amount: Uint128,
    cw20_address: Addr,
) -> Result<Response, ContractError> {
    let listing = LISTINGS.load(deps.storage, id.clone())?;

    if Uint256::from_uint128(amount) != listing.price {
        return Err(ContractError::IncorrectPayment {
            price: listing.price,
        });
    }

    let config = CONFIG.load(deps.storage)?;

    let submsg = SubMsg::reply_on_success(
        WasmMsg::Execute {
            contract_addr: config.cw721_address.to_string(),
            msg: to_json_binary(&Cw721ExecuteMsg::TransferNft {
                recipient: sender.clone(),
                token_id: listing.nft_id.clone(),
            })?,
            funds: vec![],
        },
        LISTING_REPLY,
    );

    let mut res = Response::new()
        .add_attribute("action", "receive_buy")
        .add_attribute("NFT", listing.nft_id)
        .add_attribute("seller", listing.owner.clone().into_string())
        .add_attribute("buyer", sender)
        .add_submessage(submsg);

    let cw20 = Cw20Contract(cw20_address);

    let payment = cw20.call(Cw20ExecuteMsg::Transfer {
        recipient: listing.owner.into_string().clone(),
        amount,
    })?;

    LISTINGS.remove(deps.storage, id);
    let _ = LISTING_COUNTER.update(deps.storage, |counter: u128| -> StdResult<u128> {
        Ok(counter.checked_sub(1u128).unwrap())
    });
    res = res.add_message(payment);

    Ok(res)
}

pub fn receive_offer(
    deps: DepsMut,
    env: Env,
    asked_id: String,
    amount_offered: Uint256,
    sender: String,
    amount: Uint128,
    cw20_address: Addr,
) -> Result<Response, ContractError> {
    // check funds
    if Uint256::from_uint128(amount) != amount_offered {
        return Err(ContractError::IncorrectPayment {
            price: amount_offered,
        });
    }

    let sender_addr = deps.api.addr_validate(&sender)?;
    let new_offer = Offer {
        asked_id: asked_id.clone(),
        offerer: sender_addr.clone(),
        amount_offered: amount_offered.clone(),
        amount_type: CoinType::Cw20,
    };

    OFFERS.save(
        deps.storage,
        (asked_id.clone(), new_offer.offerer.to_string()),
        &new_offer,
    )?;

    let cw20 = Cw20Contract(cw20_address);

    // send payment to the contract
    let payment = cw20.call(Cw20ExecuteMsg::Transfer {
        recipient: env.contract.address.into_string().clone(),
        amount,
    })?;

    Ok(Response::new().add_message(payment))
}
pub fn execute_receive_nft(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    receive_msg: Cw721ReceiveMsg,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    if config.cw721_address != info.sender {
        return Err(ContractError::Unauthorized {});
    };

    // info.sender is the NFT contract Address
    let sender = receive_msg.sender.clone();

    let msg: ReceiveNftMsg = from_json(&receive_msg.msg)?;
    match msg {
        ReceiveNftMsg::NewListing { price, tradeable } => {
            receive_new_listing(deps, sender, receive_msg.token_id, price, tradeable)
        }
        ReceiveNftMsg::NewTrade { target } => {
            receive_new_trade(deps, sender, receive_msg.token_id, target)
        }
    }
}

pub fn receive_new_listing(
    deps: DepsMut,
    sender: String,
    id: String,
    price: Uint256,
    tradeable: bool,
) -> Result<Response, ContractError> {
    let owner = deps.api.addr_validate(&sender)?;

    let new_listing = Listing {
        nft_id: id.clone(),
        price,
        owner,
        tradeable,
    };

    LISTINGS.save(deps.storage, id.clone(), &new_listing)?;
    let _ = LISTING_COUNTER.update(deps.storage, |counter: u128| -> StdResult<u128> {
        Ok(counter.checked_add(1u128).unwrap())
    });

    let res = Response::new()
        .add_attribute("action", "new listing")
        .add_attribute("NFT", id)
        .add_attribute("owner", sender);

    Ok(res)
}

pub fn receive_new_trade(
    deps: DepsMut,
    sender: String,
    offered_id: String,
    asked_id: String,
) -> Result<Response, ContractError> {
    let trader = deps.api.addr_validate(&sender)?;
    let config = CONFIG.load(deps.storage)?;

    let nft_owner: OwnerOfResponse = deps.querier.query_wasm_smart(
        config.cw721_address.to_string(),
        &Cw721QueryMsg::OwnerOf {
            token_id: offered_id.clone(),
            include_expired: Some(false),
        },
    )?;

    if nft_owner.owner != trader.clone() {
        return Err(ContractError::Unauthorized {});
    }

    let listing = LISTINGS.load(deps.storage, asked_id.clone())?;

    if !listing.tradeable {
        return Err(ContractError::NonTradeable {});
    }

    let new_trade = Trade {
        asked_id: asked_id.clone(),
        to_trade_id: offered_id.clone(),
        trader,
    };

    TRADES.save(
        deps.storage,
        (asked_id.clone(), new_trade.trader.to_string()),
        &new_trade,
    )?;

    Ok(Response::new()
        .add_attribute("action", "new trade")
        .add_attribute("Asked NFT", asked_id)
        .add_attribute("Offered NFT", offered_id))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(_deps: DepsMut, _env: Env, _msg: Empty) -> Result<Response, ContractError> {
    Ok(Response::default())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::GetListing { id } => to_json_binary(&get_listing(deps, id)?),
        QueryMsg::GetListingsBySeller {
            seller,
            from_index,
            limit,
        } => to_json_binary(&get_listings_seller(deps, seller, from_index, limit)?),
        QueryMsg::GetAllListings { from_index, limit } => {
            to_json_binary(&get_all_listings(deps, from_index, limit)?)
        }
        QueryMsg::GetListingCount {} => to_json_binary(&get_listing_count(deps)?),
        QueryMsg::GetTrade { id, trader } => to_json_binary(&get_trade(deps, id, trader)?),
        QueryMsg::GetTradesByAddress {
            address,
            from_index,
            limit,
        } => to_json_binary(&get_trades_address(deps, address, from_index, limit)?),
        QueryMsg::GetTradesById {
            id,
            from_index,
            limit,
        } => to_json_binary(&get_trades_id(deps, id, from_index, limit)?),
        QueryMsg::GetAllTrades { from_index, limit } => {
            to_json_binary(&get_all_trades(deps, from_index, limit)?)
        }
        QueryMsg::GetOffer { id, offerer } => to_json_binary(&get_offer(deps, id, offerer)?),
        QueryMsg::GetOffersByAddress {
            address,
            from_index,
            limit,
        } => to_json_binary(&get_offers_address(deps, address, from_index, limit)?),
        QueryMsg::GetOffersById {
            id,
            from_index,
            limit,
        } => to_json_binary(&get_offers_id(deps, id, from_index, limit)?),
        QueryMsg::GetAllOffers { from_index, limit } => {
            to_json_binary(&get_all_offers(deps, from_index, limit)?)
        }
    }
}

pub fn get_listing_count(deps: Deps) -> StdResult<u128> {
    Ok(LISTING_COUNTER.load(deps.storage)?)
}

pub fn get_listing(deps: Deps, id: String) -> StdResult<Listing> {
    let listing = LISTINGS.load(deps.storage, id)?;
    Ok(listing)
}

pub fn get_trade(deps: Deps, id: String, trader: String) -> StdResult<Trade> {
    let trade = TRADES.load(deps.storage, (id, trader))?;
    Ok(trade)
}

pub fn get_offer(deps: Deps, id: String, offerer: String) -> StdResult<Offer> {
    let offer = OFFERS.load(deps.storage, (id, offerer))?;
    Ok(offer)
}

pub fn get_listings_seller(
    deps: Deps,
    seller: String,
    from_index: Option<u64>,
    limit: Option<u64>,
) -> StdResult<Vec<Listing>> {
    let from_index = from_index.unwrap_or(0);
    let limit = limit.unwrap_or(10);

    let listings: StdResult<Vec<Listing>> = LISTINGS
        .range(deps.storage, None, None, Order::Ascending)
        .skip(from_index as usize)
        .take(limit as usize)
        .filter(|item| item.as_ref().unwrap().1.owner == seller)
        .map(|item| item.map(|(_, listing)| listing))
        .collect();
    listings
}

pub fn get_all_listings(
    deps: Deps,
    from_index: Option<u64>,
    limit: Option<u64>,
) -> StdResult<Vec<Listing>> {
    let from_index = from_index.unwrap_or(0);
    let limit = limit.unwrap_or(10);

    let listings: StdResult<Vec<Listing>> = LISTINGS
        .range(deps.storage, None, None, Order::Ascending)
        .skip(from_index as usize)
        .take(limit as usize)
        .map(|item| item.map(|(_, listing)| listing))
        .collect();
    listings
}

pub fn get_trades_address(
    deps: Deps,
    address: String,
    from_index: Option<u64>,
    limit: Option<u64>,
) -> StdResult<Vec<Trade>> {
    let from_index = from_index.unwrap_or(0);
    let limit = limit.unwrap_or(10);

    let trades: StdResult<Vec<Trade>> = TRADES
        .range(deps.storage, None, None, Order::Ascending)
        .skip(from_index as usize)
        .take(limit as usize)
        .filter(|item| item.as_ref().unwrap().1.trader == address)
        .map(|item| item.map(|(_, trade)| trade))
        .collect();
    trades
}

pub fn get_trades_id(
    deps: Deps,
    id: String,
    from_index: Option<u64>,
    limit: Option<u64>,
) -> StdResult<Vec<Trade>> {
    let from_index = from_index.unwrap_or(0);
    let limit = limit.unwrap_or(10);

    let trades: StdResult<Vec<Trade>> = TRADES
        .range(deps.storage, None, None, Order::Ascending)
        .skip(from_index as usize)
        .take(limit as usize)
        .filter(|item| item.as_ref().unwrap().1.asked_id == id)
        .map(|item| item.map(|(_, trade)| trade))
        .collect();
    trades
}

pub fn get_all_trades(
    deps: Deps,
    from_index: Option<u64>,
    limit: Option<u64>,
) -> StdResult<Vec<Trade>> {
    let from_index = from_index.unwrap_or(0);
    let limit = limit.unwrap_or(10);

    let trades: StdResult<Vec<Trade>> = TRADES
        .range(deps.storage, None, None, Order::Ascending)
        .skip(from_index as usize)
        .take(limit as usize)
        .map(|item| item.map(|(_, trade)| trade))
        .collect();
    trades
}

pub fn get_offers_address(
    deps: Deps,
    address: String,
    from_index: Option<u64>,
    limit: Option<u64>,
) -> StdResult<Vec<Offer>> {
    let from_index = from_index.unwrap_or(0);
    let limit = limit.unwrap_or(10);

    let offers: StdResult<Vec<Offer>> = OFFERS
        .range(deps.storage, None, None, Order::Ascending)
        .skip(from_index as usize)
        .take(limit as usize)
        .filter(|item| item.as_ref().unwrap().1.offerer == address)
        .map(|item| item.map(|(_, offer)| offer))
        .collect();
    offers
}

pub fn get_offers_id(
    deps: Deps,
    id: String,
    from_index: Option<u64>,
    limit: Option<u64>,
) -> StdResult<Vec<Offer>> {
    let from_index = from_index.unwrap_or(0);
    let limit = limit.unwrap_or(10);

    let offers: StdResult<Vec<Offer>> = OFFERS
        .range(deps.storage, None, None, Order::Ascending)
        .skip(from_index as usize)
        .take(limit as usize)
        .filter(|item| item.as_ref().unwrap().1.asked_id == id)
        .map(|item| item.map(|(_, offer)| offer))
        .collect();
    offers
}

pub fn get_all_offers(
    deps: Deps,
    from_index: Option<u64>,
    limit: Option<u64>,
) -> StdResult<Vec<Offer>> {
    let from_index = from_index.unwrap_or(0);
    let limit = limit.unwrap_or(10);

    let offers: StdResult<Vec<Offer>> = OFFERS
        .range(deps.storage, None, None, Order::Ascending)
        .skip(from_index as usize)
        .take(limit as usize)
        .map(|item| item.map(|(_, offer)| offer))
        .collect();
    offers
}
