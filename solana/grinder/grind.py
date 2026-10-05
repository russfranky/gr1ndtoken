#!/usr/bin/env python3
"""Grinder for the grindmine testnet demo.

Generates Ed25519 keypairs with os.urandom (CSPRNG - non-negotiable, these
keys may hold funds) and scores them against a tier prefix.

Usage:
    python3 grind.py --prefix Gr1n                 # grind until a key starts with Gr1n
    python3 grind.py --prefix Gr1n --count 2        # find 2 keys
    python3 grind.py --any                          # save one random key (for negative tests)
    python3 grind.py --prefix Gr1n --hits-file ./x.jsonl

Contract with grindmine-client: each hits-file line is
    {"address": "<base58>", "secret_b64": "<base64 of 64-byte seed||pubkey>"}
The client reads it via --hits <file> --address <addr>.

Output per hit: "<base58 address>" on stdout; the secret is appended to the
hits file (0600, O_NOFOLLOW, uid/mode verified) and never printed.
Terminal scrollback is a key store.
"""
import argparse
import base64
import os
import stat
import sys
import time

from nacl.signing import SigningKey

B58 = '123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz'

DEFAULT_HITS_FILE = os.path.expanduser('~/.config/grindmine/grind-hits.jsonl')


def ensure_hits_dir(path):
    d = os.path.dirname(os.path.abspath(path))
    os.makedirs(d, mode=0o700, exist_ok=True)
    # makedirs(exist_ok=True) won't fix perms on an existing dir; enforce.
    os.chmod(d, 0o700)


def open_hits_file(path):
    """Open the hits file, hardened: no symlinks, uid/mode verified."""
    ensure_hits_dir(path)
    try:
        fd = os.open(path, os.O_WRONLY | os.O_CREAT | os.O_APPEND | os.O_NOFOLLOW, 0o600)
    except OSError:
        raise SystemExit('refusing hits file (symlink or open failed): %s' % path)
    try:
        st = os.fstat(fd)
    except BaseException:
        os.close(fd)
        raise
    if st.st_uid != os.getuid():
        os.close(fd)
        raise SystemExit('refusing hits file not owned by this user: %s' % path)
    if stat.S_IMODE(st.st_mode) & 0o077:
        os.close(fd)
        raise SystemExit('refusing hits file readable by group/other: %s' % path)
    return fd


def save_hit(fd, addr, secret64_b64):
    # Secrets never touch stdout: append to the hardened hits file.
    with os.fdopen(os.dup(fd), 'a') as f:
        f.write('{"address": "%s", "secret_b64": "%s"}\n' % (addr, secret64_b64))


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


def validate_prefix(prefix):
    bad = [c for c in prefix if c not in B58]
    if bad:
        raise SystemExit(
            'invalid prefix: %r contains non-base58 chars %r '
            '(0, O, I, l are not in the base58 alphabet and can never match)' % (prefix, bad))
    if not prefix:
        raise SystemExit('prefix must not be empty')


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument('--prefix', default=None, help='base58 prefix to grind for')
    ap.add_argument('--count', type=int, default=1)
    ap.add_argument('--any', action='store_true', help='save one random key, no grinding')
    ap.add_argument('--hits-file', default=DEFAULT_HITS_FILE,
                    help='hits JSONL path (default ~/.config/grindmine/grind-hits.jsonl)')
    args = ap.parse_args()

    if args.prefix is not None and not args.any:
        validate_prefix(args.prefix)

    fd = open_hits_file(args.hits_file)
    try:
        if args.any or args.prefix is None:
            sk = SigningKey(os.urandom(32))
            addr = b58encode(bytes(sk.verify_key))
            save_hit(fd, addr, base64.b64encode(bytes(sk) + bytes(sk.verify_key)).decode())
            print(f'{addr} (secret saved to {args.hits_file})')
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
                save_hit(fd, addr, base64.b64encode(bytes(sk) + bytes(sk.verify_key)).decode())
                print(f'HIT {addr} (secret saved to {args.hits_file})', flush=True)
                print(f'  ({rolls:,} rolls in {dt:.1f}s, {rolls / dt:,.0f}/s)', file=sys.stderr, flush=True)
            elif rolls % 500000 == 0:
                dt = time.time() - t0
                print(f'... {rolls:,} rolls, {rolls / dt:,.0f}/s', file=sys.stderr, flush=True)
    finally:
        os.close(fd)


if __name__ == '__main__':
    main()
