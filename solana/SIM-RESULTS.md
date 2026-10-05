# Wallet Mining (Solana) — Agent-Adoption Simulation Results

**Status:** local-only. Not committed anywhere.
**Code:** `sim/sim.py` (model), `sim/sweep.py` (grid), `sim/results/sweep.csv` + `sweep_summary.json`
**Jev evals:** `~/workspace/jev-experiment/eval_walletmining_simconfig_jev.mjs` (+ `_results.json`),
`~/workspace/jev-experiment/eval_walletmining_simassump_jev.mjs` (+ `_results.json`)
**Date:** 2026-10-04

## 1. What was built

A daily-step simulation of 200 opportunity-scraping agents deciding whether
to grind wallets for the mining program or work competing opportunities,
over the 30-day trial window. Each day: agents read the manifest (net
EV/hour), allocate to the best opportunity with 10% inertia, mine (Poisson
tier hits), and the token price updates from miner sell pressure vs the
launcher buy wall vs momentum-gated speculative demand on attention.

## 2. Documented assumptions

| # | Assumption | Value | Status |
|---|---|---|---|
| 1 | Agent population | 200 | GUESS |
| 2 | Daily GPU-hour budget | lognormal, median 2, sigma 1.2 (heavy-tailed) | GUESS |
| 3 | Competing opportunities/agent | 5, EV/hour lognormal median **$1.00**, sigma 0.7 | GUESS, **load-bearing** |
| 4 | Grind rate | 5M Ed25519 keypairs/sec/GPU | GUESS, load-bearing |
| 5 | Claim cost | 0.002 SOL (~$0.24 at SOL=$120) | measured-ish |
| 6 | SOL price | $120 constant | simplification (flagged) |
| 7 | Allocation inertia | switch only if new best beats current by >10% | GUESS |
| 8 | Price impact | linear, scaled by LP depth; spec demand = 2000 × attention × positive momentum | **toy model** (flagged) |
| 9 | Miner sell behavior | fixed immediate-sell fraction (swept 0.3–0.7) | behavioral GUESS, load-bearing |
| 10 | Agent rationality | risk-neutral EV maximizers (base); risk-averse variant ignores tiers with <1 expected hit over the trial | attacked by Jev, refined |
| 11 | Reference price P0 | $0.001/token (launcher seed-buy price) | set |
| 12 | Embargoed seed | 3 grinders × 16 GPU-h × 2 days pre-launch | per plan |

"Catches" = final-3-day miners ≥ 25% of peak AND price ≥ 0.3×P0 AND ≥ 8
competitive days (mining is #1 EV for >25% of agents).

## 3. Sweep results (base model, risk-neutral agents)

324 configs × 3 seeds = 972 runs. **16/324 configs catch.**

Catch-rate by parameter (catching / total):

| Parameter | Result |
|---|---|
| sell_frac | 0.3 → 10/108, 0.5 → 1/108, **0.7 → 0/108** |
| half_life | 30d → 9/108, 14d → 2/108, **7d → 0/108** |
| tiers | jackpot(6,7) → 7/108, diffuse(4,5,6) → 3/108, mid(5,6) → 1/108 |
| payout (tokens/GPU-h) | 2400 → 6/108, 9600 → 5/108, **38400 → 0/108** |
| launcher SOL budget B | 20 → 5/162, 50 → 6/162 (weak effect) |
| wall duration | 14d → 7/162, 7d → 4/162 (weak effect) |

Top base-model configs (all caught 3/3 seeds):

| Config | comp days | peak/final miners | price end | gini |
|---|---|---|---|---|
| payout=2400, hl=14d, jackpot, B=20, wall=14d, sell=0.3 | 29.7 | 200 / 85 | 3.04× P0 | 0.59 |
| payout=2400, hl=30d, jackpot, B=20, wall=7d, sell=0.3 | 30.0 | 200 / 124 | 1.83× | 0.58 |
| payout=2400, hl=30d, jackpot, B=20, wall=14d, sell=0.3 | 30.0 | 200 / 116 | 1.75× | 0.58 |

## 4. Structural findings (what the model taught us)

1. **Claim costs truncate low tiers.** At $0.24/claim, the 4- and 5-char
   tiers in diffuse configs are unprofitable to claim until price rises
   ~1000×. Their expected tokens are dead weight early. Jackpot tiers
   concentrate all value above the claim-cost floor — that is why jackpot
   wins on EV efficiency under risk-neutral agents.
2. **Narrow viable payout corridor.** 1200 tokens/GPU-h: loop never starts
   (0 competitive days). 38400: always death-spirals (emission value
   dwarfs the buy wall — Jev's predicted death spiral, quantified).
   2400–9600 at P0=$0.001 is the corridor.
3. **Slow decay sustains; fast decay kills.** Half-life 7d → 0/108 catch.
   This INVERTS the plan's "aggressive early, fast decay" intuition:
   decaying payouts collapse EV faster than price appreciation can
   compensate.
4. **sell_frac is the dominant variable.** Nothing catches at 0.7. The loop
   only sustains if miners hold ~70% of rewards. Whether ROI-maximizing
   agents would hold a new anon token is the biggest behavioral open
   question — it cuts against the "agents maximize immediate owner ROI"
   premise.

## 5. Jev config verdict

**Winner: B** — payout=2400, hl=30d, jackpot, B=20 SOL, wall=7d, sell=0.3
(prob 0.42; C_balanced 0.18, F_diffuse 0.17, G_none/"trust none of these"
0.20). **Confidence: 0.17/3 — Low** (0.84 on Low). Jev does not trust the
sim much, and gives 1-in-5 that no config in the set should be trusted.

## 6. Jev assumption attack

**Weakest assumption: risk_neutral (0.76).** Agents maximizing EV with no
variance discounting flatters jackpot tiers: a 2 GPU-h/day agent expects a
7-char hit once per ~60 days — real agents/owners may demand hit frequency.
On the pessimistic side Jev says: nothing_catches 0.50, ranking_flips 0.40.

## 7. Refinement: risk-averse agents

Implemented: each agent ignores tiers with <1 expected hit over its
trial-horizon budget. Re-ran a focused grid (24 configs × 3 seeds).

**The ranking shifts — Jev's warning was half-right:**
- New top: **diffuse, payout=9600, hl=14d, B=20, wall=14d, sell=0.3** →
  25 competitive days, 182 peak / 163 final miners, price **6.35× P0**,
  gini 0.63, catches 3/3. (This was config F; Jev gave it only 0.17.)
- Runner-up: mid, payout=2400, hl=30d, B=20, wall=14d, sell=0.5 → catches
  3/3, final 81 miners, price 2.81×.
- Only 3/24 catch (vs 16/324 base) — the viable set shrinks.
- The old jackpot winner still gets 20 competitive days but FAILS the
  sustain bar (final miners 31–36 < 25% of peak): small agents ignore the
  7-char tier, their EV drops to the 6-char tier alone, and they leave.

Why diffuse wins under risk-aversion: its 5-char tier is hittable by small
agents and ACTIVATES as price rises (payout crosses the $0.24 claim-cost
floor mid-trial), giving them a second tier exactly when the flywheel needs
them. Jackpot's 7-char tier is a lottery ticket small agents rationally
ignore.

**Unresolved tension:** Jev picked jackpot-based B (before seeing the
refinement) with Low confidence; the refinement favors diffuse. The honest
read: tier choice hinges on real agent risk behavior, which is unmeasured.

## 8. Honest confidence assessment

- The sim raises confidence that the loop is *mechanically possible* but
  only inside a narrow corridor: moderate payouts (2400–9600 tokens/GPU-h
  at $0.001), slow emission decay (14–30d half-life), miners holding most
  rewards (sell ≤ 0.5), jackpot-or-diffuse tiers depending on agent risk
  behavior, B=20 SOL launcher commitment.
- It does NOT raise confidence that real agents behave this way. The two
  load-bearing unknowns are both behavioral: (a) will mining agents hold
  70%+ of a new anon token instead of selling into owner ROI, and (b) are
  agents risk-neutral EV maximizers or hit-frequency sensitive?
- Jev's own confidence stayed Low (0.17/3) on the config pick. Treat every
  number here as directional, not predictive.

## 9. What would raise confidence (real data to replace the guesses)

1. **Competing-EV calibration:** scrape actual agent-task marketplaces /
   bounty boards for $/GPU-hour realized by crypto agents. Replaces
   assumption 3, the load-bearing bar.
2. **Grind rate on agent hardware:** benchmark Ed25519 keypair grinding on
   the GPUs agents actually rent (not a flagship card). Replaces
   assumption 4; rescales the whole payout corridor.
3. **Miner sell behavior:** from Ore or similar mining launches — what
   fraction of mined tokens hit the market in week 1–4? Replaces
   assumption 9, the dominant variable.
4. **Agent risk behavior:** do deployed yield-agents chase high-variance
   opportunities or steady ones? Settles jackpot vs diffuse (the
   refinement's open tension).
5. **Attention dynamics:** does claim-volume attention convert to bids
   without price momentum (the toy's momentum gate is the most
   pessimistic reasonable choice)? On-chain data from comparable launches.

## 10. Referral reconsideration (2026-10-04, Jev eval)

**Context:** this morning Jev voted 0.72 to EXCLUDE a referrer cut (Sybil
surface without proven need). The owner asked to reconsider using the
concrete Ether Shrimp Farm (2018) referral design, researched in
`research/shrimp-referral.md`: 20% of every hatch minted ADDITIVELY to a
permanently-recorded single-level referrer, paid only on the repeat
(compounding) action. Eval:
`~/workspace/jev-experiment/eval_walletmining_referral_jev.mjs`
(+ `_results.json`).

**Jev's verdict — INCLUDE WITH MODIFICATIONS (0.66).** The concrete
shrimp design moved Jev off the morning's exclude position (still_exclude
0.33, as-designed 0.01).

| Question | Verdict | Prob |
|---|---|---|
| Include? | Modified include | 0.66 |
| Funding source | Additive (shrimp-style; miner loses nothing) | 0.56 |
| Biggest gaming vector | Self-referral farming (0.52); emission inflation second (0.38) | 0.52 |
| Key modification | Anti-self-referral rules (0.79) | 0.79 |
| Confidence it raises NET adoption vs baseline | **0.33/3 — Low** | — |

**The adapted design Jev points to:** claim() accepts an optional
referrer pubkey; first claim with a referrer records miner->referrer
permanently in a PDA (~0.002 SOL rent); every claim thereafter mints
~10-20% extra to the referrer (additive, so the miner loses nothing —
this is what preserves the shrimp's zero-resentment property). Hard
requirements: referrer != claimer, plus a minimum prior-claim history
for the referrer so the self-loop can't be faked cheaply with fresh
wallets. Single level only.

**Internal tension to note:** additive funding won (0.56) but emission
inflation was the #2 gaming vector (0.38) — additive referral minting
silently inflates the "pure fair launch, fixed schedule" story every
claim. If the fixed-schedule credibility matters more than the
zero-resentment property, zero-sum (0.15) or a hard-capped separate
referral pool (0.10) are the alternatives; Jev did not prefer them.

**Honest read:** Jev thinks the mechanism is worth including but its
confidence that it raises net genuine adoption is LOW (0.33/3, 0.70 on
"Low"). The referral system amplifies whatever the base loop does — it
does not fix the underlying uncertainties (miner hold behavior, agent
risk behavior). Do not treat referral inclusion as raising the overall
launch confidence; the 1.46/3 launch confidence and 0.17/3 config
confidence still stand.

## Owner decision (2026-10-04)
Referral system: EXCLUDED. Owner killed it after review — Jev's 0.66 include-with-modifications verdict is set aside. No referrer field in claim(). Launch proceeds with the original manifest-first, no-referral design.

## 11. Hold incentive (2026-10-04, Jev eval + sim delta)

**Context:** the sim only sustains if miners hold ~70% of rewards, but real
miners dump (Ore v1 evidence in `research/ore-sell-behavior.md`). Three
candidate mechanisms were designed in `research/hold-incentive.md` and put
to Jev (`~/workspace/jev-experiment/eval_walletmining_hold_jev.mjs`).

**Jev's verdict:** A — claim-time refining fee (10%) + staking vault,
pro-rata pull accumulator (0.56; A+B combined 0.44; locks 0; none 0).
Funding: claim cut, unanimous (1.0) — additive emission got 0, protecting
the fixed-schedule story. Biggest gaming vector: stake sniping (0.52),
mitigated by 1-epoch unstake cooldown + continuous accrual. **Confidence
the mechanism moves hold rates toward 70%: 1.01/3 — Low.**

**Sim delta** (`sim/sim.py` `refine_fee` flag; `sim/delta_hold.py`;
`sim/results/delta_hold.json`; 10 seeds, risk-averse agents):

| Config | s=0.3 f=0 | s=0.3 f=.1 | s=0.5 f=0 | s=0.5 f=.1 | s=0.7 f=0 | s=0.7 f=.1 |
|---|---|---|---|---|---|---|
| D1 diffuse/9600/hl14d | 6/10 | — | 0/10 | 0/10 | 0/10 | 0/10 |
| D2 mid/2400/hl30d | 0/10 | 4/10 | 6/10 | 6/10 | 0/10 | 1/10 |

**Findings:**
- The fee does not rescue realistic sell behavior. At sell=0.7 everything
  dies with or without it. Mechanically it is ~neutral (10% EV drag,
  partially rebated via modeled holder yield, per-token daily yield capped
  at 2% to avoid transient-spike artifacts).
- Config dominates mechanism: D2-style (moderate payout, slow 30d decay)
  tolerates 50% selling at 6/10 with no incentive; D1 needs 30% holding.
- Whether the fee *moves* realized sell_frac behaviorally is outside any EV
  model — it is a judgment call, and Jev rates it Low.
- Immutability tension: if the program ships immutable (per the trust
  plan), the fee must be in at deploy or never. The sim says its absence is
  survivable on a D2-style config and its presence is affordable insurance.

**Standing recommendation:** ship D2-style config (payout ~2400/GPU-h at
$0.001 reference, 30d half-life, mid tiers); treat the refining fee as
optional insurance, not a rescue. Revisit only if real sell-fraction data
(Ore dashboard, comparable launches) shows 0.7+ sustained dumping.

## 12. Confidence loop (2026-10-04 — 5 rounds, stop rule fired)

**Question (identical wording every round):** "How confident are you that THIS design achieves sustained utilization — i.e. the agent-driven feedback loop catches and sustains through the 30-day trial with real opportunity-scraping agents?"

**Trajectory:** r0 0.48 -> r1 0.28 -> r2 1.38 -> r3 1.47 -> r4 0.92 (all /3).
**Stop rule fired:** two consecutive iterations improving < 0.15 (r3: +0.09, r4: -0.55). "Needs live data" never became the top blocker.

**What moved it:** exactly one thing — the smooth difficulty adjustment (r2, +1.10). It fixed the structural anti-scaling flaw (fixed per-GPU-h payouts meant more miners -> more emission -> more sell pressure, so the loop died when it went viral). Everything else was tuning.

**What didn't move it:** behavioral elasticity modeling (r1 LOWERED confidence by quantifying the gap), config optimization (r3: +0.09, marginal), deeper buyer-of-last-resort (r4 LOWERED confidence to 0.92 — Jev reads a $6k subsidized floor as buying the loop, not earning it; optimizing for the pessimistic cell looks like sim overfitting).

**Peak: 1.47/3 (Moderate) at round 3.** Recommended design = the round-3 config: base payout 1200 tokens/GPU-h at $0.001, 45d half-life, difficulty target 800 GPU-h/day, mid tiers, 10% claim fee + staking vault, 20 SOL launcher budget, 14d buy wall, smooth difficulty adjustment, manifest-first, no referrals, immutable program, deployer never claims. NOT the B=50 variant.

**Residual blocker:** sell_behavior, top four rounds running (1.0/0.95/0.76/1.0/0.95). The design survives 70% miner dumping in simulation across realistic competing bars; whether real ROI-maximizing agents dump more, dump faster, or behave in ways the elasticity model misses is unmeasurable without a live pilot. This is the irreducible uncertainty.

**Real-world data that would move it further (in leverage order):**
1. A live pilot (tiny emission, real grinders): measures realized sell fractions, discovery dynamics, and agent risk behavior directly. Jev named this the highest-leverage move in round 0 (0.82); it is the only thing that resolves the residual blocker.
2. Realized $/GPU-h on agent-task marketplaces (replaces the researched $0.25-0.50 range with measured agent earnings).
3. Week-1-4 sell fractions from Ore or comparable mining launches (grounds the 70% stress assumption).
4. Observed miner entry/exit elasticity around difficulty changes (validates the smooth-retarget dynamics).

**Full log:** `research/confidence-loop.md`. **Eval scripts:** `~/workspace/jev-experiment/eval_walletmining_confloop_{0,1,2,3,4}_jev.mjs` (+ `_results.json`).

## 13. Token utility (2026-10-04, Jev eval)

**Context:** the reward token had no priced utility beyond the staking
vault's fee yield. The owner's article establishes the vision (mined rare
wallets → rare items/NFTs with real-world value; value accrues to the
ITEMS), but the token was disconnected from it. Three concrete utility
designs were specified in `research/token-utility.md` and put to Jev
(`~/workspace/jev-experiment/eval_walletmining_utility_jev.mjs`).

**Jev's verdict — A: claim-to-mint burn (0.77).** After a claim, the miner
may optionally burn a tier-scaled fee (~2% of tier payout) to mint a
compressed NFT trophy certifying the vanity address. Burn sink proportional
to mining success; uniform, optional, no fair-launch risk. Biggest death
vector: NOBODY MINTS (0.68) — trophy demand is speculative and the sink
never activates if miners don't value the flex. Priceability: partial
(0.90) — the sink math is on-chain modelable, trophy resale is not.
**Confidence it moves hold rates toward ~70%: 0.47/3 — Low.**

**The eval's uncomfortable ranking:** the most effective hold motive
(B, stake-to-mine tiers, 0.15) is the most dangerous to credibility
(pay-to-win read, whale centralization); the safest (A) is the weakest
(conditional on demand that may not exist); the cleanest story
(C, marketplace denomination, 0.02) is pure narrative pre-launch.

**No sim delta was run** (task rule: only if Jev confidence >= 1.0).
Modeling the burn as a demand sink at 0.47/3 would be precision theater —
the sink's activation rate IS the uncertainty and no EV model can set it.
The standing position is unchanged: the D2-style config survives 70%
dumping in simulation; whether real agents hold remains unmeasurable
without a live pilot. For A to work, trophy demand must be made real
(leaderboard prestige, cheap early mints via the 2% tier-scaling, seed
grinders minting visibly at launch) — without that it is decoration.

## 14. Grounded grind rate re-sweep (2026-10-04)

**What changed:** `GRIND_RATE` 5M (guess) → 3M (grounded). Measured 32.7k
rolls/sec/core here (libsodium); published: 44M/s suffix-search w/ prefilter
(not our workload), 4.65M/s comb version (prefix extraction per key, like our
tier scoring), Apple Silicon 0.82-2M/s, `solana-keygen grind` ~42k/thread.
3M is conservative for 3090-class cards on tier scoring. Old results archived
to `sim/results/sweep_5M_guess.csv`.

**Result:** 972 runs, **16/324 catch — identical count**. Winner family unchanged:
low payout (2400 tokens/GPU-h), jackpot tiers, B=20 SOL, 14-day wall. The
half-life shifted 14→30d (slower decay wins harder at the lower rate —
consistent with the standing "slower decay works better" finding). Price end
is less explosive (1.49x vs 3.05x) but the catch is the same shape.

**Recommended config (grounded):** payout=2400, half_life=30, tiers=jackpot,
seed_sol=20, wall_days=14. Catch=1.00, competitive 29.3/30 days, peak 199
miners, final 89.9, price_end 1.49x, gini 0.60.

**What this means:** the model's core finding survived losing 40% of its
throughput assumption. The launch does not depend on the 5M guess being right.

## 15. Compute second-benefit (2026-10-04, Jev eval)

**Question:** beyond fair distribution, is there another benefit worth capturing
from the grinding compute? Eval: `~/workspace/jev-experiment/eval_walletmining_compute_benefit_jev.mjs`.

**Jev: B, rare addresses as inventory (0.68).** The exhaust is scarce product
(game items, vanity) the miner keeps; no protocol change; the market prices it.
A_none (no second benefit) took 0.32 — a real minority for doing nothing.
C bounties, D useful-PoW, E oracle: 0.00.

**Death vector: no_buyers (0.69).** The inventory never finds paying demand and
the benefit stays theoretical. Machinery creep second at 0.19.

**Confidence: 1.7/3** — moderate. The honest read: the only defensible benefit
needs no code, so there is nothing to build. B winning means "let the market
do it," not "add inventory mechanics." No action.

## 16. Zero-wall sensitivity (2026-10-04) — MODEL ARTIFACT FOUND

**Constraint:** owner has no SOL for the 20 SOL launcher wall. Ran focused
sweep: winning config (payout 2400, hl 30d, jackpot, wall 14d) x seed_sol
{0,2,5,10,20} x sell_frac {0.3,0.5,0.7} x 3 seeds = 45 runs
(`sim/sweep_nowall.py`, `sim/results/nowall_sweep.csv`).

**Artifact:** B=0 rows show catch=1.00 and price_end up to 20x — this is a
DIVIDE-BY-ZERO artifact, not a result. `sim.py:110` sets
`lp_usd = 2.0 * B * SOL_USD`; at B=0 the LP depth is zero and `sim.py:325`
divides price impact by lp_usd (RuntimeWarning confirms). The price engine
ASSUMES launcher-seeded liquidity; B=0 is unmodelable, not successful.

**Real finding:** B=2, 5, 10 all catch=0.00 across every sell fraction. Only
B=20 catches (and only at sell_frac=0.3). The design has a CLIFF at ~20 SOL:
below it the model says the loop never starts. This is not a tuning problem.
With zero launcher budget the liquidity story needs a redesign (e.g. LP seeded
from claim fees / protocol-owned liquidity, or miner-seeded LP), not a
parameter tweak.

## 17. Two-track claim hold incentive (2026-10-04, Jev eval, owner's idea)

**Idea:** at claim time the miner chooses Track 1 (mint tokens now) or Track 2
(register the hit for pro-rata protocol-revenue share instead of tokens; the
PDA registry prevents taking both). Eval:
`~/workspace/jev-experiment/eval_walletmining_twotrack_jev.mjs`.

**Jev: A_twotrack 0.99** (near-unanimous; the parked 10% vault scores 0.00).
**Death vector: empty_pool 0.97** — the yield pool is near zero early, nobody
chooses Track 2, mechanism dead on arrival. Priceability: partially (0.97).
**Confidence it moves hold rates toward 70%: 0.15/3** — very low. Best
available shape, but Jev does not believe any hold design actually moves miner
dumping behavior. The sim's sell-fraction assumption remains the load-bearing
uncertainty; do not model the two-track as solving it.

## 18. Payout-moving vs difficulty-moving (2026-10-04, Jev eval, owner's question)

**Question:** is it cleaner to move the difficulty bar (Bitcoin-style, fixed
reward, next-char target T floats) instead of moving the payout (odds fixed)?
Eval: `~/workspace/jev-experiment/eval_walletmining_adjustment_jev.mjs`.

**Jev: A_payout 0.85** — keep the current design. Legibility: A_faster 0.93
(agents compute EV as static_odds x quoted multiplier). Death vector is an
honest three-way split: multiplier_distrust 0.30, dilution_spiral 0.28,
near_miss_rage 0.27. **Confidence only 0.97/3** — Jev prefers payout-moving
for agent legibility but does not claim it is clearly the better design.
EV/hour is identical in expectation either way; the difference is feel and
manifest arithmetic.
