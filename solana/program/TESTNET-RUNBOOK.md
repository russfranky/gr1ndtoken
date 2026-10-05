# grindmine testnet deployment — runbook (2026-10-04/05)

Local only. Never commit or push. Testnet only, never mainnet.

## What was built

Native Solana program (no Anchor — small, auditable), workspace at
`program/`:

- `grindmine-program/` — on-chain program (113KB .so)
  - `initialize`: creates the config/manifest PDA. Validates the tier
    pattern (rejects impossible patterns: first char outside
    `123456789ABCDEFGHJ`, or any char outside base58). Verifies the SPL
    mint names the program's mint-auth PDA as its authority.
  - `claim(mined_pubkey, matched)`: verifies the Ed25519 precompile
    instruction at tx index 0 (ownership proof, message bound to
    program_id + pubkey + tier), base58-encodes the pubkey in-program and
    checks the tier prefix, rejects double-claims via a PDA registry
    keyed by the mined pubkey, mints SPL rewards with a smooth difficulty
    multiplier (trailing-window adjustment, 0.1x–10x clamp).
  - No admin instructions exist after initialize. Deployed with a
    throwaway upgrade authority (key discarded after deploy).
- `grindmine-client/` — testnet driver: `init`, `claim`, `neg`
  (double-claim / bad-signature / pattern-mismatch negative tests).
- `grinder/grind.py` — off-chain grinder, os.urandom CSPRNG (non-negotiable),
  tier-prefix scoring.

## Demo parameters (testnet only, not launch values)

- Pattern: `Gr1nd`, min tier 4 chars (grindable on this VM in ~3 min;
  production tiers would be 6–7 chars per the sim).
- Payouts: 4-char = 1000 tokens, 5-char = 58000 tokens (6 decimals).
- Difficulty: 100-claim target per 1-day window, multiplier starts 1.0.

## Keys (all throwaway, testnet only)

- `program/keys/throwaway.json` — payer for init/claims (faucet-funded).
- `program/keys/deploy.json` — program deploy/upgrade authority (discarded).
- `program/target/deploy/grindmine_program-keypair.json` — program ID keypair.
- The owner's Gr1nd vanity key was NOT used anywhere in this deployment.

## Steps

1. `solana airdrop` to throwaway.json (testnet).
2. `solana program deploy --program-id <keypair> target/deploy/grindmine_program.so
   --upgrade-authority keys/deploy.json --url testnet --keypair keys/throwaway.json`
3. `grindmine-client init --program <PROGRAM_ID> --payer keys/throwaway.json`
   (creates mint with mint-auth PDA as authority, then initializes config).
4. Grind: `python3 grinder/grind.py --prefix Gr1n` → base64 secret.
5. `grindmine-client claim --program <ID> --payer keys/throwaway.json
   --key <b64> --matched 4`
6. Negative tests: `neg --case double|badsig|mismatch`.
7. Discard deploy.json (upgrade authority effectively burned).

## Simplest-reasonable choices (documented)

- Native program instead of Anchor: smaller, auditable, no IDL machinery.
- Rewards go to the tx signer's token account (the operator); the mined
  vanity key never touches chain.
- Manifest = the config PDA itself (machine-readable on-chain).
- Difficulty window will not trigger in the demo (few claims); the
  adjustment code path is exercised only in unit review, not on-chain.
