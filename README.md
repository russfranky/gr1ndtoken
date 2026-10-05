<p>
  <img src="assets/solana/solanaLogoMark.svg" width="26" alt="Solana logomark">
  <b>Built on Solana</b>
</p>

# gr1ndtoken — wallet mining

Wallet mining is a game mechanic where every in-game action generates a real
cryptographic wallet, and a derived address that matches a rare pattern means
the player mined a rare item. Rarity comes from the address itself, so the
numbers are verifiable by anyone and the odds are exact, not a black box.

This repo catalogs the research behind the idea: the rarity math, chain
selection, roll-binding cryptography, proof-of-personhood research, and the
interactive proof of concept. Companion to the article that started it:
https://russfranky.substack.com/p/wallet-mining-in-gaming

## Built on Solana

<p>
  <img src="assets/solana/solanaLogo.svg" width="220" alt="Solana">
</p>

gr1ndtoken is built on [Solana](https://solana.com): sub-second finality,
fractions-of-a-cent fees, and native Ed25519 signature verification via
precompiles — which is what makes on-chain vanity claim verification cheap
enough to work. Official Solana brand assets live in `assets/solana/`
(logomark, wordmark, lockups; brand colors `#9945FF` / `#14F195`).

## Settled architecture (from the research)

- **Chain: Base (L2).** EVM secp256k1 plus keccak256 address scheme gives clean
  16^-k prefix probabilities; claim verification lands around $0.002 to $0.01;
  Chainlink VRF v2.5 live as a backup randomness rail; EIP-2537 BLS precompiles
  live via the OP Stack Isthmus upgrade. Runner-up: Arbitrum One.
- **Roll binding: two-phase server-signed EIP-712 tickets.** The server attests
  each real gameplay action with a ticket binding (registered key S, epoch, seq)
  issued before the seed exists; the seed is the first drand quicknet round
  after the epoch closes; the winning key P is derived as P = S + t*G and pinned
  by a second server signature; per-ticket nullifiers prevent replays. A bare
  signature from a derived winning key proves nothing about legitimate
  derivation, so the ticket is what closes the offline grinding attack.
- **Identity: layered, biometrics only at high-value points.** A Jev evaluation
  was unanimous for cheap gates at registration (device attestation, Human
  Passport signal, behavioral analysis, ticket caps) with the World ID Orb
  biometric gate only at top-tier claims, high-frequency claimants, and
  rare-item withdrawals. Biometrics are a per-account economic cost, not an
  absolute wall; the dominant residual risk is bought or farmed verified humans.

## Repo map

- `docs/chain-selection-and-roll-binding-crypto.md`: full chain-selection and
  roll-binding research report (2026-10-03), with sources and thin-evidence flags.
- `docs/encrypted-biometrics-proof-of-personhood.md`: full proof-of-personhood
  research report (2026-10-03), with sources and thin-evidence flags.
- `docs/poc-build-specs.md`: the POC build specs. v1 (decimal mine numbers)
  is marked SUPERSEDED; v2's real-keygen half is what shipped, while its
  leading-zero rarity proposal was never implemented (status banners in the doc).
- `math/probability-tables.md`: the verified rarity probability tables, current
  and superseded, with methods stated plainly.
- `math/analyze.mjs`: Node verification script for the v1 decimal rarity math
  (analytic probabilities plus 2,000,000-roll Monte Carlo). Runs with plain Node.
- `math/bench.mjs`: Node benchmark of real wallet generation (secp256k1 plus
  keccak256) including the canonical test vector
  privkey `0x01` -> `0x7E5F4552091A69125d5DfCb7b8C2659029395Bdf`.
  Requires `npm install @noble/secp256k1 @noble/hashes` to run.
- `poc/wallet-mining-rarity-poc.html`: the interactive proof of concept,
  self-contained, runs offline in a browser.
- `qa/README.md`: notes on the in-progress quality loop against the POC.

## Solana anonymous-launch experiment (separate track, `solana/`)

A second, independent experiment: anonymous Solana launch where users grind
vanity addresses locally and claim SPL rewards on-chain. No relation to the
Base/EVM ticket design above beyond the shared rarity-math idea.

- `solana/SPEC-DRAFT.md`: mechanism spec (local Ed25519 keygen, pattern
  scoring, signature-proof claims, PDA duplicate registry, payout multiplier
  on trailing claim activity, 100% mining emission).
- `solana/LAUNCH-PLAN.md`: launch plan. Shape: deploy, manifest, grinder.
  No liquidity required at launch.
- `solana/SIM-RESULTS.md`: simulation results (972-run parameter sweep;
  modeled 2,400 tokens/GPU-hour, 30-day half-life).
- `solana/program/`: Anchor program (`grindmine-program`: Ed25519 precompile
  verification, pattern check, PDA claim registry, SPL minting, trailing-window
  multiplier) plus claim client, testnet runbook and status. Unit tests 5/5.
  Testnet deployment was in progress as of 2026-10-05 (not yet completed).
- `solana/grinder/grind.py`: reference local grinder.
- `solana/sim/`: simulation scripts and sweep results.
- `solana/research/`: ed25519 benchmarks, vanity-market learnings, keeper
  vanity-hit list, token-utility and incentive notes.
- `solana/ui/`: landing / grinder / leaderboard mockups (local HTML) with
  renders at 390px and 1440px.
- `solana/BRAND.md`: provisional brand notes (deferred).
- Offline by design: grinding is pure local key generation — no connection
  needed. Only the on-chain claim touches the network.

## Status, honestly

- The POC is a demo of the rarity mechanic. It generates real Ethereum
  addresses in the browser and classifies the first 8 hex chars into 26
  pattern classes (6 aggregate tiers). It does not implement on-chain claims,
  tickets, drand, or World ID. Those live in the research docs as the
  integration plan.
- A 6-phase quality loop ran 2026-10-03 and closed: 17 features tested, 2 low
  defects found and fixed, regression green, no critical/high defects.
  The living QA sheet stays out of this repo by process; see `qa/README.md`.
- The research reports are research, not audited code. Their thin-evidence
  flags are kept intact in the docs; read them before building on the claims.
- The rarity math (26-pattern counts over 2^32, test vector, Monte Carlo checks)
  was independently verified in Node on 2026-10-03. The 16^-k leading-zero
  tiers are a documented proposal in `docs/poc-build-specs.md`, not the
  shipped mechanic.
