#!/usr/bin/env python3
"""Focused sensitivity: winning config with seed_sol in {0,2,5,10,20} x sell_frac {0.3,0.5,0.7} x 3 seeds."""
import itertools, csv, os, sys, json
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from sim import run

BASE = {'payout': 2400, 'half_life': 30, 'tiers': 'jackpot', 'wall_days': 14}
SEEDS = [0, 1, 2]
rows = []
for seed_sol, sell_frac in itertools.product([0, 2, 5, 10, 20], [0.3, 0.5, 0.7]):
    cfg = {**BASE, 'seed_sol': seed_sol, 'sell_frac': sell_frac}
    for s in SEEDS:
        m = run(cfg, seed=s)
        rows.append({**cfg, 'seed': s, **m})
        print(f"B={seed_sol:2d} sell={sell_frac} seed={s}: catch={m['catches']} compd={m['competitive_days']:.1f} "
              f"peak={m['peak_miners']:.0f} final={m['final_miners']:.0f} p_end={m['price_end_ratio']:.2f} gini={m['gini']:.2f}", flush=True)

agg = {}
for r in rows:
    k = (r['seed_sol'], r['sell_frac'])
    agg.setdefault(k, []).append(r)
print('\n=== summary (mean over seeds) ===')
for k in sorted(agg):
    rs = agg[k]
    n = len(rs)
    print(f"B={k[0]:2d} sell={k[1]}: catch={sum(r['catches'] for r in rs)/n:.2f} "
          f"compd={sum(r['competitive_days'] for r in rs)/n:.1f} "
          f"peak={sum(r['peak_miners'] for r in rs)/n:.0f} final={sum(r['final_miners'] for r in rs)/n:.0f} "
          f"p_end={sum(r['price_end_ratio'] for r in rs)/n:.2f} gini={sum(r['gini'] for r in rs)/n:.2f}")
outdir = os.path.join(os.path.dirname(os.path.abspath(__file__)), 'results')
with open(os.path.join(outdir, 'nowall_sweep.csv'), 'w', newline='') as f:
    w = csv.DictWriter(f, fieldnames=list(rows[0].keys()))
    w.writeheader(); w.writerows(rows)
print('wrote results/nowall_sweep.csv')
