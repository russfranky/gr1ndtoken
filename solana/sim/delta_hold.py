#!/usr/bin/env python3
"""Delta run: refining-fee hold incentive vs realistic sell behavior.
LOCAL ONLY. Targeted check, not a rebuild.
Compares top refined configs at sell_frac {0.5, 0.7} with refine_fee {0, 0.10}.
"""
import json
import sys
sys.path.insert(0, '/home/hatch/workspace/wallet-mining-solana/sim')
from sim import run

CONFIGS = {
    'D1_diffuse': {'payout': 9600, 'half_life': 14, 'tiers': 'diffuse',
                   'seed_sol': 20, 'wall_days': 14, 'risk_averse': True},
    'D2_mid':     {'payout': 2400, 'half_life': 30, 'tiers': 'mid',
                   'seed_sol': 20, 'wall_days': 14, 'risk_averse': True},
}

print(f"{'config':<10} {'sell':<5} {'fee':<5} {'comp_days':<10} {'final_min':<10} "
      f"{'price_x':<8} {'catch':<7}")
rows = []
for name, base in CONFIGS.items():
    for sell in (0.5, 0.7):
        for fee in (0.0, 0.10):
            catches, cds, fms, pxs = 0, [], [], []
            for seed in (0, 1, 2):
                cfg = dict(base, sell_frac=sell, refine_fee=fee)
                m = run(cfg, seed=seed)
                catches += m['catches']
                cds.append(m['competitive_days'])
                fms.append(m['final_miners'])
                pxs.append(m['price_end_ratio'])
            row = {'config': name, 'sell_frac': sell, 'refine_fee': fee,
                   'catches': f"{catches}/3",
                   'comp_days_mean': round(sum(cds) / 3, 1),
                   'final_miners_mean': round(sum(fms) / 3, 1),
                   'price_end_mean': round(sum(pxs) / 3, 2)}
            rows.append(row)
            print(f"{name:<10} {sell:<5} {fee:<5} {row['comp_days_mean']:<10} "
                  f"{row['final_miners_mean']:<10} {row['price_end_mean']:<8} "
                  f"{row['catches']:<7}")

with open('/home/hatch/workspace/wallet-mining-solana/sim/results/delta_hold.json', 'w') as fh:
    json.dump(rows, fh, indent=1)
print("\nwrote sim/results/delta_hold.json")
