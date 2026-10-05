# Token Utility — Medium of the Mined-Item Economy

**Status:** local-only draft. Do NOT commit or push.
**Jev eval:** `~/workspace/jev-experiment/eval_walletmining_utility_jev.mjs` (+ `_results.json`), 2026-10-04.

## The problem

The reward token has no priced utility beyond the staking vault's fee yield.
The owner's article (wallet-mining-in-gaming) establishes the vision: mined
rare wallets become rare ITEMS/NFTs with real-world value via a gaming-DeFi
bridge — value accrues to the ITEMS. The token is currently disconnected
from that vision. The owner believes holders would hold on speculation of
future utility, but a scraping agent cannot price vague future utility into
its EV calc. The utility must be concrete, on-chain, and manifest-legible.

## Candidate designs

### A. Claim-to-mint burn (trophy NFTs) — JEV'S PICK (0.77)

After a successful claim, the miner may call `mint_item()`: burns a token
fee F (tier-scaled, e.g. 2% of that tier's payout) and mints a compressed
NFT (Bubblegum cNFT, ~0.000005 SOL) certifying the vanity address + tier +
pattern. Transferable; the private key stays with the miner. Minting is
OPTIONAL (trophy flex); the trophy leaderboard ranks minted items above
unminted claims, creating social demand for the burn.

- On-chain cost: one SPL burn + one cNFT mint per item; trivial compute.
- Fair-launch read: uniform, optional, no insider edge; burn is small vs
  payouts.
- Economic effect: deflationary sink proportional to mining success — more
  grinding → more burns → scarcer token.
- Gaming: wash-minting is costly signaling (you pay the burn each time),
  self-limiting.
- Priceability: PARTIAL (Jev 0.9) — the burn sink is observable and
  modelable (burn_per_mint × mint_rate / supply, all on-chain); trophy
  resale value is not priceable (thin market).
- Death vector (Jev 0.68): NOBODY MINTS — trophy demand is speculative; if
  miners don't value the flex, the sink never activates and the utility is
  theoretical.

### B. Stake-to-mine tiers (0.15)

Claiming tiers above a threshold (e.g. 6+ chars) requires the claimer's
stake PDA to hold ≥ X tokens staked (~30 days of median miner payouts;
1-epoch unstake cooldown already in the design). `claim()` does one extra
PDA read; rejects below threshold.

- On-chain cost: one PDA read; trivial.
- Economic effect: holding directly tied to earning power — top-tier EV is
  zero unless you hold.
- Priceability: FULL — required_stake × outside_yield vs boosted tier EV is
  an exact break-even a scraping agent can compute.
- Fair-launch read: DANGEROUS — tokens can be BOUGHT on the DEX and staked
  to unlock tiers. Progression curve for earners, capital gate for buyers.
  Jev flags the pay-to-win READ (0.05) and whale centralization (0.24) as
  the vectors; for an anon launch the narrative risk alone may be fatal.

### C. Marketplace denomination (0.02)

Item-NFTs trade in the token as unit of account; 1% protocol fee in token
on secondary trades flows to the staking vault. Lightest version: off-chain
norm + manifest-quoted floors, no new program code. Heavier: listing PDA +
escrow.

- Fair-launch read: cleanest — no gates, no mining tax.
- Death vector: NO TRADES — entirely downstream of trophy demand that may
  never exist; pre-launch it is pure narrative with zero observable numbers.

### D. None (0.03)

Keep the token as pure reward + staking-vault yield.

## Jev's verdict

| Question | Verdict | Prob |
|---|---|---|
| Utility design | **A: claim-to-mint burn** | 0.77 |
| Biggest death vector | **Nobody mints** (trophy demand speculative) | 0.68 |
| Priceability | **Partially** (sink math modelable, resale not) | 0.90 |
| Confidence it moves hold rates toward ~70% | **0.47/3 — Low** (0.57 on Low, 0.41 on Moderate) | — |

## Honest assessment

Jev picks the burn because it is the only candidate that cannot break the
fair launch: it is optional, uniform, and tiny relative to payouts. But
Jev's confidence is 0.47/3 — it expects the most likely outcome is that
nobody mints and the sink never activates. The burn is a *conditional*
sink: it only works if the trophy economy is real, and nothing in the
design makes the trophy economy real except the leaderboard's social
ranking.

The uncomfortable ranking the eval exposes:
- The most *effective* hold motive (B, stake-to-mine) is the most
  *dangerous* to the launch's credibility (pay-to-win read, whale
  centralization).
- The safest motive (A, burn) is the *weakest* (conditional on demand
  that may not exist).
- The cleanest story (C, marketplace) is pure narrative pre-launch.

Per the task's rule (sim delta only if confidence ≥ 1.0): **no sim delta
was run.** At 0.47/3, modeling the burn as a demand sink would be
precision theater — the sink's activation rate is the entire uncertainty
and no EV model can set it. The sim's honest position stands: the design
survives 70% dumping in simulation on a D2-style config, but whether real
agents hold is unmeasurable without a live pilot.

## What would actually make A work

The burn sink needs trophy demand to be real. Levers, none free:
1. Leaderboard prestige that matters beyond the experiment (the article's
   gaming-DeFi bridge: trophies tradeable/carried into a game context).
2. Mint fee sized so the flex is cheap early (when tokens are cheap) —
   tier-scaled 2% does this automatically.
3. Social proof loop: first mints by embargoed seed grinders, visible on
   the leaderboard at public launch.

Without at least (1), A is decoration. With it, A is the article's vision
made mechanical: the items accrue the value, the token accrues the burn.
