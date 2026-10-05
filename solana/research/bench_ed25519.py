"""Ed25519 keypair generation benchmark (grindtokens sim input).
Measures raw rolls/sec: one roll = one Ed25519 keypair (Solana address = pubkey).
Uses libsodium via PyNaCl - same curve as Solana (ed25519-dalek compatible).
"""
import time
import multiprocessing as mp
from nacl.bindings import crypto_sign_keypair

def worker(n, q):
    t0 = time.perf_counter()
    for _ in range(n):
        crypto_sign_keypair()
    q.put(time.perf_counter() - t0)

def bench(n, procs):
    q = mp.Queue()
    ps = [mp.Process(target=worker, args=(n // procs, q)) for _ in range(procs)]
    t0 = time.perf_counter()
    for p in ps: p.start()
    for p in ps: p.join()
    dt = time.perf_counter() - t0
    total = sum(q.get() for _ in ps)  # noqa - drain
    return n / dt

if __name__ == '__main__':
    import os
    print(f"cpus={mp.cpu_count()}")
    # warmup
    bench(2000, 1)
    for procs in (1, mp.cpu_count()):
        # scale n so each run takes ~5-10s
        n = 20000 * procs
        r = bench(n, procs)
        print(f"procs={procs} rolls/sec={r:,.0f} per_core={r/procs:,.0f}")
