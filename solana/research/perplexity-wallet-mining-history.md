# Perplexity research history — wallet mining (owner-provided 2026-10-04)

Owner's prior research from his Perplexity account (russwfranklin@gmail.com), three threads on the "Wallet Mining for Online Games" article. Saved verbatim-ish from his message; this is lineage/history, not current authority.

## THREAD A — "tell me if this would work" (article viability, ~1yr ago, 166 sources)

- Core viability: technically feasible. Addresses derive from ECDSA/secp256k1 keypairs, generation fully client-side/offline, prefix probabilities = natural rarity tiers (1-char 6.25%, 4-char 0.0015%).
- Compute: 8 hex chars < 1 min on high-end AMD GPU; each extra char 16x work; 5x470 + 1x480 rig ~440MH/s. Sweet spot 1–2 char prefixes (~12 rolls for 50% at 1 char, ~2,000 at 3 chars).
- Security: Wintermute lost $160M to poor randomness in a vanity generator — keys must be generated/secured properly. Discouraging key exports + issuing new primary address = sensible mitigation.
- Owner's own lines in the thread (important):
  - "Signatures are free? Are you really that dumb?" → signing is free/offline; on-chain verification via ecrecover costs gas.
  - **"You don't need the verification part...."** → on-chain verification isn't needed for the core loop. Client validates the pattern, records discovery, attaches item to account — no gas, no chain latency, works without a blockchain wallet.
  - Sweet spot 1–2 char prefixes; RollerCoin and WAX Mining Network cited as precedents; circumvention risk = players using external vanity generators.

## THREAD B — "does anything like this exist yet?" (deep research, Dec 2025, 10 turns, 115 sources; Jev's top pick)

- Prior-art verdict: nothing like it in production. Closest: vanity tools (VanityGen, Profanity — deprecated, security flaws, Wintermute) and PoW mining games (Alien Worlds, GoMining). None combine address generation with gameplay. CREATE2 could complement.
- Why it doesn't exist: security audit burden, compute inefficiency vs VRF/commit-reveal, regulatory uncertainty, UX friction, VRF "good enough".
- Tunable difficulty (5 patterns; owner picked #5): (1) hash puzzles H(playerId||actionId||nonce) with bit targets — smooth difficulty, millions/sec; (2) variable ticket counts per event; (3) bounded search per action; (4) off-chain RNG + on-chain verify; **(5) hybrid — keep the "your wallet found 0xLUCKY" fantasy, tuned RNG underneath. OWNER PICKED #5.**
- Shorter wallets: custom 32/64-bit game addresses H(playerId||nonce) truncated, custom alphabets, published derivation for verifiable fairness. No real funds at risk, smooth rarity curve, cheaper verification than ecrecover.
- Anti-exploitation: server-signed loot tickets (playerId, encounterId, nonce, maxRolls, ~60s expiry); roll binding candidate = H(ticket||attemptIndex) — rolls can't be reused/precomputed; one ticket/encounter; anomaly detection; hardware attestation for high-value drops.
- Precursor strings: encounter sequence (enter arena → trigger boss → survive phases → killing blow) builds signed eligibility proof; mining takes it as mandatory input; locally brute-forced hits unclaimable without it.
- Demos built (code not recoverable, descriptions only): HTML demo (precursor hash + bit slider + roll budget + SHA-256 mining), investor demo, YC proposal version, math proof doc (replay impossibility, 1/2^b fairness, farm bounds, $432k/month secondary volume per 10k players), Dwarf Fortress-themed demo.

## THREAD C — PoW progressive difficulty (Jun 2025, 38 sources)

- First-to-signal keypair discovery, each success extends required pattern. P = 1/16^n. **Bitcoin-style difficulty retarget from network participation.** Hardware tiers CPU/GPU→ASIC. Mining-market game theory (difficulty consolidates toward cost-advantaged miners). L2 for claims. Differentiator vs loot boxes: verifiable randomness no developer can manipulate.

## Jev's ranking (from the extraction)

Thread B highest by far, A moderate, C low. WARNING (Jev's): Thread B's ticket/precursor design is the direct ancestor of the October 2026 architecture (EIP-712 tickets, per-epoch addresses, drand) — read as history/lineage, NOT current authority, or it regresses decisions already made.

## Actionable pulls for the Solana anon launch (2026-10-04)

1. **Grind-rate calibration:** 440MH/s rig figure is ETH-address hashing (secp256k1 point mult), not raw hash. Ed25519 keygen is cheaper than secp256k1 — our sim's 5M keypairs/s/GPU guess should be revisited against Ed25519 benchmarks, not ETH figures. (Sim assumption flagged, not yet updated.)
2. **Claim-cost problem:** our sim found $0.24 claim costs truncate low tiers. The owner's own Thread-A position ("you don't need the verification part") suggests the question: do low-tier claims need individual on-chain verification, or can small wins accumulate client-side and settle in batches? OPEN design question — directly downstream of his stated instinct.
3. **Difficulty lineage validated:** Thread C's Bitcoin-style retarget + Thread B pattern #1 (bit targets, smooth difficulty) both point at the smooth difficulty adjustment the confidence loop already adopted in round 2. Independent lineage, same answer.
4. **Sweet-spot tiers:** 1–2 char prefixes hit in ~12 rolls — the low tiers must hit OFTEN for the loop to feel alive. Supports the diffuse/small-tier emphasis; against jackpot-only designs.
5. **Wintermute rule:** miner spec must mandate CSPRNG key generation. Non-negotiable, goes in the build spec.
6. **External-generator circumvention (Thread A):** flagged as a risk in the game context; in OUR context (permissionless mining) it's not circumvention — it's the whole game. No action.
