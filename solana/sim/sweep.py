#!/usr/bin/env python3
"""Grid sweep over launch configs. Writes sim/results/sweep.csv (one row per config×seed)."""
import itertools, csv, os, json, sys
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from sim import run

GRID = {
    'payout':    [2400, 9600, 38400],   # day-0 expected tokens per GPU-hour
    'half_life': [7, 14, 30],           # emission decay half-life (days)
    'tiers':     ['diffuse', 'mid', 'jackpot'],
    'seed_sol':  [20, 50],              # launcher SOL budget B (2*B committed total)
    'wall_days': [7, 14],               # buy-wall duration
    'sell_frac': [0.3, 0.5, 0.7],       # miner immediate-sell fraction
}
SEEDS = [0, 1, 2]

def main():
    outdir = os.path.join(os.path.dirname(os.path.abspath(__file__)), 'results')
    os.makedirs(outdir, exist_ok=True)
    keys = list(GRID.keys())
    combos = list(itertools.product(*[GRID[k] for k in keys]))
    print(f'{len(combos)} configs x {len(SEEDS)} seeds = {len(combos)*len(SEEDS)} runs')
    rows = []
    for vals in combos:
        cfg = dict(zip(keys, vals))
        for s in SEEDS:
            m = run(cfg, seed=s)
            rows.append({**cfg, 'seed': s, **m})
    with open(os.path.join(outdir, 'sweep.csv'), 'w', newline='') as f:
        w = csv.DictWriter(f, fieldnames=list(rows[0].keys()))
        w.writeheader(); w.writerows(rows)
    # aggregate: mean metrics per config
    agg = {}
    for r in rows:
        k = tuple(r[c] for c in keys)
        agg.setdefault(k, []).append(r)
    summary = []
    for k, rs in agg.items():
        n = len(rs)
        summary.append({
            **dict(zip(keys, k)),
            'catch_rate': sum(r['catches'] for r in rs) / n,
            'competitive_days': round(sum(r['competitive_days'] for r in rs) / n, 1),
            'peak_miners': round(sum(r['peak_miners'] for r in rs) / n, 1),
            'final_miners': round(sum(r['final_miners'] for r in rs) / n, 1),
            'price_end_ratio': round(sum(r['price_end_ratio'] for r in rs) / n, 3),
            'gini': round(sum(r['gini'] for r in rs) / n, 3),
            'total_claims': round(sum(r['total_claims'] for r in rs) / n, 0),
        })
    # composite score for ranking (documented): catches dominate, then
    # competitive days, then scale, penalize centralization
    for s in summary:
        s['score'] = round(
            s['catch_rate'] * 1000 + s['competitive_days'] * 10
            + min(s['peak_miners'], 200) * 0.25 - s['gini'] * 20
            + min(s['price_end_ratio'], 3) * 5, 2)
    summary.sort(key=lambda s: -s['score'])
    with open(os.path.join(outdir, 'sweep_summary.json'), 'w') as f:
        json.dump(summary, f, indent=1)
    print('top 8:')
    for s in summary[:8]:
        print(f"  score={s['score']:7.2f} catch={s['catch_rate']:.2f} compd={s['competitive_days']:4.1f} "
              f"peak={s['peak_miners']:5.1f} final={s['final_miners']:5.1f} p_end={s['price_end_ratio']:.2f} "
              f"gini={s['gini']:.2f} | payout={s['payout']} hl={s['half_life']} tiers={s['tiers']} "
              f"B={s['seed_sol']} wall={s['wall_days']}d")
    ncatch = sum(1 for s in summary if s['catch_rate'] > 0)
    print(f'configs with catch_rate>0: {ncatch}/{len(summary)}')

if __name__ == '__main__':
    main()
