#!/usr/bin/env python3
# Grind a real Solana address starting with "Gr1nd". Local only, throwaway key.
# Fast path: integer range check instead of full base58 encode per roll.
import os, sys, time, base64, multiprocessing as mp
from nacl.signing import SigningKey

B58 = '123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz'
B58D = {c: i for i, c in enumerate(B58)}

def b58val(s: str) -> int:
    n = 0
    for c in s:
        n = n * 58 + B58D[c]
    return n

PREFIX = 'Gr1nd'
X = b58val(PREFIX)
LO, HI = X * 58**39, (X + 1) * 58**39

def worker(prog_q, res_q):
    rolls = 0
    while True:
        sk = SigningKey(os.urandom(32))
        n = int.from_bytes(bytes(sk.verify_key), 'big')
        rolls += 1
        if LO <= n < HI:
            res_q.put((bytes(sk), rolls))
            return
        if rolls % 200000 == 0:
            prog_q.put(rolls)
            rolls = 0

def main():
    ctx = mp.get_context('fork')
    prog_q, res_q = ctx.Queue(), ctx.Queue()
    procs = [ctx.Process(target=worker, args=(prog_q, res_q)) for _ in range(2)]
    t0 = time.time()
    for p in procs: p.start()
    total = 0
    found = None
    try:
        while found is None:
            try:
                found = res_q.get(timeout=10)
            except Exception:
                while not prog_q.empty():
                    total += prog_q.get()
                dt = time.time() - t0
                print(f'... {total:,} rolls, {total/dt:,.0f}/s', flush=True)
    finally:
        for p in procs: p.terminate()
    sk_bytes, rolls = found
    total += rolls
    sk = SigningKey(sk_bytes)
    n = int.from_bytes(bytes(sk.verify_key), 'big')
    s, nn = '', n
    while nn > 0:
        nn, r = divmod(nn, 58); s = B58[r] + s
    addr = s.rjust(44, '1')
    dt = time.time() - t0
    assert addr.startswith(PREFIX), addr
    print(f'FOUND after ~{total:,} rolls in {dt:.0f}s')
    print(f'address: {addr}')
    print(f'secret_b64_throwaway: {base64.b64encode(sk_bytes).decode()}')

if __name__ == '__main__':
    main()
