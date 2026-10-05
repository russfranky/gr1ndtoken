# Ether Shrimp Farm — Referral System, Exact Mechanics

**Status:** local-only research for the wallet-mining launch plan.
**Date:** 2026-10-04

## The contract

**Ether Shrimp Farm** — the 2018 Ethereum "idle game" (4chan-originated,
May 2018). Players hatch shrimp → shrimp lay eggs daily → eggs are hatched
into more shrimp (compounding) or sold for ETH through an on-chain
bonding-curve market.

- **Contract address (mainnet):** `0x58aff91f5b48245bd83deeb2c7d31875f68b3f0d`
  (cited as the verified "Ether Shrimp Farm Source code";
  a clone deployment at `0xfeb591571f3c0db1a7c9280a24b71af6296ff4e0`
  was advertised for the ethfarmer.xyz mirror)
- **Source:** verified on Etherscan; canonical code mirrored in
  https://github.com/myglobalidentity/virtual-accelerator (README embeds
  the full `ShrimpFarmer` contract, Solidity 0.4.18)
- **Contemporaneous description:** "users earn 20% of the profits of
  anyone they introduce to the game through a referral link" (CCN, May
  2018); the game's own site: "Earn 20% the number of all eggs hatched by
  anyone who starts playing using your link"

## The referral mechanism, precisely

Solidity, from the contract source (function `hatchEggs`):

```solidity
mapping (address => address) public referrals;

function hatchEggs(address ref) public {
    require(initialized);
    if (referrals[msg.sender] == 0 && referrals[msg.sender] != msg.sender) {
        referrals[msg.sender] = ref;          // (1) set-once, permanent
    }
    uint256 eggsUsed = getMyEggs();
    uint256 newShrimp = SafeMath.div(eggsUsed, EGGS_TO_HATCH_1SHRIMP);
    hatcheryShrimp[msg.sender] = SafeMath.add(hatcheryShrimp[msg.sender], newShrimp);
    claimedEggs[msg.sender] = 0;
    lastHatch[msg.sender] = now;

    //send referral eggs
    claimedEggs[referrals[msg.sender]] = SafeMath.add(
        claimedEggs[referrals[msg.sender]],
        SafeMath.div(eggsUsed, 5)             // (2) 20% of hatched eggs
    );

    //boost market to nerf shrimp hoarding
    marketEggs = SafeMath.add(marketEggs, SafeMath.div(eggsUsed, 10));  // (3)
}
```

Mechanics, point by point:

1. **Who gets paid:** exactly one referrer per player, recorded in
   `referrals[msg.sender]` on that player's FIRST hatch that passes a
   `ref` address. It is permanent and can never be changed. Single level
   only — no multi-level / no depth.
2. **What %:** 20% (`eggsUsed / 5`) of the eggs the referred player hatches.
3. **On what actions:** ONLY on hatching (the compounding action). NOT on
   buying eggs with ETH, NOT on selling eggs. The referrer earns on the
   action every serious player repeats constantly — a continuous revenue
   stream, not a one-time bounty.
4. **Where the cut comes from:** ADDITIVE emission. The 20% is minted from
   nothing (`claimedEggs[referrer] += ...`) — it is NOT deducted from the
   referred player's eggs and NOT taken from the market. The referred
   player loses nothing, so there is zero resentment/friction in the
   referral.
5. **What the referrer gets:** `claimedEggs` — the same claimable-egg
   balance every player has. The referrer can hatch them into shrimp
   (compounding their own farm) or sell them for ETH through the market.
6. **Dev fee (separate):** 4% on egg buys and sells to `ceoAddress`
   (`devFee`). Not related to referrals.
7. **Anti-gaming properties:** effectively none.
   - The `referrals[msg.sender] != msg.sender` check is vacuous (it tests
     the stored value, which is `0` at that point by the first condition).
   - There is NO `ref != msg.sender` check: **self-referral is possible** —
     a player can pass their own address and earn +20% on their own
     hatches, pure profit, no cost.
   - No referral depth limit needed (single level by construction).
   - No sybil resistance at all: creating N wallets each referring the
     next costs only gas.

## The coupled anti-hoarding mechanic (worth noting)

Line (3): every hatch also adds 10% of the hatched eggs to `marketEggs`,
the virtual supply in the bonding-curve pricing (`calculateTrade`). More
`marketEggs` = worse egg prices for everyone. This punishes hoarding and
pushes players to keep hatching (compounding) rather than sitting on
shrimp — which in turn generates MORE referral eggs. The referral system
and the compounding incentive are coupled: the thing that pays referrers
is the same action the price mechanics push players toward.

## Why it was effective at driving growth

1. **Paid on the repeat action, not the entry action.** A one-time
   signup bounty pays once; 20%-of-every-hatch pays forever. Referrers
   were incentivized to recruit players who would COMPOUND (the highest-
   value behavior), because idle or exit-only players generate no
   referral eggs.
2. **Additive, not extractive.** The referred player loses nothing, so
   referral links spread without the "you're taxing me" objection that
   kills zero-sum affiliate schemes.
3. **Permanent attribution.** Set-once referrer = lifetime 20% of that
   player's compounding. This made referral-link spam rational even for
   small referrers: one whale recruit pays for years.
4. **Frictionless funnel underneath it:** `getFreeShrimp()` gave every new
   wallet 300 shrimp for just gas. Free entry + referral link = the
   viral loop had no buy-in barrier.
5. **4chan-native distribution:** the game spread through the same forums
   that created it; referral links were the native unit of sharing.

## Caveats for our use

- This is a **closed-loop Ponzi game**: referral eggs were valuable only
  because later players' ETH bought eggs through the same bonding curve.
  The referral system amplified growth AND the eventual collapse (egg
  supply hyperinflated; the market drained). Mechanically brilliant,
  economically terminal.
- The lack of self-referral protection is a non-starter for an
  agent-driven system: a scraping agent WILL self-refer on day one, and
  referral rings are trivially scriptable. Any adaptation needs an
  explicit answer to this (see the Jev eval).
- Single-level only. Multi-level was never tried here; the BNB-Miner
  family later experimented with similar single-level cuts.

## Adaptation sketch for our Solana design (for the Jev eval)

`claim` accepts an optional referrer pubkey. On first claim carrying a
referrer, a PDA records miner→referrer permanently (rent ~0.002 SOL).
On every subsequent claim, the referrer receives e.g. 10–20% of the
reward — either minted additively (extra emission) or split zero-sum
from the reward. The shrimp lesson says: pay on the REPEAT earning
action (claims), make it additive so the miner loses nothing, and make
attribution permanent.
