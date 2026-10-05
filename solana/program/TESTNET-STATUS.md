# grindmine testnet deployment — FINAL STATUS (2026-10-05 ~04:00 UTC)

## Result: BLOCKED on testnet SOL — faucet unreachable from this environment

15+ airdrop attempts over ~90 minutes (1 and 2 SOL, 4 fresh addresses):
every one failed with "rate limit reached". A fresh-address test proved the
limit is IP-based: this VM shares one egress IP and it is throttled.
Web faucet requires Cloudflare Turnstile (not solvable via curl).
solfaucet.com API returned empty. Local validator cannot run (kernel blocks
io_uring). Helius has no testnet endpoint. Per the task instruction, I stopped
rather than touching mainnet.

## Built, tested, and ready to deploy the moment SOL arrives

- **Program** (`program/grindmine-program/`, native Rust, no Anchor):
  113KB valid eBPF (`target/deploy/grindmine_program.so`).
  `initialize` (pattern-validating config/manifest PDA) + `claim`
  (ed25519 precompile ownership proof, in-program base58 tier check,
  PDA double-claim registry, SPL mint with trailing-window difficulty
  multiplier, on-chain supply cap (`max_supply`/`minted_total`,
  `SupplyExhausted` past the cap). No admin instructions after initialize;
  `initialize` restricted to a hardcoded deployer key (front-run guard).
  Upgrade authority: testnet deploys stay upgradeable by explicit choice
  (deploy script `--final` flag for immutable); keyfile deletion is NOT
  claimed as revocation (red-team C3).
- **Unit tests 8/8 pass**: impossible first chars rejected
  (`Zebra`, `grind`, `zzzzz`), non-base58 chars rejected (`0`,`O`,`I`,`l`),
  base58 codec verified (known vector + roundtrip), initialize authority
  placeholder rejected, degenerate init params rejected, Config length
  matches fields (supply-cap growth accounted).
- **Client** (`program/grindmine-client/`, builds clean):
  `init`, `claim`, `neg` (double-claim / bad-sig / pattern-mismatch).
- **Grinder** (`grinder/grind.py`): os.urandom CSPRNG, tier scoring.
  Two `Gr1n` keys ground: `Gr1nrGVTSfbFBcwDR4UHCMKrTUDFPmSsb1TZqSh5WjWX`
  and `Gr1nkwvLRHuaHaNQUutuWbe3v5ykE3hpWgczXvByoWnc`
  (secrets in `program/keys/mined1.b64`, `mined2.b64`).
- **One-shot deploy script**: `program/deploy_testnet.sh`
  (deploy → init → claim → 3 negative tests → burn upgrade authority).
- **Program ID** (keypair generated, NOT yet deployed):
  `7MBhRT9LVVzu2W5esmUS4K7idT8w1GzVMekNhX5Cu6PF`
- **Docs**: `program/TESTNET-RUNBOOK.md` (design choices, steps).

## To unblock

Fund the throwaway payer on TESTNET with ~3 SOL:
`H99FnaQtUqGh7BrN3hxQUKo3MDyrMYvwPA1XxfFHjL21`
then run `program/deploy_testnet.sh`. The owner's Gr1nd vanity key was
not used anywhere. Nothing was committed or pushed; all work is local
under `~/workspace/wallet-mining-solana/`.
