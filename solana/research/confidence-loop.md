# Confidence Loop Log — Solana wallet-mining anon launch

**Goal:** maximize Jev's confidence in the design via iterate(sim refine / design tweak -> Jev re-eval).
**Stop rule:** max 6 iterations, OR two consecutive iterations each improving headline confidence by < 0.15, OR Jev's top blocker is "requires live/real-world data" twice running.
**Confidence question (identical wording every round):** "How confident are you that THIS design, as specified, achieves sustained utilization — i.e. the agent-driven feedback loop catches and sustains through the 30-day trial with real opportunity-scraping agents? Judge the design itself, not the quality of the simulation."

**Baselines (pre-loop):**
- Launch plan achieves real utilization: 1.46/3
- Config pick: 0.17/3 (0.20 on "trust none of these")
- Hold incentive moves hold rates to ~70%: 1.01/3
- Best sim: D2-style (payout 2400/GPU-h, hl 30d, mid tiers, B=20, wall=14d) catches 6/10 at sell_frac=0.5, with or without the 10% fee; everything dies at 0.7.

---
## Iteration 0 — baseline (2026-10-04)
- **Design:** D2-style (payout 2400/GPU-h @ $0.001, hl 30d, mid tiers) + 10% claim refining fee + staking vault + manifest-first launch, no referrals.
- **Sim evidence in state:** catches 6/10 at sell_frac=0.5 with/without fee; dies at 0.7; prior scores 1.46 / 0.17 / 1.01.
- **Jev confidence: 0.48/3** (Low 0.52, Moderate 0.47). **Blocker: sell_behavior (1.0 unanimous).** Highest-leverage move: live_pilot (0.82) — unavailable without launching; taking next: stronger hold incentive (0.16).
- Eval: `~/workspace/jev-experiment/eval_walletmining_confloop_0_jev.mjs` (+ `_results.json`)
## Iteration 1 — behavioral elasticity (2026-10-04)
- **Change:** endogenous selling (sell_t = BASE_SELL/(1+E*apy_smooth)); swept E in {0..50} x baseline {0.6,0.7} x fee {0.1,0.2} on D2, 5 seeds. Ore v3 Fermi: E~=22 implied (81% locked, 17% APY, baseline 0.9).
- **Sim:** E~=5 needed for reliable catch at 60-70% baseline selling (4/5 at E=5; <=2/5 at E<=2). Fee without behavioral response is pure drag (fee=0.2 worse than 0.1 at E=0). Fee level barely matters once E>=5.
- **Jev confidence: 0.28/3** (Low 0.72) — DOWN 0.20. Quantifying the blocker made the gap concrete. Mechanism: A_statusquo 0.39, D_buyback_split 0.32, C_loyalty 0.27. Elasticity plausibility: discounted 0.80 (Ore analogy weak). **Blocker: sell_behavior (0.95).**
- Eval: `~/workspace/jev-experiment/eval_walletmining_confloop_1_jev.mjs` (+ `_results.json`)
## Iteration 2 — smooth difficulty adjustment (2026-10-04)
- **Change:** STRUCTURAL. Replaced fixed per-GPU-h payouts with smooth difficulty adjustment (per-hit pay x clamp(target/trailing_gpuh), 0.7/0.3 smoothed). Rejected along the way: hard epoch budget cap (boom-bust), naive per-hit retarget on yesterday's hits (explodes on 0->N), SOL-denominated hold subsidy (reflexive boom-bust, no catch-rate gain). Also grounded competing-EV in real data: $0.25-0.50/hr (vast.ai $0.29-0.59/hr 4090 rental; Render $3-7/day; hosts $0.20-0.60/hr) replacing the $1.00 guess.
- **Sim (sim/delta_difficulty.py):** anti-scaling inversion GONE. With difficulty: $0.25 bar -> 2/5 (sell .5) and 4/5 (sell .7); $0.50 -> 3/5 and 4/5; $1.00 -> 3/5 and 0/5. Lower bars now catch better, as a viral loop needs. Target not knife-edge (200->3/5, 400->4/5, 800->5/5).
- **Jev confidence: 1.38/3** (Moderate 0.54, High 0.40) — UP 1.10, biggest single jump. Difficulty verdict: yes_as_specified (1.0 UNANIMOUS). **Blocker: sell_behavior (0.76)** — still top, but down from 1.0/0.95.
- Eval: `~/workspace/jev-experiment/eval_walletmining_confloop_2_jev.mjs` (+ `_results.json`)
## Iteration 3 — optimized config on the difficulty ridge (2026-10-04)
- **Change:** fine joint sweep (payout x hl x diff_target, 18 configs x 5 seeds, $0.25 bar, 70% selling) + discovery-ramp realism check (logistic awareness).
- **Sim (sim/delta_fine.py):** WINNER payout=1200, hl=45d, diff_target=800: 5/5 catch, 26.8 comp days, 195 final miners, 4.4x price, gini 0.58. Ridge: 1200-2400 all 4-5/5; 4800 dies everywhere (death-spiral corridor). Robustness: $0.25->5/5 and $0.50->4/5 at 70% selling; only pessimistic $1.00+70% dies (0/5). Discovery: 7d ramp 5/5, 21d ramp 5/5 (14d cell 2/5, likely noise).
- **Jev confidence: 1.47/3** (Moderate 0.51, High 0.46) — UP 0.09 (<0.15: strike one). **Blocker: sell_behavior (1.0 unanimous, third round).**
- Eval: `~/workspace/jev-experiment/eval_walletmining_confloop_3_jev.mjs` (+ `_results.json`)
## Iteration 4 — deeper buyer-of-last-resort (2026-10-04)
- **Change:** launcher budget 20 -> 50 SOL (~$6,000) to fix the residual hard cell (pessimistic $1.00 bar + 70% selling: 0/5 -> 3/5). Wall duration irrelevant; depth matters.
- **Sim:** B=50 matrix: $0.25 -> 5/5, 5/5; $0.50 -> 5/5, 5/5; $1.00 -> 4/5, 3/5. Every realistic cell catches reliably.
- **Jev confidence: 0.92/3** (Low 0.73) — DOWN 0.55. Reading: deepening the subsidy raised sim robustness but lowered belief it is a self-sustaining viral loop rather than a bought one. Optimizing for the pessimistic cell looks like overfitting to the sim.
- **Blocker: sell_behavior (0.95, fourth round).**
- Eval: `~/workspace/jev-experiment/eval_walletmining_confloop_4_jev.mjs` (+ `_results.json`)

## STOP — rule fired: two consecutive iterations with improvement < 0.15 (r3: +0.09, r4: -0.55). Max 6 not reached. "Needs live data" never became the top blocker (max 0.08).

## Trajectory
| Round | Change | Confidence | Delta | Blocker |
|---|---|---|---|---|
| 0 | baseline (D2 + 10% fee) | 0.48/3 | — | sell_behavior (1.0) |
| 1 | behavioral elasticity model (E~=5 needed; Ore Fermi E~=22) | 0.28/3 | -0.20 | sell_behavior (0.95) |
| 2 | SMOOTH DIFFICULTY ADJUSTMENT (fixed anti-scaling) + real competing-bar data | 1.38/3 | +1.10 | sell_behavior (0.76) |
| 3 | optimized config (1200/45d/dt800) + discovery ramp | 1.47/3 | +0.09 | sell_behavior (1.0) |
| 4 | deeper buyer-of-last-resort (B=50) | 0.92/3 | -0.55 | sell_behavior (0.95) |

**Peak: 1.47/3 at round 3.** Recommended design = round-3 config (B=20, not B=50).
**What moved it:** the difficulty adjustment (+1.10) — the only structural fix; everything else was tuning.
**What didn't:** hold incentives, subsidies, deeper walls — Jev reads subsidies as buying the loop, not earning it.
**Residual blocker:** sell_behavior, four rounds running. The design survives 70% dumping in sim; whether real agents dump more (or dump differently) is unmeasurable without a live pilot.
