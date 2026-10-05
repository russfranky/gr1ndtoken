#!/usr/bin/env python3
"""Delta run: finer joint sweep around the difficulty-adjusted winner.
LOCAL ONLY. Optimizes payout x half-life x difficulty_target at the
realistic competing bar ($0.25) and realistic selling (0.7), fee 0.1.
Goal: the config that best survives the sell_behavior blocker.
"""
import json
import sys
sys.path.insert(0, '/home/hatch/workspace/wallet-mining-solana/sim')
import sim as S

S.COMPETE_MEDIAN = 0.25
BASE = {'tiers': 'mid', 'seed_sol': 20, 'wall_days': 14, 'risk_averse': True,
        'sell_frac': 0.7, 'refine_fee': 0.10}

rows = []
print(f"{'payout':<7} {'hl':<4} {'dt':<5} {'catch':<7} {'comp_d':<7} "
      f"{'fin_min':<8} {'px':<7} {'gini':<6}")
best = None
for payout in (1200, 2400, 4800):
    for hl in (21, 30, 45):
        for dt in (400.0, 800.0):
            catches, cds, fms, pxs, gns = 0, [], [], [], []
            for seed in range(5):
                cfg = dict(BASE, payout=payout, half_life=hl,
                           difficulty_target_gpuh=dt)
                m = S.run(cfg, seed=seed)
                catches += m['catches']
                cds.append(m['competitive_days'])
                fms.append(m['final_miners'])
                pxs.append(m['price_end_ratio'])
                gns.append(m['gini'])
            row = {'payout': payout, 'half_life': hl, 'diff_target': dt,
                   'catches': f"{catches}/5",
                   'comp_days_mean': round(sum(cds) / 5, 1),
                   'final_miners_mean': round(sum(fms) / 5, 1),
                   'price_end_mean': round(sum(pxs) / 5, 2),
                   'gini_mean': round(sum(gns) / 5, 3)}
            rows.append(row)
            print(f"{payout:<7} {hl:<4} {dt:<5.0f} {row['catches']:<7} "
                  f"{row['comp_days_mean']:<7} {row['final_miners_mean']:<8} "
                  f"{row['price_end_mean']:<7} {row['gini_mean']:<6}")
            key = (catches, sum(cds) / 5)
            if best is None or key > best[0]:
                best = (key, row)

S.COMPETE_MEDIAN = 1.00
with open('/home/hatch/workspace/wallet-mining-solana/sim/results/delta_fine.json', 'w') as fh:
    json.dump(rows, fh, indent=1)
print("\nwrote sim/results/delta_fine.json")
print("BEST:", best[1])
