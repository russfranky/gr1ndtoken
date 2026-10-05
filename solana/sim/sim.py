#!/usr/bin/env python3
"""
Agent-adoption simulation for the Solana wallet-mining anon launch.
LOCAL ONLY. Not committed anywhere.

Model: N scraping agents, each with a daily GPU-hour budget and a set of
competing opportunities. Each simulated day, agents read the manifest
(net EV/hour of mining), allocate to the best opportunity with inertia,
mine (Poisson hits per tier), and token price updates from miner selling,
the launcher buy wall, and speculative demand driven by attention.

Every constant below is documented as MEASURED / DERIVED / GUESS.
GUESS items are flagged for the Jev assumption-attack step.
"""

import numpy as np

# ---------------------------------------------------------------- constants
SOL_USD = 120.0            # DERIVED simplification: SOL held constant (flagged)
CLAIM_COST_SOL = 0.002005  # MEASURED-ish: rent-exempt PDA ~0.002 SOL + 0.000005 fee
CLAIM_COST_USD = CLAIM_COST_SOL * SOL_USD   # ~= $0.24 per claim at $120 SOL

GRIND_RATE = 3e6           # GROUNDED 2026-10-04: tier-scoring workload on 3090-class GPU.
                           # Published: 44M/s suffix-search w/ mod-58^K prefilter
                           # (alhimikix/solana-suffix-gpu) — NOT our workload (it skips
                           # base58 for non-matches). Same source, comb version
                           # (prefix extraction per key, like our tier scoring): 4.65M/s.
                           # 3M is conservative for lower-end cards.
                           # CPU measured here: 32.7k/s/core (libsodium); published
                           # 28-42k/thread. An 8-core desktop ~= 0.1 GPU-hour equiv.
                           # Load-bearing: scales all hit rates linearly.

N_AGENTS = 200             # GUESS: population of opportunity-scraping agents
DAYS = 30                  # trial window from LAUNCH-PLAN.md
K_OPPS = 5                 # GUESS: competing opportunities each agent watches
INERTIA = 0.10             # GUESS: switch only if new best beats current by >10%
P0 = 0.001                 # reference token price USD (launcher seed-buy price)

COMPETE_MEDIAN = 1.00       # GUESS, LOAD-BEARING: median $/GPU-hour of generic
COMPETE_SIGMA = 0.7         #   crypto-agent tasks (range pick $0.50-2.00).
BUDGET_MEDIAN = 2.0         # GUESS: median daily GPU-hours per agent
BUDGET_SIGMA = 1.2          # heavy-tailed: a few whales

SPEC_K = 2000.0            # GUESS (toy): speculative USD buy per attention-unit
                           #   per unit of positive price momentum
SEED_MOMENTUM = 0.05       # GUESS: announcement pop on day 0
SEED_GRINDERS = 3          # embargoed independent grinders (plan section 4)
SEED_GPUH_EACH = 16.0      # GUESS: seed grinding effort per grinder per seed-day
SEED_DAYS = 2

TOTAL_SUPPLY = 1e9         # nominal fixed supply (only a fraction emits in trial)

# ---------------------------------------------------------------- tiers
def hits_per_gpuh(n_chars):
    """Expected tier hits per GPU-hour. 1-in-58^n per keypair (DERIVED)."""
    return 3600.0 * GRIND_RATE / (58.0 ** n_chars)

# (leading chars, payout multiple). Payouts are difficulty-proportional:
# payout/hit(n) = P_total * mult_n / W  with  W = sum(mult*hitrate),
# so day-0 expected tokens per GPU-hour == P_total across all tier sets.
TIER_SETS = {
    'diffuse':  [(4, 1), (5, 58), (6, 58 ** 2)],
    'mid':      [(5, 1), (6, 58)],
    'jackpot':  [(6, 1), (7, 58)],
}

def tier_table(name):
    tiers = []
    W = sum(m * hits_per_gpuh(n) for n, m in TIER_SETS[name])
    for n, m in TIER_SETS[name]:
        tiers.append({'chars': n, 'mult': m, 'hitrate': hits_per_gpuh(n),
                      'share': m * hits_per_gpuh(n) / W})
    return tiers, W

# ---------------------------------------------------------------- one run
def run(cfg, seed=0):
    """
    cfg keys: payout (day-0 expected tokens/GPU-h), half_life (days),
              tiers (name), seed_sol (launcher SOL budget B),
              wall_days, sell_frac.
    Returns dict of metrics.
    """
    rng = np.random.default_rng(seed)
    tiers, W = tier_table(cfg['tiers'])
    payout0 = cfg['payout']
    half_life = cfg['half_life']
    B = cfg['seed_sol']
    wall_days = cfg['wall_days']
    sell_frac = cfg['sell_frac']
    f = cfg.get('refine_fee', 0.0)   # GUESS/policy: claim-time refining fee
                                     # diverted to the holder vault (0 = off)
    E_ELAST = cfg.get('sell_elasticity', 0.0)  # GUESS/behavioral: staking-APY
    BASE_SELL = cfg.get('baseline_sell', sell_frac)  # elasticity of holding.
                                     # sell_t = BASE_SELL / (1 + E * apy_smooth).
                                     # E=0 reproduces the old fixed-sell model.

    budgets = np.clip(BUDGET_MEDIAN * np.exp(BUDGET_SIGMA * rng.standard_normal(N_AGENTS)),
                      0.1, 200.0)
    opp_best = np.max(COMPETE_MEDIAN *
                      np.exp(COMPETE_SIGMA * rng.standard_normal((N_AGENTS, K_OPPS))),
                      axis=1)

    price = P0
    # Launcher commits 2*B SOL total (documented): B into LP, B as buy wall.
    # yield_sol (default 0) REALLOCATES part of the wall budget to a
    # SOL-denominated hold subsidy paid pro-rata to stakers over the trial.
    # Same total budget, different split — not new spending.
    YIELD_SOL = cfg.get('yield_sol', 0.0)
    WALL_B = max(B - YIELD_SOL, 0.0)
    lp_usd = 2.0 * B * SOL_USD          # toy initial LP depth
    wall_daily_usd = WALL_B * SOL_USD / wall_days

    tokens_earned = np.zeros(N_AGENTS)
    ever_mined = np.zeros(N_AGENTS, dtype=bool)
    holder_bal = np.zeros(N_AGENTS)  # unclaimed token balances; fee pool accrues here
    carry_pool = 0.0                 # fee accrued while nobody holds yet
    prev_fee_pool = 0.0              # yesterday's fee pool (yield expectation)

    DIFF_TARGET = cfg.get('difficulty_target_gpuh', 0.0)  # GUESS/policy:
        # SMOOTH difficulty adjustment on trailing participation. When >0,
        # per-hit payouts scale by mult_t = clamp(target/trailing_gpuh),
        # smoothed (0.7 old + 0.3 new) to avoid oscillation. More miners ->
        # lower pay per hit -> total emission ~constant (Bitcoin-like), per-
        # miner EV self-equilibrates. Unlike the hard budget cap (tried and
        # rejected 2026-10-04: it removed the supply response that dampened
        # reflexive moons, causing boom-bust), this keeps a continuous,
        # differentiable response: no cliffs, no tier-activation cascades.

    _diff_mult = 1.0
    _trailing_gpuh = []

    def diff_mult_today(gpu_h_today):
        """Update and return today's difficulty multiplier."""
        nonlocal _diff_mult
        _trailing_gpuh.append(gpu_h_today)
        if DIFF_TARGET > 0 and len(_trailing_gpuh) >= 1:
            trailing = float(np.mean(_trailing_gpuh[-7:]))
            # seed-phase nominal if trailing is ~0 (avoid div-by-zero blowup)
            raw = DIFF_TARGET / max(trailing, 1.0)
            raw = float(np.clip(raw, 0.05, 20.0))
            _diff_mult = 0.7 * _diff_mult + 0.3 * raw
        return _diff_mult

    def pay_table(t, dmult=1.0):
        """Per-hit payouts for day t. Returns (pay_net, pay_gross, decay)."""
        decay = 0.5 ** (t / half_life)
        pay_net, pay_gross = {}, {}
        for tr in tiers:
            n = tr['chars']
            gross = payout0 * (tr['mult'] / W) * decay * dmult
            pay_gross[n] = gross
            pay_net[n] = gross * (1.0 - f)
        return pay_net, pay_gross, decay

    def mining_stats(px, pay_net, pay_gross, budgets_arr=None, total_held=0.0,
                     fee_pool_day=0.0, sell_t=None, sol_usd_per_tok_day=0.0):
        """(ev_per_agent_usd, expected_tokens_per_gpuh) over PROFITABLE tiers only.
        A tier is skipped by rational agents when payout < claim cost.
        pay_hit is NET of the refining fee. Rational agents add the expected
        fee-vault yield on the tokens they will hold to their EV.
        If cfg['risk_averse'] (Jev assumption-attack refinement): each agent
        additionally ignores tiers it will likely never hit in the trial
        (expected hits over budget/day * DAYS < 1). Jackpot tiers have extreme
        variance: a 2 GPU-h/day agent expects a 7-char hit once per ~60 days."""
        risk_averse = cfg.get('risk_averse', False)
        prof = []
        for tr in tiers:
            n = tr['chars']
            pay_hit = pay_net[n]           # net of fee, from pay_table
            usd_hit = pay_hit * px - CLAIM_COST_USD
            if usd_hit > 0:
                prof.append((tr, pay_hit, usd_hit))
        tok = sum(tr['hitrate'] * pay_hit for tr, pay_hit, _ in prof)      # net
        tok_gross = sum(tr['hitrate'] * pay_gross[tr['chars']]             # pre-fee
                        for tr, _, _ in prof)
        # Expected fee-vault yield per GPU-h for an agent holding at the
        # population rate: held tokens earn fee_pool/total_held per day for
        # ~1/sell_frac expected days (geometric exit). In steady state this
        # rebates (1-sell_frac)*f of gross per GPU-h: the fee punishes dumping
        # more than holding, which is the intended gradient. The per-token
        # daily yield is capped (a transient small-holder-base spike is not
        # a sustainable expectation; rational agents anticipate dilution).
        yld = 0.0
        st = sell_t if sell_t is not None else sell_frac
        if total_held > 0 and 0.0 < st < 1.0:
            if f > 0 and fee_pool_day > 0:
                yld_per_token_day = min(fee_pool_day / total_held, 0.02)
                yld += (tok * (1.0 - st) * yld_per_token_day
                        * (1.0 / st) * px)
            # SOL-denominated hold subsidy: real cash yield, no cap — it is
            # the legible part of the hold return (denominated in SOL, not
            # the volatile token). Expected holding duration 1/st days.
            if sol_usd_per_tok_day > 0:
                yld += tok * (1.0 - st) * sol_usd_per_tok_day * (1.0 / st)
        if not risk_averse or budgets_arr is None:
            ev = sum(tr['hitrate'] * usd_hit for tr, _, usd_hit in prof) + yld
            return np.full(N_AGENTS, ev), tok, [p[0] for p in prof], tok_gross
        horizon = budgets_arr * DAYS          # GPU-hours per agent over trial
        ev_agent = np.zeros(N_AGENTS)
        for tr, _, usd_hit in prof:
            eligible = horizon * tr['hitrate'] >= 1.0
            ev_agent[eligible] += tr['hitrate'] * usd_hit
        ev_agent += yld
        return ev_agent, tok, [p[0] for p in prof], tok_gross

    # embargoed seed phase: independent grinders mine before day 0 (plan sec. 4)
    seed_pay_net, seed_pay_gross, _ = pay_table(0)
    seed_tok, seed_claims = 0.0, 0
    ev0, tok0, prof0, _ = mining_stats(P0, seed_pay_net, seed_pay_gross, budgets)
    for tr in prof0:
        lam = SEED_GRINDERS * SEED_GPUH_EACH * SEED_DAYS * tr['hitrate']
        c = rng.poisson(lam)
        seed_claims += c
        seed_tok += c * seed_pay_net[tr['chars']]
    # launcher buys seed tokens at P0 (buyer of first resort); not resold in sim
    # seed the difficulty trailing average with nominal seed effort
    for _ in range(SEED_DAYS):
        _trailing_gpuh.append(SEED_GRINDERS * SEED_GPUH_EACH)

    # Discovery ramp: the manifest must be FOUND. Agents become aware over
    # time (logistic); unaware agents cannot mine. discovery_days=0 (default)
    # reproduces the old instant-awareness model. Tests how slow discovery
    # can be before the loop dies — gradual entry may actually HELP by
    # avoiding a day-0 emission flood.
    DISC_DAYS = cfg.get('discovery_days', 0.0)
    if DISC_DAYS > 0:
        disc_day = np.clip(DISC_DAYS / 2 + (DISC_DAYS / 6) * rng.logistic(
            size=N_AGENTS), 0, DAYS)
    else:
        disc_day = np.zeros(N_AGENTS)

    alloc = np.zeros(N_AGENTS, dtype=int)  # 0 = mine, 1 = best competing opp
    pn0, pg0, _ = pay_table(0, _diff_mult)
    ev_mine, _, _, _ = mining_stats(P0, pn0, pg0, budgets)
    ev_mine = np.where(disc_day <= 0, ev_mine, -np.inf)  # only aware mine
    alloc = np.where(ev_mine >= opp_best, 0, 1)

    hist = {'price': [], 'miners': [], 'claims': [], 'competitive_frac': []}
    prev_price = P0 / (1.0 + SEED_MOMENTUM)  # so day-0 momentum = SEED_MOMENTUM
    apy_hist = []          # observed staking APY series (for elasticity)
    sol_apy_hist = []      # SOL-subsidy APY-equivalent series
    sell_series = []       # realized daily sell fraction (for reporting)
    gpu_h_prev = SEED_GRINDERS * SEED_GPUH_EACH  # difficulty input for day 0

    for t in range(DAYS):
        total_held = float(holder_bal.sum())
        # Endogenous selling: agents observe the staking APY (7-day smoothed,
        # from yesterday's fee pool and holder base) and sell less when it is
        # high. sell_t = BASE_SELL / (1 + E * apy). E=0 -> old fixed model.
        if E_ELAST > 0 and total_held > 0 and prev_fee_pool > 0:
            apy_now = prev_fee_pool / total_held * 365.0
        else:
            apy_now = 0.0
        apy_hist.append(apy_now)
        apy_smooth = float(np.mean(apy_hist[-7:]))
        # SOL-denominated hold subsidy, expressed as APY-equivalent so it
        # feeds the same elasticity: sol_apy = (daily SOL per staked token
        # / price) * 365. Denominated in SOL, it is the legible part of the
        # hold return — an ROI agent can compare it directly to competing
        # opportunities without modeling token volatility.
        if YIELD_SOL > 0 and total_held > 0:
            sol_usd_per_tok_day = (YIELD_SOL * SOL_USD / DAYS) / total_held
            sol_apy = sol_usd_per_tok_day / price * 365.0
        else:
            sol_usd_per_tok_day = 0.0
            sol_apy = 0.0
        sol_apy_hist.append(sol_apy)
        incentive = apy_smooth + sol_apy
        sell_t = BASE_SELL / (1.0 + E_ELAST * incentive)
        sell_series.append(sell_t)
        # Difficulty: today's multiplier from trailing participation
        # (yesterday's gpu_h and before). Slow-moving; agents take it as given.
        dmult = diff_mult_today(gpu_h_prev)
        pay_net, pay_gross, _ = pay_table(t, dmult)
        ev_mine, tok_per_gpuh, prof, tok_gross = mining_stats(
            price, pay_net, pay_gross, budgets, total_held, prev_fee_pool,
            sell_t, sol_usd_per_tok_day)
        ev_mine = np.where(disc_day <= t, ev_mine, -np.inf)  # unaware sit out

        # re-evaluate with inertia
        cur_ev = np.where(alloc == 0, ev_mine, opp_best)
        best_ev = np.maximum(ev_mine, opp_best)
        want = np.where(ev_mine >= opp_best, 0, 1)
        switch = (want != alloc) & (best_ev > cur_ev * (1.0 + INERTIA))
        alloc = np.where(switch, want, alloc)

        miners = alloc == 0
        n_miners = int(miners.sum())
        ever_mined |= miners
        gpu_h = float(budgets[miners].sum())

        claims_t = 0
        tokens_mined_gross = 0.0
        for tr in prof:
            lam = gpu_h * tr['hitrate']
            c = int(rng.poisson(lam)) if lam > 0 else 0
            claims_t += c
            tokens_mined_gross += c * pay_gross[tr['chars']]
            # attribute tokens proportional to budget
        fee_day = f * tokens_mined_gross
        liquid_mined = tokens_mined_gross - fee_day
        gpu_h_prev = gpu_h   # feeds tomorrow's difficulty update
        if gpu_h > 0:
            liquid_earned = budgets[miners] / gpu_h * liquid_mined
            tokens_earned[miners] += liquid_earned
            # holders = miners keeping (1-sell_t); tracked so the fee
            # pool can accrue pro-rata (same never-exit optimism as baseline)
            holder_bal[miners] += (1.0 - sell_t) * liquid_earned
        # fee pool distributed pro-rata to pre-existing holder balances
        if total_held > 0:
            holder_bal += holder_bal / total_held * (fee_day + carry_pool)
            carry_pool = 0.0
        else:
            carry_pool += fee_day

        sell_usd = sell_t * liquid_mined * price
        prev_fee_pool = fee_day
        momentum = price / prev_price - 1.0
        attention = (np.log10(1 + claims_t + (seed_claims if t == 0 else 0)) *
                     np.log10(1 + n_miners + (SEED_GRINDERS if t == 0 else 0)))
        spec_usd = SPEC_K * attention * max(0.0, momentum)
        buy_usd = (wall_daily_usd if t < wall_days else 0.0) + spec_usd

        prev_price = price
        price = price * (1.0 + (buy_usd - sell_usd) / lp_usd)
        price = float(np.clip(price, P0 * 0.02, P0 * 200.0))
        lp_usd = lp_usd + 0.1 * (buy_usd + sell_usd)  # volume deepens LP (toy)

        comp_frac = float(np.mean(ev_mine >= opp_best))
        hist['price'].append(price)
        hist['miners'].append(n_miners)
        hist['claims'].append(claims_t)
        hist['competitive_frac'].append(comp_frac)

    miners_arr = np.array(hist['miners'])
    comp_arr = np.array(hist['competitive_frac'])
    peak = int(miners_arr.max())
    final_miners = float(miners_arr[-3:].mean())
    competitive_days = int((comp_arr > 0.25).sum())
    price_end = hist['price'][-1]

    # Gini across miners that earned anything
    e = tokens_earned[ever_mined]
    if e.size > 1 and e.sum() > 0:
        es = np.sort(e)
        gini = (2 * np.arange(1, len(es) + 1) @ es) / (len(es) * es.sum()) - (len(es) + 1) / len(es)
    else:
        gini = 0.0

    catches = bool(final_miners >= 0.25 * max(peak, 1)
                   and price_end >= 0.3 * P0
                   and competitive_days >= 8)

    return {
        'competitive_days': competitive_days,
        'peak_miners': peak,
        'final_miners': round(final_miners, 1),
        'total_claims': int(sum(hist['claims'])) + seed_claims,
        'unique_miners': int(ever_mined.sum()),
        'gini': round(float(gini), 3),
        'price_end_ratio': round(price_end / P0, 3),
        'catches': catches,
        'tokens_emitted': round(float(tokens_earned.sum() + seed_tok), 0),
        'mean_sell': round(float(np.mean(sell_series)), 3),
        'mean_apy': round(float(np.mean(apy_hist)), 3),
        'mean_sol_apy': round(float(np.mean(sol_apy_hist)), 3),
    }


if __name__ == '__main__':
    import sys, json
    # quick manual calibration: one config, print trajectory
    cfg = {'payout': 9600, 'half_life': 7, 'tiers': 'diffuse',
           'seed_sol': 20, 'wall_days': 7, 'sell_frac': 0.7}
    if len(sys.argv) > 1:
        cfg = json.loads(sys.argv[1])
    m = run(cfg, seed=0)
    print(json.dumps(m, indent=1))
