
# Marketplace 


## 1. Listing & Cancel Listing

NFT Owner sends the nft with send_nft message (encoded) to the marketplace contract by giving a price input

```typescript
import { toBase64, toUtf8 } from "@cosmjs/encoding"
const listEncodedMsg = toBase64(toUtf8(JSON.stringify({
    new_listing: {price: "40000", tradeable: true},
 })))


let data = await client.execute(
    senderAddress,
    nftcontractAddress,
    {   send_nft: {
        contract: marketPlaceContractAddress, 
        token_id: tokenId.toString(), 
        msg: listEncodedMsg}
    },  "auto",
  )
```

NFT owner can cancel the listing and the contract transfers the NFT back to the owner
```typescript
let data = await client.execute(
    senderAddress,
    marketPlaceContractAddress,
    {   cancel_listing: {
        id: tokenId.toString()
     }
    },  "auto",
  )

```
## 2. Buy

Another user can directly buy the NFT by giving an id input and either

a. sends the native funds

```typescript
let data = await client.execute(
    senderAddress,
    marketPlaceContractAddress,
    {   buy: {
        id: tokenId.toString()
        }
    },  "auto","",
    [{amount: "50000", denom: "uxion"}]
  )
```

b. sends cw20 by send message

```typescript
import { toBase64, toUtf8 } from "@cosmjs/encoding"
const buyEncodedMsg = toBase64(toUtf8(JSON.stringify({
    buy: {id: tokenId.toString()},
 })))

let data = await client.execute(
    senderAddress,
    cw20contractAddress,
    {send : {
        contract: marketPlaceContractAddress,
        amount: "50000",
        msg: buyEncodedMsg,
    }},  "auto","",
  )
```



## 3. Offer

An user offers a price they want by sending it to the contract by giving target (token_id) and offered_price input. They can either

a. sends the native token by giving a target and offered_price input and also including the funds.

```typescript
let data = await client.execute(
    senderAddress,
    marketPlaceContractAddress,
    {   offer: {
        target: tokenId.toString(), 
        offered_price: "50000" }
    },  "auto","",
    [{amount: "50000", denom: "uxion"}]
  )
```
b. sends the cw20 by send function with an encoded msg

```typescript
import { toBase64, toUtf8 } from "@cosmjs/encoding"
const offerEncodedMsg = toBase64(toUtf8(JSON.stringify({
    offer: {target: tokenId.toString(), offered_price: "30000" },
 })))


let data = await client.execute(
    senderAddress,
    cw20contractAddress,
    {send : {
        contract: marketPlaceContractAddress,
        amount: "30000",
        msg: offerEncodedMsg,
    }},  "auto","",
  )
```

The offerer can cancel their offer with
```typescript
let data = await client.execute(
    senderAddress,
    marketPlaceContractAddress,
    {   cancel_offer: {
        id: tokenId.toString()
     }
    },  "auto",
  )
```

And, the listing owner can accept offers by giving id and offerer address as input

```typescript
let data = await client.execute(
    senderAddress,
    marketPlaceContractAddress,
    {   accept_offer: {
        id: tokenId.toString(), 
        offerer: offererAddress }
    },  "auto",
  )
```

or reject them 

```typescript
let data = await client.execute(
    senderAddress,
    marketPlaceContractAddress,
    {   reject_offer: {
        id: tokenId.toString(), 
        offerer: offererAddress }
    },  "auto",
  )
```

## 4. Trade

An NFT owner can send a trade request to a listed NFT by sending their NFT to the marketplace contract by giving target (asked NFT id) and offered ( NFT id they already own) as input

```typescript
import { toBase64, toUtf8 } from "@cosmjs/encoding"
const tradeEncodedMsg = toBase64(toUtf8(JSON.stringify({
    new_trade: {target: askedTokenId.toString()},
 })))


let data = await client.execute(
    senderAddress,
    nftcontractAddress,
    {   send_nft: {
        contract: marketPlaceContractAddress, 
        token_id: tokenId.toString(), 
        msg: tradeEncodedMsg}
    },  "auto",
  )
```

And, they can cancel their trade request by giving asked NFT id as input with 

```typescript
let data = await client.execute(
    senderAddress,
    marketPlaceContractAddress,
    {   cancel_trade: {
        id: tokenId.toString()}
    },  "auto",
  )
```

The listing owner that receives the request can accept the trade by giving asked nft_id and trader address as input
```typescript
let data = await client.execute(
    senderAddress,
    marketPlaceContractAddress,
    {   accept_trade: {
        id: tokenId.toString(), 
        trader: traderAddress }
    },  "auto",
  )
```


## 5. Queries

Listing queries

```typescript
let data = await client.queryContractSmart(
    marketPlaceContractAddress,
    {  get_listing:  {
        id: tokenId.toString(), }
    },
  )

  console.log(
    "Get listing for token id: ",
    data
  )


let data2 = await client.queryContractSmart(
    marketPlaceContractAddress,
    {  get_listings_by_seller:  {
        seller: senderAddress, }
    },
  )


  console.log(
    "Get listings for seller: ",
    data2
  )


  let data3 = await client.queryContractSmart(
    marketPlaceContractAddress,
    {  get_all_listings:  {}
    },
  )

  console.log(
    "Get All Listings: ",
    data3
  )

```

Offer queries

```typescript
let data = await client.queryContractSmart(
    marketPlaceContractAddress,
    {  get_offer:  {
        id: tokenId.toString(),
        offerer: offererAddress }
    },
  )

  console.log(
    "Get offers for token id and offerer address:  ",
    data
  )


let data2 = await client.queryContractSmart(
    marketPlaceContractAddress,
    {  get_offers_by_address:  {
        address: offererAddress, }
    },
  )

  console.log(
    "Get Offers for address: ",
    data2
  )

  let data3 = await client.queryContractSmart(
    marketPlaceContractAddress,
    {  get_offers_by_id:  {
        id: tokenId.toString(), }
    },
  )

  console.log(
    "Get Offers for token id: ",
    data3
  )

  let data4 = await client.queryContractSmart(
    marketPlaceContractAddress,
    {  get_all_offers:  {}
    },
  )

  console.log(
    "Get All Offers: ",
    data4
  )

```

Trade Queries

```typescript
let data = await client.queryContractSmart(
    marketPlaceContractAddress,
    {  get_trade:  {
        id: tokenId.toString(),
        trader: traderAddress }
    },
  )

  console.log(
    "Get trades for token id and trader address:  ",
    data
  )


let data2 = await client.queryContractSmart(
    marketPlaceContractAddress,
    {  get_trades_by_address:  {
        address: traderAddress, }
    },
  )

  console.log(
    "Get Trades for address: ",
    data2
  )

  let data3 = await client.queryContractSmart(
    marketPlaceContractAddress,
    {  get_trades_by_id:  {
        id: tokenId.toString(), }
    },
  )

    console.log(
    "Get Trades for token id: ",
    data3
  )


  let data4 = await client.queryContractSmart(
    marketPlaceContractAddress,
    {  get_all_trades:  {}
    },
  )

  console.log(
    "Get All Trades: ",
    data4
  )


```