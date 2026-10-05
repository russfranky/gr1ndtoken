#!/usr/bin/env python3
"""Delta run: behavioral elasticity of holding wrt staking APY.
LOCAL ONLY. sell_t = BASE_SELL / (1 + E * apy_smooth).
Question: what elasticity E is needed for the loop to catch at
baseline (no-incentive) selling of 0.6 / 0.7?
D2 config (risk-averse agents), 5 seeds per cell.
"""
import json
import sys
sys.path.insert(0, '/home/hatch/workspace/wallet-mining-solana/sim')
from sim import run

BASE = {'payout': 2400, 'half_life': 30, 'tiers': 'mid',
        'seed_sol': 20, 'wall_days': 14, 'risk_averse': True}

# backward-compat sanity: E=0 must equal fixed sell_frac
san = run(dict(BASE, sell_frac=0.5, refine_fee=0.10, sell_elasticity=0.0), seed=0)
print('sanity E=0:', {k: san[k] for k in ('catches', 'competitive_days', 'final_miners')})

rows = []
print(f"{'E':<6} {'base':<5} {'fee':<5} {'catch':<7} {'comp_d':<7} "
      f"{'fin_min':<8} {'px':<7} {'mean_sell':<10} {'mean_apy':<9}")
for E in (0, 1, 2, 5, 10, 20, 50):
    for base_sell in (0.6, 0.7):
        for fee in (0.10, 0.20):
            catches, cds, fms, pxs, mss, ays = 0, [], [], [], [], []
            for seed in range(5):
                cfg = dict(BASE, sell_frac=base_sell, baseline_sell=base_sell,
                           sell_elasticity=float(E), refine_fee=fee)
                m = run(cfg, seed=seed)
                catches += m['catches']
                cds.append(m['competitive_days'])
                fms.append(m['final_miners'])
                pxs.append(m['price_end_ratio'])
                mss.append(m['mean_sell'])
                ays.append(m['mean_apy'])
            row = {'E': E, 'baseline_sell': base_sell, 'refine_fee': fee,
                   'catches': f"{catches}/5",
                   'comp_days_mean': round(sum(cds) / 5, 1),
                   'final_miners_mean': round(sum(fms) / 5, 1),
                   'price_end_mean': round(sum(pxs) / 5, 2),
                   'mean_sell': round(sum(mss) / 5, 3),
                   'mean_apy': round(sum(ays) / 5, 3)}
            rows.append(row)
            print(f"{E:<6} {base_sell:<5} {fee:<5} {row['catches']:<7} "
                  f"{row['comp_days_mean']:<7} {row['final_miners_mean']:<8} "
                  f"{row['price_end_mean']:<7} {row['mean_sell']:<10} "
                  f"{row['mean_apy']:<9}")

with open('/home/hatch/workspace/wallet-mining-solana/sim/results/delta_elastic.json', 'w') as fh:
    json.dump(rows, fh, indent=1)
print("\nwrote sim/results/delta_elastic.json")
