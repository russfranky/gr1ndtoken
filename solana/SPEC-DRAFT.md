# Wallet Mining on Solana — Local Draft (anon launch experiment)

**Status:** local-only draft. Do NOT commit or push to the public wallet-mining repo.

## Concept

Generate Ed25519 keypairs off-chain, score the resulting address by rarity,
and have an on-chain program mint reward tokens when a miner proves they
found an address meeting a tier. The miner keeps the vanity wallet itself;
the contract only pays for the proof of work.

## Solana-specific differences from the Ethereum POC

- Addresses are 32-byte Ed25519 public keys rendered in base58 (44 chars,
  58-char alphabet: no `0`, `O`, `I`, `l`).
- Rarity per leading character is 1-in-58, not 1-in-16. The Ethereum POC's
  8-hex-char tiers (1 in ~4.3B) map to roughly 5–6 leading base58 chars
  (58^5 ≈ 656M, 58^6 ≈ 38B). Tiers need fewer characters for equivalent
  difficulty.
- `solana-keygen grind` already proves the grinding path; the new piece is
  the scoring + claim layer. GPU miners do millions of keypairs/sec.

## Claim flow (on-chain program)

1. Miner grinds keypairs locally until one hits a tier (e.g. starts with
   "SOL", 4+ leading identical chars, matching suffix).
2. Miner sends a `claim` instruction with: the mined public key, a signature
   over a program-issued nonce, and the tier id. **The private key never
   leaves the miner.**
3. The program verifies the Ed25519 signature via the built-in precompile,
   checks the pattern by base58-encoding the key in-program (a few thousand
   compute units for 32 bytes), and confirms the address was never claimed
   via a PDA registry keyed by the pubkey.
4. The program mints SPL reward tokens from its mint authority to the
   miner's token account and marks the address claimed.

**Anti-front-running property:** claims prove ownership with a signature, so
nobody can snipe a pending claim tx without the private key. No commit-reveal
scheme is needed.

## Issuance design

- Fixed tiers have cliff effects (4 chars pays X, 5 pays 100X). Preferred:
  a minimum rarity threshold with payout proportional to measured
  difficulty: `reward = base * 58^(matched_chars) / normalization`.
- Epoch-based difficulty adjustment keeps issuance steady as miners get
  faster.
- Hard supply cap + halving schedule.

## Sybil layer (from Jev verdict, 2026-10-03)

- Cheap gates at registration: identity score + per-identity ticket caps.
- Orb-level proof (World ID) only at high-value claims and withdrawals.
- Velocity tripwires; fresh proofs required per high-value claim.
- On Solana: a program-owned identity account per claimant.
- Top Jev warning carries over: bought/farmed verified humans (~$20–55/acct)
  is a per-account tax, not a wall. Size tier rewards below farming cost.

## Cost model

- Per claim: rent-exempt PDA (~0.002 SOL) + tx fee (~0.000005 SOL).
- Expected value of a hit must beat compute cost. Size the payout table
  against real grinding speeds, not just rarity.

## Prior art

Ore proved contract-mined issuance works on Solana, but it is pure hash
PoW. This design differs: the mined artifact is the wallet itself, which the
miner keeps as a trophy. That is the wallet-mining thesis.

## Design boundary: never pay-per-roll (owner, 2026-10-04)
The design could trivially become a gambling machine if rolls cost anything. That is explicitly NOT the goal. Rolls (keypair grinding) are free, always — the miner's only cost is their own compute. No token payment, no fee, no stake required to grind. This is a structural bright line: it keeps the mechanism on the mining side (proof of work, free entry) rather than the gambling side (paid chance). Any future mechanism that puts a price on the *attempt* rather than the *claim* violates this boundary. (Claim-side costs are a separate question — see the batch-claims open item — but per-roll payment is out, permanently.)
