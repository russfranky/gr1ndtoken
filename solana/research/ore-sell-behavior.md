# Ore Miner Sell Behavior — Research Findings

**Status:** local-only research for the wallet-mining sim calibration.
**Date:** 2026-10-04
**Question:** what fraction of mined rewards do miners actually sell vs hold in weeks 1–4 of a mining-token launch?

## Bottom line

No published week-by-week miner sell fractions exist for Ore v1. The honest
answer is: hard numbers were not found. What exists is strong directional
evidence, and it points in an uncomfortable direction for our sim's
sell_frac=0.3 assumption.

## What the evidence shows

### Ore v1 (April 2024) — the closest analog to our design

- **No structural incentive to hold, by the founder's own admission.**
  Hardhat Chad's v1 pause announcement (April 16, 2024) listed the three
  problems v2 had to fix; #2 was: "There is currently no structural
  incentive to hold the token. Miners who hold Ore should receive an
  advantage when mining."
  (https://cryptonews.com/news/ore-suspends-mining-amid-solana-congestion-to-prepare-for-v2/)
- **Price action consistent with heavy immediate selling:** ORE peaked at
  $3,786 on April 5 (three days after trading began) and was back near
  ~$2,000 within days — roughly a 45%+ retrace while mining continued at
  full tilt. (theblock.co, 2024-04-05)
- **Fast claiming:** of ~4,100 ORE mined in the first days, nearly 3,000
  had already been claimed. Claimed != sold, but miners paid real costs
  (priority fees, failed txs) and claimed into a liquid market.
  (theblock.co, 2024-04-05)
- **Direct qualitative signal (v2 era):** Blockworks' Jack Kubinec, mining
  ORE himself: "At this point in time it seems like it's almost always
  more profitable to just dump."
  (https://blockworks.com/news/lightspeed-newsletter-solana-ore-v2-review)

Reading: Ore v1 miners overwhelmingly sold. A 70%-hold assumption would
almost certainly NOT have held for v1.

### Ore v3 (current) — high hold rates, but heavily engineered

From the Blockworks Ore data dashboard primer
(https://blockworks.com/insights/ore-dashboard-primer):

- **~81% of circulating ORE supply is locked** (staked + treasury) since
  the v3 launch — but v3 pays ~17% staking APY (rolling 30D) and the
  protocol runs continuous buybacks (>$27.3M since v3 launch, 90% buried).
- **rORE (refining-yield) claims are ~1% of daily claims**, "indicating
  most long-term holders are retaining uORE with no major exits."
- **Refining APY runs 50–90%** (currently ~70%), paid from a 10% refining
  fee taken on every claim and redistributed to unclaimed holders.
- The v3 mining loop itself recaptures miner spend: losing tiles'
  SOL funds buybacks, so miner expense becomes buy pressure.

Reading: high hold rates ARE achievable — but only with a full stack of
hold incentives (claim tax redistributed to holders, staking yield,
programmatic buybacks). Our current design has none of these.

## Implication for the simulation

The sim's winning config needs miners to hold ~70% of rewards
(sell_frac=0.3), and nothing sustains at 70% immediate-sell. The Ore
evidence says:

1. **sell_frac=0.3 is not a free parameter; it must be earned by mechanism
   design.** Ore v1 (pure emission, no hold incentives — the closest analog
   to our spec) is consistent with very high immediate selling.
2. Ore v3 proves the hold rate is movable, but the levers are known and
   specific: a claim-time fee redistributed to holders (the 10% refining
   fee is the single most effective one), staking yield, and buybacks
   funded by protocol revenue.
3. **Recommended sim follow-up:** re-run the top configs at sell_frac 0.5
   and 0.7 to map the real corridor, AND/OR add a refining-fee-style hold
   incentive to the model (e.g. X% fee on claims redistributed pro-rata to
   unclaimed balances) and test whether it moves the sustainable sell
   fraction into reach.

## Closest proxies if harder data is needed later

- Bitcoin miner selling (public miner treasury data) — different cost
  structure (real capex/opex), weaker analog.
- Helium / Render / other "mine-and-earn" Solana launches with published
  holder-retention curves — worth checking if the sim needs a second
  calibration point.
- The Blockworks Ore dashboard itself (Dune-backed) is the live source to
  watch: miner P&L segmentation and the rORE-claim ratio are the closest
  real-time "do miners hold" instruments in production anywhere.
