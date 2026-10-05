#!/usr/bin/env python3
"""Delta run: SOL-denominated hold subsidy (reallocation of launcher budget).
LOCAL ONLY. yield_sol SOL of the 20 SOL budget moves from buy wall to a
pro-rata hold subsidy; its APY-equivalent feeds the same elasticity.
D2 config (risk-averse), baseline_sell=0.7, fee=0.1, 5 seeds/cell.
"""
import json
import sys
sys.path.insert(0, '/home/hatch/workspace/wallet-mining-solana/sim')
from sim import run

BASE = {'payout': 2400, 'half_life': 30, 'tiers': 'mid',
        'seed_sol': 20, 'wall_days': 14, 'risk_averse': True,
        'baseline_sell': 0.7, 'sell_frac': 0.7, 'refine_fee': 0.10}

# backward-compat: yield_sol=0 must match delta_elastic E=2/base=0.7/fee=0.1 cell
san = run(dict(BASE, sell_elasticity=2.0, yield_sol=0.0), seed=0)
print('sanity:', {k: san[k] for k in ('catches', 'mean_sell', 'mean_sol_apy')})

rows = []
print(f"{'yield_sol':<10} {'E':<5} {'catch':<7} {'comp_d':<7} {'fin_min':<8} "
      f"{'px':<7} {'mean_sell':<10} {'sol_apy':<8}")
for ys in (0, 5, 10):
    for E in (2, 5, 10):
        catches, cds, fms, pxs, mss, sas = 0, [], [], [], [], []
        for seed in range(5):
            cfg = dict(BASE, sell_elasticity=float(E), yield_sol=float(ys))
            m = run(cfg, seed=seed)
            catches += m['catches']
            cds.append(m['competitive_days'])
            fms.append(m['final_miners'])
            pxs.append(m['price_end_ratio'])
            mss.append(m['mean_sell'])
            sas.append(m['mean_sol_apy'])
        row = {'yield_sol': ys, 'E': E, 'catches': f"{catches}/5",
               'comp_days_mean': round(sum(cds) / 5, 1),
               'final_miners_mean': round(sum(fms) / 5, 1),
               'price_end_mean': round(sum(pxs) / 5, 2),
               'mean_sell': round(sum(mss) / 5, 3),
               'mean_sol_apy': round(sum(sas) / 5, 2)}
        rows.append(row)
        print(f"{ys:<10} {E:<5} {row['catches']:<7} {row['comp_days_mean']:<7} "
              f"{row['final_miners_mean']:<8} {row['price_end_mean']:<7} "
              f"{row['mean_sell']:<10} {row['mean_sol_apy']:<8}")

with open('/home/hatch/workspace/wallet-mining-solana/sim/results/delta_sol_yield.json', 'w') as fh:
    json.dump(rows, fh, indent=1)
print("\nwrote sim/results/delta_sol_yield.json")
