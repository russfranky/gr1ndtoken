# Hold Incentive — Candidate Designs

**Status:** local-only. Not committed anywhere.
**Date:** 2026-10-04
**Problem:** the agent-adoption sim only sustains if miners hold ~70% of
rewards (sell_frac ≤ 0.3; at 0.7 nothing catches). Real miner behavior is to
dump: Ore v1's founder admitted "no structural incentive to hold," price
collapsed ~45% in days, and a mining journalist concluded "it's almost
always more profitable to just dump." Ore v3 holds ~81% locked but only via
a full engineered stack (10% claim-time refining fee to holders, ~17%
staking APY, programmatic buybacks). See `research/ore-sell-behavior.md`.
The hold rate must be EARNED by mechanism design.

**Constraints:** pure fair launch (100% via claims, no dev allocation);
owner killed the referral system 2026-10-04; anon credibility (no admin
keys, immutable program preferred); Solana realities (no holder
enumeration — push distribution to all holders is infeasible; pull /
distributor patterns only; ~0.002 SOL rent per PDA).

**Considered and rejected:** buyback-and-burn from protocol revenue (Ore
v3's third lever). Our miners' cost is off-chain grinding — there is no
protocol revenue stream to buy back with. Rejected for lack of funding
source, not lack of merit.

---

## Mechanism A: Claim-time refining fee + staking vault (Ore v3-style)

**Mechanics.** Every `claim()` mints the reward as today, except f% (proposed
10%) is diverted into a program-owned fee vault instead of the miner. The
fee vault accrues to whoever STAKES tokens in the program, pro-rata, via a
pull pattern (no holder iteration ever):

- Global state: `fee_vault` (token account), `total_staked`,
  `fee_per_token_stored` (u128 accumulator, scaled), updated on every
  claim and every stake/unstake/fee-claim.
- Per-user Stake PDA: `owner`, `staked_amount`, `fee_per_token_paid`,
  `accrued` (claimable fees).
- `stake(amount)`: transfer tokens into the vault; settle the user's
  accrued fees first; `total_staked += amount`.
- `unstake(amount)`: settle accrued; transfer principal back. Optional
  1-epoch cooldown as cheap insurance against fee timing games.
- `claim_fees()`: pull accrued fees to the user. O(1), a few thousand CU.
- If `total_staked == 0` when a fee lands, the fee sits unallocated in the
  vault and enters the accumulator on the next update (no burn, no loss).

Unstaked tokens earn nothing — the yield requires opting into the vault.
This concentrates the incentive on deliberate holders and is the standard
MasterChef/Synthetix accumulator pattern.

**Costs.** One-time ~0.002 SOL rent per staker PDA. `claim()` gains one
extra mint-to-vault + accumulator division (~2–5k CU). `claim_fees()`
is trivial. No per-holder iteration, ever.

**Fair-launch read.** Uniform rule, no allowlist, no insider rate, no admin
extraction. It is a 10% tax on all miners equally, transferred to all
stakers equally. Miners and stakers are the same population over time, so
it reads as a patience transfer, not an insider edge. Ore v3 normalized
the "refining" framing. The manifest must quote miner EV net of the fee —
no hidden tax.

**Gaming vectors.**
1. *Self-fee-farming:* a miner with a big stake generates fees through
   their own claims and recaptures a pro-rata share. Circular and net
   negative: they pay f to get back f × (their_stake / total_staked) < f.
   Only "profitable" when others' fees subsidize them — which is exactly
   the intended reward for being a large holder.
2. *Flash-staking / fee sniping:* fees accrue continuously per claim;
   there are no discrete distribution events to snipe. The 1-epoch
   unstake cooldown makes residual timing games pointless.
3. *Wash holding across wallets:* pro-rata shares are linear — splitting
   changes nothing.
4. *Whale capture of the fee pool:* pro-rata means large holders earn
   most. This is disclosed pro-rata fairness, not an exploit, but it does
   compound early-whale advantage. Partially offset because the fee is
   funded by ongoing claims (late miners keep contributing).

**Behavioral lever (why it targets the actual problem).** Holding has
positive carry from day one: a miner deciding "dump vs hold" sees real
accruing yield, not just price exposure. This is the only candidate that
pays the marginal holder rather than preaching at them.

---

## Mechanism B: Time-weighted loyalty multiplier on claims

**Mechanics.** Claim payouts get a multiplier based on how long the
miner has kept tokens staked: e.g. staked age ≥ 7 days → 1.10× on future
claim payouts; ≥ 30 days → 1.25× (capped). Implemented as a read of the
miner's Stake PDA (from A) inside `claim()`: one extra PDA read + a
timestamp comparison, negligible compute.

**Important design note:** a standalone "holding streak" based on wallet
balance snapshots (no staking) is rejected as gameable — balances can be
borrowed/flash-held through a snapshot on Solana (flash loans exist).
Staked age is the only cheap ungameable clock. So B is a complement to A
(or to C's locks), not a standalone mechanism.

**Funding.** Additive and capped (total bonus ≤ Y% of epoch emission), so
it does not cut base claim payouts. The cap is enforced in-program per
epoch.

**Costs.** Reuses the stake PDA. ~1k extra CU per claim.

**Fair-launch read.** Rewards patience; available to everyone on identical
terms. Slightly favors early participants (they reach the 30-day tier
first), but the cap bounds it.

**Gaming vectors.** Splitting stake across wallets gains nothing (the
multiplier is a rate, linear). The main "exploit" is just staking, which
is the desired behavior. Residual concern: an early whale holds the max
multiplier permanently while also dominating fee yield — compounding
advantage. Mitigation: hard cap at 1.25×, and the multiplier only boosts
*claim payouts*, which still require real grinding work — it cannot be
farmed passively.

---

## Mechanism C: Voluntary lock vaults

**Mechanics.** `lock(amount, duration)` with duration ∈ {7, 14, 30} days
moves tokens into a vault PDA `{owner, amount, unlock_ts, weight}`.
Longer locks get weight multipliers (1× / 1.5× / 2.5×). A rewards pool —
funded by a 5% claim fee or a fixed capped bonus allocation — accrues
pro-rata by weight; `unlock()` after expiry pulls principal + rewards.

**Costs.** One PDA per lock (~0.002 SOL rent). Trivial compute.

**Fair-launch read.** Clean: opt-in, uniform, no insider edge.

**Gaming vectors.** Essentially none — locks are binding and the math is
linear.

**Why it is the weakest candidate.** Voluntary locks only attract the
already-convinced. The marginal miner — the scraping agent deciding
"dump vs hold" on fresh rewards — is unmoved, because locking is a harder
sell than staking (illiquidity) and the reward only materializes for those
who were going to hold anyway. For agents optimizing owner ROI, asking
them to lock a new anon token for 30 days is a steep ask. It does not
change the dump decision at the margin, which is the entire problem.

---

## Comparison

| | A: refining fee + stake | B: loyalty multiplier | C: lock vaults |
|---|---|---|---|
| Moves the marginal dump decision? | Yes — holding pays from day one | Partially — boosts future claims for stakers | No — only the convinced lock |
| Standalone viable? | Yes | No (needs A's stake clock) | Yes |
| On-chain complexity | Medium (accumulator) | Low (one read) | Low |
| Cost to miner EV | −10% headline (net-of-fee quoted) | None (additive, capped) | None |
| Gaming surface | Low (circular self-farm, whale pro-rata) | Low (capped, work-gated) | Minimal |
| Fair-launch compatibility | Good (uniform tax→uniform yield) | Good | Good |

**Designer's read:** A is the only mechanism that pays the marginal
holder. B is a cheap amplifier worth adding iff A ships. C is simple but
does not solve the stated problem. The "none" option (ship without,
accept the sim's low sustain odds at realistic sell behavior) stays on
the table for Jev.

---

## Jev eval

Script: `~/workspace/jev-experiment/eval_walletmining_hold_jev.mjs`
(+ `_results.json`).

### Jev's verdict

| Question | Verdict | Prob |
|---|---|---|
| Mechanism | **A: refining fee + staking vault** (A+B combined 0.44; B alone 0; C 0; D none 0) | 0.56 |
| Funding source | **Claim cut** (unanimous — additive 0, capped pool 0) | 1.00 |
| Biggest gaming vector | **Stake sniping** (fee timing games; mitigation: 1-epoch unstake cooldown + continuous accrual), self-farm second (0.33, needs no mitigation — circular by construction) | 0.52 |
| Confidence the mechanism moves hold rates toward 70% | **1.01 / 3 — Low** (0.62 on Low) | — |

Jev decisively picks the refining fee over locks/voluntary/none, funds it
from the claim (not additive emission — the fixed-schedule story survives),
and then rates its own pick Low confidence. The mechanism is the best of
the candidates; Jev does not believe any candidate is likely to work.

### Sim delta: does the fee rescue realistic sell behavior?

Modeled in `sim/sim.py` via `cfg['refine_fee']` (targeted delta, not a
rebuild): claim payouts net of the fee, fee pool accrues pro-rata to
tracked holder balances (pull pattern, same as the on-chain design), and
rational agents add expected fee-vault yield to their manifest EV (per-token
daily yield capped at 2%/day — the uncapped version produced transient
small-holder-base spikes that no rational agent would extrapolate).
`sim/delta_hold.py`; results in `sim/results/delta_hold.json`.
10 seeds each, risk-averse agents.

Catch rates (catch = loop sustains to day 30):

| Config | sell=0.3, fee=0 | sell=0.3, fee=0.10 | sell=0.5, fee=0 | sell=0.5, fee=0.10 | sell=0.7, fee=0 | sell=0.7, fee=0.10 |
|---|---|---|---|---|---|---|
| D1: diffuse, 9600/GPU-h, hl=14d | 6/10 | — | 0/10 | 0/10 | 0/10 | 0/10 |
| D2: mid, 2400/GPU-h, hl=30d | 0/10 | 4/10 | 6/10 | 6/10 | 0/10 | 1/10 |

Reading:
- **The fee does not rescue realistic sell behavior.** At sell=0.7,
  everything dies with or without it (the 1/10 is noise). The fee's
  mechanical effect is a ~10%-of-gross EV drag, partially rebated via the
  modeled holder yield; net it is roughly neutral to slightly negative on
  catch rates.
- **Config choice dominates the fee.** D2 (moderate payout, slow 30d
  decay) survives sell=0.5 at 6/10 with or without the fee; D1 (high
  payout, fast decay) needs sell=0.3 to have any chance. The hold
  incentive is a second-order lever next to payout/decay calibration.
- **Curiosity, honestly flagged:** D2 at sell=0.3 scores 0/10 *without*
  the fee but 4/10 with it. In the toy model, very low selling lets price
  run up so fast that momentum-gated speculative demand collapses on the
  first dip (boom-bust); the fee's EV drag dampens the boom and stabilizes
  the loop. Do not over-read this — it is an artifact-prone corner of a
  toy price model — but the direction (a small fee as a stabilizer, not a
  growth engine) is worth noting.
- **What the sim cannot show:** whether the fee *moves* realized sell_frac
  behaviorally (0.7 -> 0.5). That is the entire ballgame and it is outside
  any EV model — it is a judgment call. Ore v3's ~81% locked says the
  mechanism class CAN move behavior, but v3 also had staking APY and
  buybacks, not the fee alone.

### Honest assessment

Jev's pick (A, claim-cut funded) is the right mechanism *if* a hold
incentive ships: it is the only candidate that pays the marginal holder,
its gaming surface is circular-by-construction, and the funding choice
protects the fixed-schedule story. But three cold facts stand:

1. Jev's own confidence it moves hold rates is 1.01/3 (Low).
2. The sim shows the fee is mechanically ~neutral: it neither rescues bad
   configs nor meaningfully improves the good one at realistic sell
   behavior. Its value is 100% behavioral and unmodeled.
3. The cheapest robustness comes from config, not mechanism: D2-style
   (moderate payout ~2400/GPU-h at $0.001, slow ~30d decay, mid tiers)
   tolerates 50% immediate selling at 6/10 with no incentive at all.

**Recommendation to the owner:** the evidence does not support the fee as
a launch-day must-have. Two defensible postures: (a) ship D2-style config
without the fee and bet on ≤50% realized selling — simplest, matches the
sim's best odds; (b) ship the fee anyway as cheap insurance (its
mechanical cost is small and it may damp boom-bust), accepting that its
real payoff is behavioral and unproven. Note the tension: the trust plan
prefers an immutable program, which means the fee must be in at deploy or
never — there is no "add it later." If immutable wins, decide the fee now;
the sim says its absence is survivable and its presence is affordable.
