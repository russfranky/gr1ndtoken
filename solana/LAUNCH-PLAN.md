# Wallet Mining (Solana) — Anon Launch Plan

**Status:** local-only draft. Do NOT commit or push to the public wallet-mining repo.
**Jev eval:** `~/workspace/jev-experiment/eval_walletmining_launch_jev.mjs`
**Results:** `~/workspace/jev-experiment/eval_walletmining_launch_jev_results.json` (2026-10-04)

## Objective

Not just seen — UTILIZED. The primary user is AI agents that scrape for
opportunities on behalf of owners. The loop must make a scraping agent
conclude this is the best ROI available and act on it autonomously.

## Jev verdicts (open decisions)

| Decision | Verdict | Prob |
|---|---|---|
| Issuance model | **A: pure fair launch** — 100% via claims, no dev allocation, no presale (unanimous, 1.0) | 1.00 |
| Primary viral vector | **A: machine-readable manifest** (0.65); D: layered manifest + human amplifier (0.35) | 0.65 |
| Referrer cut | **Exclude** (0.72) — adds Sybil surface without proven need | 0.72 |
| Seed strategy | **B: embargoed independent grinders** (0.56) over fully organic (0.25) | 0.56 |
| Top trust anchor | **Deployer wallet never claims** (0.50); constrained mint schedule (0.25) | 0.50 |
| Likeliest loop break | **EV negative at launch** (0.47); no liquid price discovery (0.38) | 0.47 |
| Biggest underweighted risk | **Emission death spiral** (0.37); undetectable launcher farming (0.29); Sybil claim farms (0.23) | 0.37 |
| Confidence this achieves real utilization | **1.46 / 3** (low-moderate; honest, not high) | — |

Jev is NOT highly confident. The plan below is built around the two break
points Jev named: day-one EV and price discovery.

## The plan

### 1. Issuance — pure fair launch (Jev unanimous)
- Fixed-supply SPL token. 100% emitted through mining claims.
- No presale, no team allocation, no investor unlocks.
- Mint authority held by the program under a programmed emission schedule
  (decaying: aggressive early, fast decay).
- Deployer wallet NEVER claims. On-chain auditable. This is the launch's
  only trust asset — breaking it ends the experiment's credibility.

### 2. Viral vector — the opportunity manifest (Jev 0.65)
A versioned JSON endpoint + mirrored on-chain account publishing:
- Tier table, current payouts (token + USD via DEX price)
- Difficulty per tier, reference grind rates
- **Net expected value per GPU-hour**, claim costs included

Any scraper computes ROI in one fetch and ranks this against every other
opportunity it watches. Payouts and difficulty are on-chain, so no trust in
the launcher is required to verify the numbers.

Human amplifier (the 0.35 for layered): a trophy leaderboard of rarest
claims derived purely from on-chain data — no backend, nothing to trust.
No referrer cut (Jev 0.72 exclude).

### 3. The feedback loop
Agent fetches manifest -> EV positive -> grinds -> claims on-chain (public
tx) -> claim volume is on-chain legible -> indexers/scrapers pick up rising
claim counts -> price discovery -> higher USD payouts -> higher EV -> more
agents join -> deeper distribution -> tracker listings -> human attention
follows machine attention -> more agents.

The real viral vector is OWNER ROI: agents reporting earnings to owners who
allocate more compute. Not social posts.

### 4. Seed — embargoed independent grinders (Jev 0.56)
- Recruit 2–3 independent grinders under embargo pre-announcement.
- They produce the first real claims, proving the loop end-to-end and
  populating on-chain history before the manifest goes public.
- Their wallets are disclosed as seed wallets and retired after.

### 5. Solving Jev's #1 break point: day-one EV + price discovery
A manifest with theoretical EV and no liquid price loses to every real
opportunity. The launcher acts as **buyer of first resort**:
- Launcher commits a fixed SOL budget to buy tokens from early miners at a
  published reference price, then seeds a DEX liquidity pool with the
  acquired tokens + SOL.
- This gives grinders their first exit, creates a real USD price on day
  one, and makes the manifest's EV numbers honest instead of theoretical.
- The SOL spend is the experiment's customer-acquisition cost. It is
  bounded, disclosed in the plan, and buys no allocation — only market
  tokens like anyone else.

### 6. Answering the death-spiral risk (Jev 0.37, top risk)
Fixed token payouts collapse in USD terms if price drops -> EV dies ->
grinding stops -> volume dies -> price dies further. Mitigations:
- **Bounded trial window** (e.g. 30-day emission schedule): this is an
  experiment, not a perpetual economy. The spiral has a floor because the
  emission ends.
- Front-loaded decaying emission: highest rewards while attention is
  highest; late-stage payouts are small by design.
- Diffuse rewards (many small tiers, not few jackpots): no single farmer —
  including the launcher — captures disproportionate value.

### 7. Answering launcher-farm suspicion (Jev 0.29)
"Deployer never claims" is theater if the launcher mines from fresh
wallets. Stated plainly in the plan:
- It cannot be prevented, only made pointless: diffuse tier rewards +
  per-identity claim velocity caps (from the 2026-10-03 identity verdict:
  cheap gates at registration, Orb only at high-value claims) mean farming
  scales linearly with cost, with no insider edge.
- The launcher's disclosed role is buyer/LP-seeder, never miner.

### 8. Sybil at launch (Jev 0.23)
Ship the cheap identity gates from day one (device attestation + rate
limits + velocity tripwires). Biometric gates stay reserved for high-value
claims per the prior verdict. A claim farm capturing emission before
honest grinders arrive is the failure mode to watch in week one.

## Sequencing
1. Silent mainnet deploy + closed testing (own grinding, claim path,
   manifest accuracy).
2. Embargoed grinders seed first claims.
3. Launcher buys seed tokens at reference price, seeds DEX LP.
4. Public manifest + announcement in agent-dense channels.
5. Monitor: manifest fetches, claims/day, unique claimants, distribution
   Gini, grind-rate estimates, USD EV per GPU-hour.

## Kill criteria (bounded experiment)
- Kill if: USD EV per GPU-hour stays below the top-3 competing
  opportunities for 7 consecutive days after LP seeding, OR claim
  centralization (top 5 wallets > 60% of claims) persists past week two
  despite caps, OR a critical program bug is found (only if deployed
  --final/immutable = no patch path; upgradeable deploys keep a patch
  path at the cost of the trust anchor — red-team C3).
- On kill: publish a post-mortem locally. Emission schedule ends on its
  own; no intervention needed.

## Open items (need Russ)
- SOL budget for buyer-of-first-resort + LP seeding.
- Trial window length (30 days proposed).
- Reference price for seed buys.
- Grinder recruitment (who, how many, embargo terms).
- Go/no-go on immutable program (no upgrade authority = no patch path).

## Simulation update (2026-10-04)

An agent-adoption sim (200 scraping agents, 30-day window, 972 runs over
324 configs) plus two Jev evals. Full results: `SIM-RESULTS.md`, code in
`sim/`. What it changes in this plan:

- **Emission decay: plan intuition was inverted.** Section 6 says
  "aggressive early, fast decay." The sim says half-life 7d → 0/108
  configs sustain; 14–30d → the only winners. Decaying payouts collapse
  EV faster than price appreciation compensates. Recommendation: slow
  decay (14–30d half-life), not fast.
- **SOL budget: B=20 is enough.** 50 SOL performs barely better. The
  launcher commitment stays small; the budget is not the lever — miner
  sell behavior is.
- **Tier choice is now conditional.** Risk-neutral agents → jackpot tiers
  (6–7 chars) win on EV efficiency (claim costs truncate low tiers).
  Hit-frequency-sensitive agents → diffuse tiers win (the 5-char tier
  activates mid-trial as price crosses the claim-cost floor). Unmeasured;
  needs real agent-behavior data.
- **Viable payout corridor is narrow:** 2400–9600 tokens/GPU-h at the
  $0.001 reference price. Below it the loop never starts; above it the
  emission death-spirals (Jev's top risk, now quantified).
- **Load-bearing unknowns the sim cannot resolve:** (1) will mining
  agents hold ~70% of rewards vs selling into owner ROI — nothing sustains
  at 70% immediate-sell; (2) the $1.00/GPU-h competing-EV calibration is
  a guess that sets the entire bar. Both need real data (see
  SIM-RESULTS.md section 9).
- **Jev confidence stayed Low (0.17/3)** on the config pick and gives
  1-in-5 that no simulated config should be trusted. Treat sim numbers
  as directional.

## Open item added (2026-10-04): token utility
- Jev recommends the claim-to-mint burn (trophy NFTs, ~2% tier-scaled burn)
  as the token's item-economy utility, but with Low confidence (0.47/3) —
  its death vector is that nobody mints. Needs Russ's call: include the
  burn+mintage path in the immutable program at deploy, or ship the token
  as pure reward + staking yield. Full analysis:
  `research/token-utility.md`.
