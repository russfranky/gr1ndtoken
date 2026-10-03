# Wallet-Mining Game Design: Chain Selection, Roll Binding, End-to-End Architecture

Research date: 2026-10-03. Flags: `index` = found via web search/fetch (not re-verified live); `verified live` = read in the research browser. All fee/round-time/protocol figures are cited with URLs. Design-analysis claims (cryptographic reasoning, no external source) are marked as such.

## Summary

**Recommended chain: Base.** It is the only candidate that simultaneously offers (1) the EVM secp256k1+keccak256 address scheme (clean 16^-k prefix probabilities), (2) claim verification at ~$0.002–0.01 (vs ~$0.20+ on L1), (3) Chainlink VRF v2.5 live as a backup randomness rail, (4) EIP-2537 BLS12-381 precompiles live via the OP Stack Isthmus upgrade — enabling **trustless on-chain verification of drand quicknet beacons at ~110k gas**, and (5) 2-second blocks with mature ERC-4337/paymaster + Coinbase Smart Wallet infrastructure for gasless claims. Runner-up: **Arbitrum One** (~250ms blocks, BOLD fraud proofs, VRF live; modestly higher fees).

**Core construction (Occam's answer to Part B):** two-phase server-signed EIP-712 tickets + a strict timing rule (seed = first drand round *after* the epoch closes) + on-chain BLS verification of the beacon + per-ticket nullifiers + claim authorization signed by the registered key S. The key insight: **a bare signature from a derived winning key proves nothing about legitimate derivation** — after seed reveal, anyone holding `s` can grind arbitrary `seq` values offline and sign from the resulting keys. What closes the grinding attack is the **ticket**: a server attestation binding `(S, epoch, seq)` issued *before* the seed existed, with `P` pinned by a second server signature after the seed is known (P is public data, so the client verifies the server computed it honestly). No ZK prover, no exotic Schnorr, no commit-reveal claim ceremony is needed; two `ecrecover`s + one `keccak256` + nullifier do the work.

**Residual risks (honest list):** (1) the ticket issuer must honestly attest real gameplay — "prove you played" reduces to trust in attestation, which cryptography cannot eliminate; the server can grief/censor but cannot steal; (2) bot farms doing *real* (automated) gameplay remain an economic, not cryptographic, problem — the cheapest attack is N bot accounts each earning capped tickets, profitable iff expected prize value exceeds botting cost; (3) drand liveness/threshold trust (League of Entropy operators must not sign early in private); (4) L2 sequencer trust for soft confirmations (Base sequencer is Coinbase-operated).

---

## PART A — CHAIN SELECTION

### A.1 Address scheme and pattern-probability math

**EVM chains (Ethereum L1, Base, Arbitrum One, Optimism, Polygon PoS):** address = last 20 bytes of `keccak256(uncompressed secp256k1 pubkey)`, rendered as 40 hex chars. Because keccak256 output is modeled as uniform, the probability that an address matches a fixed `k`-hex-char prefix is exactly **16^-k per roll** (e.g., 4 hex chars ≈ 1/65,536; 6 hex chars ≈ 1/16.7M). Independent per roll, cleanly composable into rarity tiers. This is the reference scheme and it transfers unchanged to every EVM L2/sidechain.

**Solana:** addresses are raw 32-byte ed25519 public keys, conventionally base58-encoded. A "rare prefix" would be defined over base58 digits: per-character probability ≈ 1/58 but **non-uniform at the leading byte** (base58 leading-zero suppression), so the math is messier and less standard. The linear derivation `P = S + t·G` is also natural on ed25519, but Solana has no keccak-address step and no EVM ticket/claim tooling — the whole Part C construction would need re-engineering for the Solana account/program model.

**Aptos / Sui (evaluated as "native randomness" alternatives):** both use ed25519-based address schemes (not secp256k1+keccak), so the reference probability math does not transfer; Move tooling instead of Solidity.

### A.2 Per-chain evaluation

**Ethereum L1.**
- Fees: gas averaged 0.8993 gwei on 2026-09-19, ~0.5 gwei April 2026 average; an ERC-20 transfer (~65k gas) cost ~$0.076 at 0.5 gwei with ETH ≈ $2,350. A claim transaction (~150–250k gas by our estimate) would cost roughly **$0.18–0.40** — prohibitive for frequent small wins. Median tx fee 2026Q1: $0.012283. ([coinlaw.io](https://coinlaw.io/ethereum-gas-fees-statistics/); [MDPI](https://www.mdpi.com/2674-1032/5/3/81), index)
- Randomness: Chainlink VRF v2.5 live (20% LINK / 24% native premium). On-chain drand verification possible via EIP-2537 (live since Pectra, May 2025). ([docs.chain.link](https://docs.chain.link/vrf/v2-5/supported-networks); [7blocklabs](https://www.7blocklabs.com/blog/faster-proof-verification-on-ethereum-using-the-eip-2537-bls12-381-precompiles), index)
- Finality/latency: 12s blocks. Verdict: best security, but claim costs kill the game loop.

**Base.**
- Fees: simple transfer ~$0.001 or less; DEX swap ~$0.001–$0.01; 200k gas ≈ $0.002 at $2,000 ETH ([Base ecosystem research](https://github.com/lordbasilaiassistant-sudo/theagentcafe/blob/HEAD/base-ecosystem-research.md), Mar 2026, index). Measured 2026-09-27: base fee 0.005 gwei ([cryptocompass](https://cryptocompass.com/articles/base-token-no-date-no-quota-but-6-27-billion-dollars-on-the-chain), index). 2026Q1 median tx fee $0.001634 ([MDPI](https://www.mdpi.com/2674-1032/5/3/81), index). **Derived estimate: a claim tx (see C.2) ≈ $0.002–0.01.**
- VRF: Chainlink VRF v2.5 live on Base — coordinator `0xd5D517aBE5cF79B7e95eC98dB0f0277788aFF634`, min confirmations 0, premium 50% LINK / 60% native ([docs.chain.link](https://docs.chain.link/vrf/v2-5/supported-networks), index). Supra dVRF claims 80+ chain coverage incl. major L2s ([supra.com](https://supra.com/vi/academy/chainlink-vrf/), index).
- BLS: EIP-2537 precompiles enabled on OP Stack via the Isthmus (L2 Pectra) upgrade ([Optimism design-docs FMA](https://github.com/ethereum-optimism/design-docs/blob/HEAD/security/fma-isthmus-l2-pectra.md), index) → on-chain drand verification at ~110k gas (pairing check 37,700 + 32,600×2; [7blocklabs](https://www.7blocklabs.com/blog/faster-proof-verification-on-ethereum-using-the-eip-2537-bls12-381-precompiles), index).
- Finality/latency: 2s blocks, sub-second soft confirmations; 7-day native withdrawal window (irrelevant unless bridging items out) ([ethskills](https://github.com/andginja/ethskills/blob/HEAD/skills/layer-2s/SKILL.md), index).
- AA: ERC-4337 + EIP-7702 (Pectra) live; paymaster gas sponsorship ~$1–10 per 1,000 txs; Coinbase Smart Wallet (passkey, gasless UX) is Base-native ([theagentcafe research](https://github.com/lordbasilaiassistant-sudo/theagentcafe/blob/HEAD/base-ecosystem-research.md); [ethskills](https://github.com/andginja/ethskills/blob/HEAD/skills/layer-2s/SKILL.md), index).

**Arbitrum One.**
- Fees: 2026Q1 median $0.002183; DEX-swap range quoted $0.05–$0.30 (higher than Base's $0.01–$0.05) ([MDPI](https://www.mdpi.com/2674-1032/5/3/81); [eco.com](https://eco.com/support/en/articles/10273675-what-is-optimism-the-ethereum-l2-and-op-mainnet-explained), index). TVL $1.37B (Sep 15, 2026, DeFiLlama via eco.com).
- VRF v2.5 live; fraud proofs: BOLD multi-round; 7-day withdrawals. Latency: ~250ms blocks ([atlas CHAIN_CONFIG](https://github.com/fastlane-labs/atlas/blob/HEAD/docs/CHAIN_CONFIG.md), index). Verdict: lowest-latency EVM option; modestly pricier than Base.

**Optimism (OP Mainnet).**
- Fees: cheapest in the 2026Q1 sample — median $0.000027, ~78% below Arbitrum and ~75% below Base ([MDPI](https://www.mdpi.com/2674-1032/5/3/81), index); swap range $0.01–$0.05 ([eco.com](https://eco.com/support/en/articles/10273675-what-is-optimism-the-ethereum-l2-and-op-mainnet-explained), index).
- VRF v2.5 live; Cannon single-round fault proofs; 2s blocks; TVL $438M. Verdict: cheapest fees, but thinner ecosystem/distribution than Base; credible alternative if fee minimization dominates.

**Polygon PoS.**
- Fees: "fractions of a cent," typically $0.001–$0.01; Polygon claims ~$0.002 avg ([coinlaw](https://coinlaw.io/polygon-statistics/); [cryptonews.net](https://cryptonews.net/news/altcoins/33474999/), index). **Conflict:** the MDPI study measured Polygon's median fee *rising* 19.6% in 2026Q1 to $0.006868 — the only chain with rising fees. Evidence is mixed; treat fee leadership claims skeptically.
- VRF v2.5 live but with the highest premiums observed: 70% LINK / 84% POL ([docs.chain.link](https://docs.chain.link/vrf/v2-5/supported-networks), index).
- Latency: ~2s blocks (1.5s claimed after June 2026 gas-limit raise); finality cut to ~5s (July 2025); Rio upgrade (Oct 2025) ended reorgs ([cryptocompass](https://cryptocompass.com/articles/why-is-polygon-considered-one-of-crypto-s-most-enterprise-friendly-chains), index).
- Structural minus: sidechain (PoS bridge) security, not Ethereum-rollup security; POL-denominated fees add FX friction. Verdict: cheap but weaker trust story and conflicting fee evidence.

**Solana.**
- Fees: median non-vote tx fee 5,438 lamports ≈ $0.00057 (Aug 27, 2026, SOL $104.92); average ~0.000008 SOL mid-Sep 2026; slot time 366ms ([communitynews.org](https://communitynews.org/sections/articles/solana-casinos/); [xgram.io](https://xgram.io/blog/solana-fees-explained-what-a-sol-transaction-really-costs-in-2026), index). Practical finality <1s.
- Randomness: **no Chainlink VRF** (described as "nascent on Solana" in a 2026 Solana ADR); options are Switchboard on-demand VRF (~0.002 SOL/req, TEE-attestation-based verification per a community comparison table), ORAO (multi-sig), Pyth Entropy (commit-reveal service), or the SlotHashes sysvar for low-stakes use ([solana-vrf comparison](https://github.com/coderpepe/solana-vrf); [ADR 0004](https://github.com/rob-9/proof-of-thought/blob/HEAD/docs/adr/0004-pyth-entropy-vrf.md); [agent-skills](https://github.com/solonnikov/agent-skills/blob/HEAD/skills/solana-program-scaffold/references/randomness.md), index).
- Minuses: base58/ed25519 address scheme breaks the clean 16^-k math; no EVM ticket tooling; weaker VRF story than EVM chains. Verdict: best raw latency/fees, wrong cryptography/tooling for this design.

**Aptos / Sui (native-randomness candidates).**
- Aptos: native on-chain randomness ("Aptos Roll") via weighted VRF/DKG among validators — instant, unbiasable under <50% malicious stake, callable from Move ([AIP-41](https://github.com/aptos-foundation/aips/blob/HEAD/aips/aip-041-move-apis-for-public-randomness-generation.md); [AIP-79](https://github.com/aptos-foundation/aips/blob/HEAD/aips/aip-079-implementation-of-instant-on-chain-randomness.md), index).
- Sui: `sui::random::Random` shared object (0x8), DKG-based, usable only in `entry` functions to block test-and-abort ([mystenlabs/skills](https://github.com/mystenlabs/skills/blob/HEAD/sui-overview/ecosystem.md), index).
- Verdict: the best *native* randomness infrastructure in the industry — but non-EVM address schemes and Move VM mean the secp256k1/keccak wallet-mining construction does not transfer. Revisit only if the design is ever ported off EVM.

### A.3 Comparison table (2026 figures; index sources)

| Chain | Addr scheme / P(prefix k) | Claim-tx cost (est.) | VRF | On-chain drand verify | Block time / finality |
|---|---|---|---|---|---|
| Ethereum L1 | secp256k1+keccak, 16^-k | ~$0.18–0.40 | VRF v2.5 (20%/24%) | Yes (EIP-2537) | 12s |
| **Base** | secp256k1+keccak, 16^-k | **~$0.002–0.01** | VRF v2.5 (50%/60%), Supra | **Yes (Isthmus)** | 2s / sub-s soft |
| Arbitrum One | secp256k1+keccak, 16^-k | ~$0.005–0.05 | VRF v2.5 | Likely (ArbOS Pectra)* | ~250ms / sub-s soft |
| Optimism | secp256k1+keccak, 16^-k | ~$0.001–0.01 | VRF v2.5 | Yes (Isthmus) | 2s / sub-s soft |
| Polygon PoS | secp256k1+keccak, 16^-k | ~$0.001–0.01 | VRF v2.5 (70%/84%) | No (no EIP-2537 cited) | ~2s / ~5s |
| Solana | ed25519/base58, ~58^-k messy | ~$0.0005 | Nascent; Switchboard/ORAO/Pyth | N/A | ~400ms / <1s |
| Aptos/Sui | ed25519-based | low | **Native (wVRF/DKG)** | N/A | <1s |

\* Arbitrum EIP-2537 support not directly confirmed by a source I found — flagged as thin evidence; Arbitrum has tracked L1 Pectra parity via ArbOS upgrades, but I could not cite it.

### A.4 Recommendation

**ONE recommended chain: Base.**
1. Clean probability math (EVM reference scheme, 16^-k).
2. Cheapest claim verification among credible EVM chains (~$0.002–0.01 derived estimate), so even low-tier wins are worth claiming on-chain.
3. Chainlink VRF v2.5 live (min confirmations 0) as a fallback rail; Supra dVRF also available.
4. EIP-2537 BLS precompiles live via Isthmus → trustless on-chain drand quicknet verification (~110k gas, amortized once per epoch).
5. 2s blocks + sub-second soft confirmations fit a game loop; ERC-4337 paymasters + Coinbase Smart Wallet enable sponsored (gasless) claims at ~$1–10 per 1,000 txs.
6. Largest L2 distribution (Coinbase funnel).

**Runner-up: Arbitrum One** — lowest EVM latency (~250ms blocks), BOLD fraud proofs, VRF live, deep DeFi liquidity; costs modestly more per claim. Choose it over Base only if sub-second block cadence matters more than per-claim cost. (Optimism is the honorable mention for absolute fee minimization.)

---

## PART B — BINDING ROLLS TO REAL GAMEPLAY

### B.1 Commit-reveal for roll entitlements

**Construction.** Each side commits before the other reveals: server commits `H(serverSeed)` (or the player commits `H(actionLog)`), then reveals are exchanged in a fixed order with timeouts; the roll seed = combination (e.g., XOR/hash) of both openings.

**What breaks on abort (selective-abort attacks).** The party that reveals *last* sees the outcome first and can abort (refuse to reveal) when it loses:
- *Player-side abort:* the last revealer computes the result from the other's opening and withholds their own opening if unfavorable. Standard mitigations: (a) **aborter-loses rule** — non-reveal after commit forfeits (used in the Tyche 2-party lottery: "if one party does not open, the other automatically wins") ([arXiv 2409.03464](https://arxiv.org/html/2409.03464v1), index); (b) **deposits/slashing** — forfeit a bond on non-reveal ([cryptonewsbytes](https://cryptonewsbytes.com/ethereum-smart-contract-randomness-bug-how-ethereums-first-lottery-in-2015-had-no-winners/), index); (c) **randomized reveal order** (Commit-Reveal²) to blunt the last-mover edge ([arXiv 2504.03936](https://arXiv.org/pdf/2504.03936), index).
- *Server/operator-side abort:* whoever controls redraw/VRF-subscription funding can trigger redraws before the oracle responds — a selective-abort capability. Mitigations: separate the subscription funder from the redraw initiator; enforce redraw cooldowns longer than the max oracle pending window (24h for Chainlink VRF) ([skillhub audit notes](https://github.com/abhishek1kr/skillhub/blob/HEAD/library/ai-security/nft-gaming/SKILL.md), index).
- *Same-tx selective revert:* if randomness and the state-changing effect (e.g., mint) happen in one transaction, a contract wallet can observe the outcome and revert via `onERC721Received`. Mitigation: split commit and reveal across transactions; use `_mint` not `_safeMint` ([skillhub](https://github.com/abhishek1kr/skillhub/blob/HEAD/library/ai-security/nft-gaming/SKILL.md), index).

**Occam's verdict for this game:** commit-reveal *between player and server* is the wrong tool — it adds rounds, timeouts, and abort-handling complexity to every gameplay action. The strictly simpler construction achieving the same binding is the **timing rule**: the seed is defined as the first drand round *after* the epoch closes, so during gameplay the seed does not exist yet and neither side can grind or abort on it (cf. the scadium ADR: "the round number is a pure function of the close time… its value does not exist until after the betting is over") ([scadium ADR](https://github.com/mcftc/scadium/blob/HEAD/docs/adr/0004-public-randomness-beacon.md), index). Commit-reveal is only needed if you insist on player-supplied entropy; with a public beacon you don't.

### B.2 VRF-gated rolls

**How it would work:** a VRF request is issued per epoch (or per high-value action); the returned random word(s) gate roll rights — e.g., `seed_epoch = VRF_output`, rolls derive from it. The request must be made *after* the action window closes (requestConfirmations ≥ 0 on Base; the output is unpredictable to the requester regardless of timing).

**Chainlink VRF v2.5 (current):** subscription and direct-funding on all supported networks; v2.5 changes: `setCoordinator` upgradeability, native-token payment, percentage premiums replacing flat LINK fees ([docs.chain.link migration](https://docs.chain.link/vrf/v2-5/migration-from-v2), index). Billing: subscription = post-fulfillment, charged **actual** callback gas used; direct funding = upfront, charged the **full configured callback gas limit** + `coordinator_flat_fee` (millionths of LINK) + wrapper overhead; insufficient balance reverts ([chainlink-agent-skills billing](https://github.com/smartcontractkit/chainlink-agent-skills/blob/HEAD/chainlink-vrf-skill/references/billing.md), index). Premiums: Ethereum 20% LINK / 24% native; Base 50% / 60%; Polygon 70% / 84% ([docs.chain.link](https://docs.chain.link/vrf/v2-5/supported-networks), index). Reported latency ~2s average (secondary source, thin evidence).

**Cost model choice:** for one seed per epoch, **direct funding** is simpler (no subscription management; the game operator's contract pays per request). Derived estimate at Base gas prices: cents per request (gas × 1.6 premium) — cheap at hourly cadence (~$0.25–1/day order of magnitude; labeled estimate). **Subscription** wins if many consumer contracts/requests share funding (one balance, actual-gas billing).

**drand vs VRF for this design:** drand quicknet is free, 3s cadence, publicly verifiable, and verifiable *on-chain* via EIP-2537 (~110k gas, cached per epoch) — no per-request oracle fee and no subscription operator. VRF's edge is push-to-chain convenience and no BLS precompile dependency. **Recommendation: drand as primary (it was the prior iteration's choice and on-chain verification is now cheap), Chainlink VRF as fallback** if drand liveness ever degrades (precedent: Cloudflare relay outage March 2025 degraded one HTTP relay for ~a week, though the beacon itself kept producing and other relays stayed up — [ChainSafe/forest PR](https://github.com/ChainSafe/forest/pull/5364), index).

**Supra dVRF:** available as a third rail (claims 80+ chains, pull/push models, low latency); adds another trust domain — keep as contingency, not primary.

### B.3 Server-signed action attestations (EIP-712 tickets)

**What must be signed.** A ticket is a bearer token: whoever holds a valid ticket can claim its roll. The signed struct must pin:
- `player` (S — the registered secp256k1 key/address),
- `epoch` (monotonic game epoch id),
- `seq` (per-player, per-epoch strictly increasing action sequence number),
- `nonce` (unique per ticket; doubles as the on-chain nullifier preimage),
- `pHash = keccak256(P)` — the derived roll public key (phase-2 ticket; see C.1), so the ticket pins the exact derivation,
- `expiresAt` (deadline),
and the EIP-712 **domain separator** must bind `chainId` + `verifyingContract` (+ name/version) so tickets cannot replay across chains or contract deployments. The contract consumes each ticket's nullifier exactly once. Standard pattern (nonces, expiry, domain separation, rejecting high-`s` malleability and `address(0)`) is documented in the [EIP-712 skill reference](https://github.com/mystic0xx/awesome-solidity-smart-contracts-skills/blob/HEAD/skills/eip712-signature-verification/SKILL.md) (index); OpenZeppelin's `ECDSA.recover` enforces lower-half `s`.

**Ticket nonces and epochs.** Nonce discipline: `nonce = keccak256(S, epoch, seq)` is deterministic and unique — no server-side nonce registry needed; the contract tracks `used[nullifier]`. Epochs: gameplay is divided into fixed epochs (e.g., 1 hour). Tickets are only issued during their epoch; claims only accepted after the epoch's seed finalizes and before `expiresAt`. Epoch rotation bounds the damage of any key compromise and lets caps/rate-limits reset cleanly.

**What this prevents:** replay (nullifier + domain separator), duplication (one claim per nullifier), offline forgery (attacker cannot forge the server's ECDSA signature), cross-player theft (ticket names S; claim auth must come from S). **What it does not prevent:** the server issuing tickets for fake actions (attestation trust — see C.4), or the server censoring a player's tickets (griefing; mitigations in C.4).

### B.4 Linear key-derivation binding: P = S + H(seed ‖ action_seq)·G

**Why a bare signature from a winning key does NOT prove legitimate derivation (design analysis).** After the seed is public, the player (who knows `s`) can compute `t_i = H(seed‖i)` for *arbitrary* `i` — including action sequences they never performed — derive `P_i = S + t_i·G` and `p_i = s + t_i`, and produce a perfectly valid signature "from the winning key." The signature proves knowledge of `p_i`, i.e., proves the signer knows `s` — but it says **nothing about whether action `i` happened**. Verification of the signature checks a self-chosen statement; the grinding attack (compute millions of `i` offline, keep the winners) is wide open. This is the exact failure the mission names, and no signature scheme alone fixes it: Schnorr, ECDSA, or otherwise, a signature cannot attest to gameplay that never occurred.

**Known fixes, compared:**
1. **Tickets binding seq to attested actions (recommended).** The server signs `(S, epoch, seq)` *before the seed exists* (phase-1 ticket) and `(S, epoch, seq, P)` after (phase-2). Offline grinding fails at the ticket gate: no ticket, no claim — regardless of how many keys the player derives. The signature's job shrinks to what signatures are actually good at: proving the claimer controls S. This is the Occam answer: it reuses boring, audited primitives (`ecrecover` × 2) and adds no proving infrastructure.
2. **One-time/rotated server secrets.** Replace public `seed` with `H(serverSecret_epoch ‖ …)`, committed before the epoch and revealed after. Closes player-side grinding (players can't derive until reveal) — but it hands the server an *undetectable bias* capability: the server can grind `serverSecret` offline to favor or punish specific players, since it chooses the secret. Strictly worse trust-wise than a public beacon; reject for the primary design (usable only as a fallback rail with the bias caveat disclosed).
3. **ZK proof of correct derivation.** Prove in zero knowledge: "I know `(s, seq)` such that `P = S + H(seed‖seq)·G`, `seq` is covered by a valid server ticket, and I authorize this claim" — without revealing `s`. Practical 2026 options: **SP1** (Succinct; Plonky3-based, fastest prover in several public benchmarks, strong dev adoption) vs **RISC Zero** (own STARK; lower memory on large programs; Boundless proof market) — both production-grade Rust zkVMs in an active benchmarking war as of Q1 2026 ([eco.com](https://eco.com/support/en/articles/11803106-what-is-risc-zero-zkvm-verifiable-computation); [zkvm-fib-bench](https://github.com/mariari/zkvm-fib-bench), index); **Circom** (hand-written circuits; mature but far more engineering effort for secp256k1 arithmetic). Verdict: viable but heavy — a prover service, verifier contract, and trusted circuit audit — to prove a statement the two-phase ticket already establishes with two `ecrecover`s. Keep ZK as the *fallback path* if the ticket server ever censors phase-2 issuance (player self-proves derivation instead of waiting for the server), not the primary path.
4. **Schnorr-based proof of discrete-log relation.** Note: EVM has **no Schnorr (BIP-340) precompile**; the practical EVM instantiation of "proof of knowledge of the derived key" is an ECDSA signature from `P` verified via `ecrecover`. As shown above, it proves knowledge of `s` — useful as a *claim-authorization* primitive — but it is not a fix for grinding. The prior iteration's "Schnorr-style proofs binding each claim to the registered key" maps cleanly onto: **claim authorization signed by S (EIP-712)**, which binds the payout to the registered key while keeping `s` in the user's wallet (a signature *from P* would instead force `s` into the hot game client — worse key hygiene; see C.3).

### B.5 Rate limiting and Sybil resistance

Layers, cheapest first:
1. **Per-account/per-epoch caps enforced on-chain.** The claim contract (or ticket contract) enforces `maxClaims[S][epoch]`; the ticket server will not issue more than `maxTickets` per (S, epoch). On-chain enforcement is trustless; off-chain issuance caps are cheaper but rely on the server. Use both (server as primary, contract as backstop).
2. **Ticket costs / staking.** Require a small stake to register S (slashable on proven botting/fraud); or price tickets (pay-per-roll converts farming into a direct cost). Tradeoff: stake slashing needs an adjudication path for "proven botting" (subjective, off-chain detection); ticket pricing changes the game economy and can price out real players.
3. **Proof-of-personhood for high tiers.** World ID ("World"): ~18M Orb-verified humans across 160 countries, ~38M app downloads (Apr 2026); World ID 3.0 + 1,500-Orb rollout (Aug 2026) — but biometric-iris model is banned/under investigation in several jurisdictions ([proof-of-personhood wiki](https://github.com/sampraszheng/yxz/blob/HEAD/wiki/concepts/proof-of-personhood.md); [ainvest](https://www.ainvest.com/news/worldcoin-wld-faces-tokenomics-pressure-biometric-identity-adoption-2609/), index). Alternatives: Human Passport (~2M users, ZK credentials), BrightID, Idena — all much smaller. Field consensus is **tiered**: lightweight Sybil scores for low-stakes actions, escalating to World ID/KYC for valuable ones ([aboard research](https://github.com/ostin-pil/aboard/blob/HEAD/research/sybil-identity.md), index).
4. **Recommended posture:** on-chain per-epoch caps + registration stake for all players; gate only the *highest* rarity tiers (or high-frequency claimants) behind World ID / Human Passport. This prices out casual Sybils without making the core loop KYC-gated.

### B.6 Claim theft and front-running mitigations

- **Theft via mempool observation:** closed by binding, not by hiding. The claim transaction carries an EIP-712 authorization signed by S over `(nullifier, beneficiary, expiresAt)`. A front-runner who copies the transaction cannot change the beneficiary (signature won't verify) and cannot forge S's signature; replaying it identically just pays the same beneficiary. There is no price-sensitive MEV (payout per rarity tier is fixed), so there is nothing to extract by reordering.
- **Commit-reveal claims:** unnecessary here — they add a full round-trip of latency to fix a problem binding already solves (this is the Occam cut: don't build what a signature already does).
- **Stealing another player's rolls:** tickets name S; the claim contract requires `recover(claimAuth) == S == ticket.player`. An attacker holding someone else's winning ticket cannot satisfy the authorization check.
- **Sequencer-level front-running (L2):** Base/Arbitrum sequencers order transactions; a malicious sequencer could *delay* but not *steal* a claim (same binding argument). Note as residual trust.

---

## PART C — PUT IT TOGETHER

### C.1 Recommended end-to-end architecture (chain: Base)

**Actors & keys.** Player registers `S = s·G` (secp256k1) via the game client; `s` never leaves the player's wallet (MetaMask / Coinbase Smart Wallet / hardware). Game server holds `sk_server` (ECDSA); optionally N-of-M threshold ticket signers. Claim contract `WalletMine` on Base; item contract (ERC-1155, rarity-tiered ids).

**Epoch cadence.** Fixed epochs (e.g., 1 hour). `seed_E = drand quicknet randomness of round R_E`, where `R_E = first round with timestamp > closeTime_E` (pure function of close time; nobody can choose it). drand quicknet: 3-second rounds, League of Entropy mainnet, unchained BLS (G1 sigs), chain hash `52db9ba70e0cc0f6eaf7803dd07447a1f5477735fd3f661792ba94600c84e971` ([drand docs](https://docs.drand.love/blog/2023/10/16/quicknet-is-live/); [gay.jl issue](https://github.com/bmorphism/gay.jl/issues/220), index).

**Step 1 — Play (during epoch E).** Player performs a real in-game action. Game server verifies the action server-side and issues **phase-1 ticket**: EIP-712 signature over `Ticket1(S, epoch=E, seq, nonce)` with `seq` strictly increasing per (S, E). (`nonce = keccak256(S, E, seq)`.) The player cannot compute any roll outcome: `seed_E` does not exist yet. Selective-participation (only playing when the next roll would win) is impossible.

**Step 2 — Epoch close.** `closeTime_E` passes. `R_E` is now determined. The first claimer for the epoch submits the drand round as a *carried proof* `(R_E, sig, randomness)`; the contract verifies the BLS signature via EIP-2537 (~110k gas), checks `R_E` is the correct first-after-close round, caches `seed_E`. One verification per epoch, amortized over all claims (pattern: [future-drand](https://github.com/future-drand/future-drand/blob/HEAD/README.md), index).

**Step 3 — Phase-2 ticket (after close).** Anyone (server or player client) computes `t = keccak256(seed_E ‖ E ‖ seq)` — note the epoch in the hash for domain separation — and `P = S + t·G` (all public inputs; the client does this locally with any secp256k1 library). The server signs **phase-2 ticket**: EIP-712 over `Ticket2(S, epoch=E, seq, pHash=keccak256(P_uncompressed), expiresAt)`. The player client **verifies P locally** before accepting the ticket (recompute from public data); a lying server is detected immediately (griefing only — it cannot steal, it doesn't know `s`).

**Step 4 — Claim (player).** If `address(P)` matches a rarity prefix, the player signs EIP-712 `ClaimAuth(S, nullifier, beneficiary, expiresAt)` with their wallet (`s` never exported) and submits `claim(S, E, seq, P, ticket2Sig, claimAuthSig, beneficiary)`. Optionally the claim tx is sponsored via a paymaster (gasless UX).

### C.2 Exact on-chain claim verification steps

1. `require(block.timestamp > closeTime_E)` — epoch finalized; `seed_E` cached (verify-and-cache carried drand proof if first claim of the epoch: BLS pairing check, round-number check).
2. `t = keccak256(abi.encodePacked(seed_E, E, seq))`; (no EC math needed — P comes from the ticket).
3. Verify server ticket: `ECDSA.recover(_hashTypedDataV4(keccak256(abi.encode(TICKET2_TYPEHASH, S, E, seq, keccak256(P), expiresAt))), ticket2Sig) == SERVER` and `block.timestamp ≤ expiresAt`.
4. `addr = address(uint160(uint256(keccak256(P))))` where `P` is the 65-byte uncompressed key; `require((addr & prefixMask) == prefix)` for the claimed tier. Rarity math: tier-k prefix wins with probability 16^-k per roll.
5. `nullifier = keccak256(S, E, seq)`; `require(!used[nullifier])`; `used[nullifier] = true` (replay/duplication closed).
6. Verify claim auth: `ECDSA.recover(_hashTypedDataV4(keccak256(abi.encode(CLAIM_TYPEHASH, S, nullifier, beneficiary, expiresAt))), claimAuthSig) == S` (front-running/theft closed; also enforce per-epoch cap `claims[S][E] ≤ MAX`).
7. Mint/transfer the tiered item (ERC-1155) to `beneficiary`.

**Cost (derived estimate, Base):** two `ecrecover` (~6k gas) + `keccak256`s (~1k) + nullifier SSTORE (20k) + ERC-1155 mint (~50k) + tx overhead ≈ 150–250k gas → at the measured 2026-09-27 base fee of 0.005 gwei and ETH ≈ $2,350, **≈ $0.002–0.005 per claim** (labeled estimate; congestion moves it). Epoch seed verification (~110k gas) is paid once per epoch by the first claimer and amortized.

### C.3 What the player must never export or reuse

1. **Never export `s`** outside their wallet — the game client must not learn it; all player signatures go through the wallet (EIP-712). (This is why claim auth is signed by S, not by the derived key P — signing from P would force `s` into the hot client.)
2. **Never reuse a ticket/nullifier** — one claim per (S, epoch, seq), enforced on-chain.
3. **Never reuse `(epoch, seq)`** across epochs — epoch is in every hash and ticket.
4. **Never accept an unverified phase-2 ticket** — the client must recompute `P = S + H(seed‖E‖seq)·G` locally and check it matches the ticket's `pHash` before claiming.
5. **Never let tickets be transferable** — they are bound to S by signature checks, not by possession.

### C.4 Residual risks

1. **Attestation trust (fundamental).** "The player really did a+b+c" is attested by the game server; cryptography binds rolls to the attestation but cannot verify gameplay. A compromised/malicious server can mint tickets for fake actions (inflating supply) or censor real players' tickets (griefing). It **cannot steal** wins (doesn't know `s`) and its lies about P are client-detectable. Mitigations: N-of-M threshold ticket signers, server bond/slashing on proven fraud, public ticket-transparency log so players can audit issuance counts vs. claims.
2. **drand trust.** Unpredictability assumes a threshold of League of Entropy operators do not sign rounds early in private, and the beacon stays live (the [tlock analysis](https://github.com/systemslibrarian/crypto-lab-beacon-lock) states both assumptions explicitly, index). Fallback rail: Chainlink VRF v2.5 (live on Base).
3. **Sequencer trust.** Base's sequencer (Coinbase) orders txs and provides soft confirmations; it can delay/censor but, by the binding argument, cannot redirect payouts. 7-day withdrawal window applies only if items must exit to L1.
4. **Economic (not cryptographic) farming.** See C.5.
5. **Smart-contract risk.** The construction leans on `ecrecover`, EIP-712 domain separation, and the EIP-2537 wrapper — all standard, but the contracts need a professional audit; the future-drand registry is unaudited community code (flagged).

### C.5 The cheapest attack that remains

**Run a bot farm that actually plays.** The cryptography closes forgery, grinding-after-reveal, replay, theft, and front-running — but it cannot distinguish a human's action from a bot's *real* action attested by the game server. The cheapest viable attack: operate N accounts, each performing the minimum genuine actions to earn that epoch's capped tickets, and claim whatever wins. Cost to attacker ≈ (bot operating cost + any registration stake) × N; expected revenue ≈ N × tickets_per_epoch × Σ_tiers 16^-k_tier × value_tier. **The defense is economic, not cryptographic:** set per-epoch ticket caps, rarity tiers, and prize values so that expected value per account stays below botting cost, require a registration stake to raise N's cost, and gate the top tiers behind proof-of-personhood (World ID / Human Passport, B.5). If EV turns positive, farms will come — no signature scheme prevents it.

---

## Sources

Fee/chain data (all index, read 2026-10-03):
- https://livecryptoprices.com/blog/crypto-transaction-cost-calculator/ — 2026 cross-chain fee matrix
- https://coinlaw.io/ethereum-gas-fees-statistics/ — ETH gas 2026, L2BEAT per-op costs
- https://www.mdpi.com/2674-1032/5/3/81 and https://arxiv.org/pdf/2606.22206v1 — peer-reviewed-style fee/speed study, 2026Q1 medians
- https://cryptocompass.com/articles/base-token-no-date-no-quota-but-6-27-billion-dollars-on-the-chain — Base base-fee measurement 2026-09-27
- https://github.com/lordbasilaiassistant-sudo/theagentcafe/blob/HEAD/base-ecosystem-research.md — Base fee structure, paymaster sponsorship economics (Mar 2026)
- https://eco.com/support/en/articles/10273675-what-is-optimism-the-ethereum-l2-and-op-mainnet-explained — OP/Arbitrum/Base comparison (Sep 2026)
- https://coinlaw.io/polygon-statistics/ ; https://cryptonews.net/news/altcoins/33474999/ ; https://cryptocompass.com/articles/why-is-polygon-considered-one-of-crypto-s-most-enterprise-friendly-chains — Polygon fees/finality
- https://xgram.io/blog/solana-fees-explained-what-a-sol-transaction-really-costs-in-2026 ; https://communitynews.org/sections/articles/solana-casinos/ — Solana fees/slot times
- https://github.com/andginja/ethskills/blob/HEAD/skills/layer-2s/SKILL.md — Base/OP chain facts, Coinbase Smart Wallet
- https://github.com/fastlane-labs/atlas/blob/HEAD/docs/CHAIN_CONFIG.md — block times table
- https://github.com/htwtech/main_blog/blob/HEAD/solana-fees-priority-fees-transaction-cost.md — Solana fee mechanics

Randomness infrastructure (index, read 2026-10-03):
- https://docs.chain.link/vrf/v2-5/supported-networks — VRF v2.5 networks, coordinators, premiums
- https://github.com/smartcontractkit/chainlink-agent-skills/blob/HEAD/chainlink-vrf-skill/references/billing.md — VRF v2.5 billing formulas
- https://docs.drand.love/blog/2023/10/16/quicknet-is-live/ — drand quicknet GA (3s rounds)
- https://github.com/bmorphism/gay.jl/issues/220 — quicknet chain hash, endpoints
- https://github.com/future-drand/future-drand/blob/HEAD/README.md — on-chain drand verification via EIP-2537, carried proofs
- https://github.com/ChainSafe/forest/pull/5364 — quicknet relay degradation precedent (Mar 2025)
- https://github.com/mcftc/scadium/blob/HEAD/docs/adr/0004-public-randomness-beacon.md — "first round after close" seed pattern
- https://github.com/systemslibrarian/crypto-lab-beacon-lock — drand trust assumptions (liveness, no early signing)
- https://supra.com/vi/academy/chainlink-vrf/ — Supra dVRF coverage claims
- https://github.com/aptos-foundation/aips/blob/HEAD/aips/aip-041-move-apis-for-public-randomness-generation.md ; https://github.com/aptos-foundation/aips/blob/HEAD/aips/aip-079-implementation-of-instant-on-chain-randomness.md — Aptos native randomness
- https://github.com/mystenlabs/skills/blob/HEAD/sui-overview/ecosystem.md ; https://github.com/mystenlabs/skills/blob/HEAD/onchain-randomness/security.md — Sui randomness, entry-only/test-and-abort guidance
- https://github.com/coderpepe/solana-vrf ; https://github.com/rob-9/proof-of-thought/blob/HEAD/docs/adr/0004-pyth-entropy-vrf.md ; https://github.com/solonnikov/agent-skills/blob/HEAD/skills/solana-program-scaffold/references/randomness.md — Solana VRF options

Cryptography & protocol (index, read 2026-10-03):
- https://www.7blocklabs.com/blog/faster-proof-verification-on-ethereum-using-the-eip-2537-bls12-381-precompiles — EIP-2537 gas schedule
- https://github.com/ethereum-optimism/design-docs/blob/HEAD/security/fma-isthmus-l2-pectra.md — EIP-2537 on OP Stack (Isthmus)
- https://github.com/mystic0xx/awesome-solidity-smart-contracts-skills/blob/HEAD/skills/eip712-signature-verification/SKILL.md — EIP-712 replay-protection pattern
- https://cryptonewsbytes.com/ethereum-smart-contract-randomness-bug-how-ethereums-first-lottery-in-2015-had-no-winners/ — commit-reveal pitfalls, modern fixes
- https://arxiv.org/html/2409.03464v1 (Tyche) ; https://arXiv.org/pdf/2504.03936 (Commit-Reveal²) — abort-resistant lottery constructions
- https://github.com/abhishek1kr/skillhub/blob/HEAD/library/ai-security/nft-gaming/SKILL.md — VRF selective-abort/redraw vectors, same-tx reveal vectors
- https://eco.com/support/en/articles/11803106-what-is-risc-zero-zkvm-verifiable-computation ; https://github.com/mariari/zkvm-fib-bench — SP1 vs RISC Zero 2026
- https://github.com/sampraszheng/yxz/blob/HEAD/wiki/concepts/proof-of-personhood.md ; https://www.ainvest.com/news/worldcoin-wld-faces-tokenomics-pressure-biometric-identity-adoption-2609/ ; https://github.com/ostin-pil/aboard/blob/HEAD/research/sybil-identity.md ; https://github.com/worldcoin/developer-docs/blob/HEAD/world-id/credentials/1.mdx — proof-of-personhood landscape 2026

Thin or conflicting evidence (flagged in text): Arbitrum EIP-2537 support (not directly sourced); Polygon fee direction (sources conflict); VRF ~2s latency (single secondary source); claim-tx gas (derived estimate, not measured); VRF per-request cost on Base (derived from the billing formula, not a quoted figure).
