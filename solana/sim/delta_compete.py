#!/usr/bin/env python3
"""Delta run: competing-EV calibration sensitivity.
LOCAL ONLY. Replaces the $1.00/GPU-h GUESS with researched ranges:
  - $0.25: Render-like DePIN yields ($3-7/day per 4090), vast.ai host income low end
  - $0.50: vast.ai host income mid ($0.20-0.60/hr), 50%-utilization blends
  - $1.00: original conservative guess (kept as pessimistic case)
D2 config (risk-averse), sell_frac {0.5, 0.7}, fee 0.1, 5 seeds/cell.
"""
import json
import sys
sys.path.insert(0, '/home/hatch/workspace/wallet-mining-solana/sim')
import sim as S

BASE = {'payout': 2400, 'half_life': 30, 'tiers': 'mid',
        'seed_sol': 20, 'wall_days': 14, 'risk_averse': True,
        'refine_fee': 0.10}

rows = []
print(f"{'compete':<8} {'sell':<5} {'catch':<7} {'comp_d':<7} {'fin_min':<8} "
      f"{'px':<7} {'mean_sell':<10}")
for cm in (0.25, 0.50, 1.00):
    S.COMPETE_MEDIAN = cm
    for sell in (0.5, 0.7):
        catches, cds, fms, pxs, mss = 0, [], [], [], []
        for seed in range(5):
            cfg = dict(BASE, sell_frac=sell)
            m = S.run(cfg, seed=seed)
            catches += m['catches']
            cds.append(m['competitive_days'])
            fms.append(m['final_miners'])
            pxs.append(m['price_end_ratio'])
            mss.append(m['mean_sell'])
        row = {'compete_median': cm, 'sell_frac': sell,
               'catches': f"{catches}/5",
               'comp_days_mean': round(sum(cds) / 5, 1),
               'final_miners_mean': round(sum(fms) / 5, 1),
               'price_end_mean': round(sum(pxs) / 5, 2),
               'mean_sell': round(sum(mss) / 5, 3)}
        rows.append(row)
        print(f"{cm:<8} {sell:<5} {row['catches']:<7} {row['comp_days_mean']:<7} "
              f"{row['final_miners_mean']:<8} {row['price_end_mean']:<7} "
              f"{row['mean_sell']:<10}")

S.COMPETE_MEDIAN = 1.00  # restore
with open('/home/hatch/workspace/wallet-mining-solana/sim/results/delta_compete.json', 'w') as fh:
    json.dump(rows, fh, indent=1)
print("\nwrote sim/results/delta_compete.json")
