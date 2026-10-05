#!/usr/bin/env python3
"""Delta run: smooth difficulty adjustment vs the anti-scaling flaw.
LOCAL ONLY. Tests whether difficulty_target_gpuh fixes the finding that
LOWER competing bars catch WORSE (success killed the loop).
D2 config (risk-averse), fee 0.1, 5 seeds/cell.
"""
import json
import sys
sys.path.insert(0, '/home/hatch/workspace/wallet-mining-solana/sim')
import sim as S

BASE = {'payout': 2400, 'half_life': 30, 'tiers': 'mid',
        'seed_sol': 20, 'wall_days': 14, 'risk_averse': True,
        'refine_fee': 0.10}

rows = []
print(f"{'diff':<6} {'compete':<8} {'sell':<5} {'catch':<7} {'comp_d':<7} "
      f"{'fin_min':<8} {'px':<7}")
for dt in (0.0, 400.0):
    for cm in (0.25, 0.50, 1.00):
        S.COMPETE_MEDIAN = cm
        for sell in (0.5, 0.7):
            catches, cds, fms, pxs = 0, [], [], []
            for seed in range(5):
                cfg = dict(BASE, sell_frac=sell, difficulty_target_gpuh=dt)
                m = S.run(cfg, seed=seed)
                catches += m['catches']
                cds.append(m['competitive_days'])
                fms.append(m['final_miners'])
                pxs.append(m['price_end_ratio'])
            row = {'difficulty_target': dt, 'compete_median': cm,
                   'sell_frac': sell, 'catches': f"{catches}/5",
                   'comp_days_mean': round(sum(cds) / 5, 1),
                   'final_miners_mean': round(sum(fms) / 5, 1),
                   'price_end_mean': round(sum(pxs) / 5, 2)}
            rows.append(row)
            print(f"{dt:<6.0f} {cm:<8} {sell:<5} {row['catches']:<7} "
                  f"{row['comp_days_mean']:<7} {row['final_miners_mean']:<8} "
                  f"{row['price_end_mean']:<7}")

S.COMPETE_MEDIAN = 1.00
with open('/home/hatch/workspace/wallet-mining-solana/sim/results/delta_difficulty.json', 'w') as fh:
    json.dump(rows, fh, indent=1)
print("\nwrote sim/results/delta_difficulty.json")
