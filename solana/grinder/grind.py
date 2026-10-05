#!/usr/bin/env python3
"""Grinder for the grindmine testnet demo.

Generates Ed25519 keypairs with os.urandom (CSPRNG - non-negotiable, these
keys may hold funds) and scores them against a tier prefix.

Usage:
    python3 grind.py --prefix Gr1n          # grind until a key starts with Gr1n
    python3 grind.py --prefix Gr1n --count 2 # find 2 keys
    python3 grind.py --any                   # print one random key (for negative tests)

Output per hit: "<base58 address>" on stdout; the secret is appended to
./grind-hits.jsonl (0600) and never printed. Terminal scrollback is a key store.
"""
import argparse
import base64
import os
import sys
import time

from nacl.signing import SigningKey

B58 = '123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz'

HITS_FILE = 'grind-hits.jsonl'


def save_hit(addr, sk_b64):
    # Secrets never touch stdout: append to a 0600 file, created with
    # O_CREAT|0o600 so umask can't widen it. Terminal scrollback is a key store.
    fd = os.open(HITS_FILE, os.O_WRONLY | os.O_CREAT | os.O_APPEND, 0o600)
    try:
        with os.fdopen(fd, 'a') as f:
            f.write('{"address": "%s", "secret_b64": "%s"}\n' % (addr, sk_b64))
    except BaseException:
        os.close(fd)
        raise


def b58encode(b: bytes) -> str:
    n = int.from_bytes(b, 'big')
    s = ''
    while n > 0:
        n, r = divmod(n, 58)
        s = B58[r] + s
    pad = 0
    for c in b:
        if c == 0:
            pad += 1
        else:
            break
    return '1' * pad + s


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument('--prefix', default=None, help='base58 prefix to grind for')
    ap.add_argument('--count', type=int, default=1)
    ap.add_argument('--any', action='store_true', help='print one random key, no grinding')
    args = ap.parse_args()

    if args.any or args.prefix is None:
        sk = SigningKey(os.urandom(32))
        addr = b58encode(bytes(sk.verify_key))
        save_hit(addr, base64.b64encode(bytes(sk)).decode())
        print(f'{addr} (secret saved to {HITS_FILE})')
        return

    prefix = args.prefix
    found = 0
    rolls = 0
    t0 = time.time()
    while found < args.count:
        sk = SigningKey(os.urandom(32))
        addr = b58encode(bytes(sk.verify_key))
        rolls += 1
        if addr.startswith(prefix):
            found += 1
            dt = time.time() - t0
            save_hit(addr, base64.b64encode(bytes(sk)).decode())
            print(f'HIT {addr} (secret saved to {HITS_FILE})', flush=True)
            print(f'  ({rolls:,} rolls in {dt:.1f}s, {rolls / dt:,.0f}/s)', file=sys.stderr, flush=True)
        elif rolls % 500000 == 0:
            dt = time.time() - t0
            print(f'... {rolls:,} rolls, {rolls / dt:,.0f}/s', file=sys.stderr, flush=True)


if __name__ == '__main__':
    main()
