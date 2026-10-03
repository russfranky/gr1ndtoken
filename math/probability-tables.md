# Wallet-mining rarity probability tables

## Current system: 26 eight-hex-char patterns of a real Ethereum address (shipped POC)

Method: each roll draws 32 bytes from a CSPRNG as a secp256k1 private key,
computes the uncompressed public key, hashes it with keccak256, and takes the
last 20 bytes as the address (exactly how Ethereum derives addresses). The
first 8 hex characters of the address are classified into 26 pattern classes
over the exact 2^32 prefix space. Counts below are exact; they sum to
4,294,967,296 = 2^32.

| id | name | tier | count | p | 1 in N |
|---|---|---|---|---|---|
| perfect-eight | Perfect Eight | MYTHIC | 16 | 3.7253e-9 | 268,435,456 |
| perfect-run | Perfect Run | MYTHIC | 18 | 4.1910e-9 | 238,609,294 |
| double-quad | Double Quad | LEGENDARY | 240 | 5.5879e-8 | 17,895,698 |
| sig-7+1 | Seven of a Kind | LEGENDARY | 1920 | 4.4703e-7 | 2,236,962 |
| sig-6+2 | Six Plus Pair | LEGENDARY | 4800 | 1.1176e-6 | 894,785 |
| sig-4+4 | Quad Pair | LEGENDARY | 6960 | 1.6205e-6 | 617,029 |
| sig-5+3 | Full House Five | LEGENDARY | 13440 | 3.1292e-6 | 319,566 |
| mirror | Mirror | LEGENDARY | 65520 | 1.5255e-5 | 65,552 |
| twin-halves | Twin Halves | LEGENDARY | 65280 | 1.5199e-5 | 65,792 |
| sig-6+1+1 | Six Plus Singles | EPIC | 94080 | 2.1905e-5 | 45,649 |
| sig-5+2+1 | Five Plus | EPIC | 564480 | 1.3143e-4 | 7,608 |
| sig-4+2+2 | Quad Plus Pairs | EPIC | 665280 | 1.5490e-4 | 6,456 |
| sig-5+1+1+1 | Five of a Kind | RARE | 2446080 | 5.6952e-4 | 1,756 |
| sig-4+3+1 | Quad Plus Trip | RARE | 940800 | 2.1905e-4 | 4,565 |
| sig-3+3+2 | Double Trip | RARE | 940800 | 2.1905e-4 | 4,565 |
| sig-2+2+2+2 | Four Pair | RARE | 4499040 | 1.0475e-3 | 955 |
| sig-3+3+1+1 | Trip Pair Plus | RARE | 12230400 | 2.8476e-3 | 351 |
| sig-1+1+1+1+1+1+1+1 | Snowflake | RARE | 518918382 | 1.2082e-1 | 8.3 |
| sig-4+2+1+1 | Quad Plus | UNCOMMON | 18345600 | 4.2714e-3 | 234 |
| sig-4+1+1+1+1 | Quad | UNCOMMON | 36691200 | 8.5428e-3 | 117 |
| sig-3+2+2+1 | Trip Plus Pairs | UNCOMMON | 36691200 | 8.5428e-3 | 117 |
| sig-3+1+1+1+1+1 | Trip | UNCOMMON | 322882560 | 7.5177e-2 | 13.3 |
| sig-2+2+2+1+1 | Three Pair | UNCOMMON | 220147200 | 5.1257e-2 | 19.5 |
| sig-2+1+1+1+1+1+1 | One Pair | UNCOMMON | 1614412800 | 3.7588e-1 | 2.7 |
| sig-3+2+1+1+1 | Trip Plus Pair | UNCOMMON | 293529600 | 6.8343e-2 | 14.6 |
| sig-2+2+1+1+1+1 | Two Pair | COMMON | 1210809600 | 2.8191e-1 | 3.5 |

Tier totals: COMMON 28.19%, UNCOMMON 55.20%, RARE 12.57%, EPIC 0.03%,
LEGENDARY 0.0037%, MYTHIC 7.9e-7%.

Verification, 2026-10-03 (against the shipped artifact export):

- Canonical test vector: private key
  `0x0000000000000000000000000000000000000000000000000000000000000001`
  derives to `0x7E5F4552091A69125d5DfCb7b8C2659029395Bdf`, matching the known
  Ethereum derivation vector. This proves the pipeline is real wallet
  generation, not a stand-in.
- The 26 counts above were extracted from the shipped build and sum to
  exactly 2^32. A 1,000,000-prefix fuzz run against the shipped classifier
  produced zero unknown classifications.

## Proposed, not implemented: six leading-zero tiers (wallet-gen-v2.md)

The rebuild spec `research_notes/wallet-mining-poc/wallet-gen-v2.md` proposes
replacing the 26 patterns with six tiers based on the count k of leading zero
nibbles in the address. This was never implemented in the POC; the shipped
build uses the 26-pattern system above. The math, kept here for reference:

Because keccak256 output is modeled as uniform over nibbles, the address
nibbles are uniform and independent. P(exactly k leading zero nibbles) =
(1/16)^k * (15/16) = 15 * 16^-(k+1). The tier probabilities sum to exactly 1:
15/16 * (1 + 1/16 + 1/16^2 + ...) = 1.

| Tier      | k (leading zero nibbles) | P(exactly k)        | 1 in N    | Percent     |
|-----------|--------------------------|---------------------|-----------|-------------|
| COMMON    | 0                        | 0.9375              | 1.07      | 93.75%      |
| UNCOMMON  | 1                        | 0.05859375          | 17        | 5.859375%   |
| RARE      | 2                        | 0.0036621094        | 273       | 0.3662109%  |
| EPIC      | 3                        | 0.0002288818        | 4,369     | 0.0228882%  |
| LEGENDARY | 4                        | 1.43051e-5          | 69,906    | 0.0014305%  |
| MYTHIC    | 5 or more                | 9.53674e-7          | 1,048,576 | 0.0000954%  |

Math validation (not the shipped product), 2026-10-03:

- Monte Carlo: 20,000 real wallets generated in Node (secp256k1 plus
  keccak256, ~0.91 ms per wallet). Observed leading-zero counts: k=0: 18,771
  (93.86%, expect 93.75); k=1: 1,153 (5.77%, expect 5.86); k=2: 72 (0.36%,
  expect 0.37); k=3: 4 (0.02%, expect 0.023). All within sampling noise.
- Real generated examples from that run:
  k=0 `0x42f2e6a0de53ca55b3755ea1074fe8d6b0486c3d`,
  k=1 `0x0a2a987ec4bbfd9f7bf05f10116b3b0f48404fcc`,
  k=2 `0x00f43f5c2b9c0bdbd5ccbe6884f97b888ed1ee5b`,
  k=3 `0x000872f8887f051d30e320368ab6c7cee62e1718`.

Recalibration (the article's "multiple rolls reshape the experience"): for k
rolls per action, P(at least one tier-T hit) = 1 - (1 - p)^k, and expected
actions to first hit = 1/P. The POC computes this live from its shipped table.
## Superseded: 26 decimal digit patterns (v1 POC, decimal PRNG)

Method (kept for the record; replaced by the rebuild): 8 decimal digits from a
seeded PRNG, classified first-match-wins through positional rules
(all-same, run, double-quad, mirror, twin-halves) then a multiset-signature
ladder. Probabilities are exact counts over all 100,000,000 possible numbers.

| id | name | tier | p | 1 in N |
|---|---|---|---|---|
| perfect-eight | Perfect Eight | MYTHIC | 1.0e-7 | 10,000,000 |
| perfect-run | Perfect Run | MYTHIC | 6.0e-8 | 16,666,667 |
| double-quad | Double Quad | LEGENDARY | 9.0e-7 | 1,111,111 |
| sig-7+1 | Seven of a Kind | LEGENDARY | 7.2e-6 | 138,889 |
| sig-6+2 | Six Plus Pair | LEGENDARY | 1.8e-5 | 55,556 |
| sig-4+4 | Quad Pair | LEGENDARY | 2.61e-5 | 38,314 |
| sig-5+3 | Full House Five | LEGENDARY | 5.04e-5 | 19,841 |
| mirror | Mirror | LEGENDARY | 9.99e-5 | 10,010 |
| twin-halves | Twin Halves | LEGENDARY | 9.9e-5 | 10,101 |
| sig-6+1+1 | Six Plus Singles | EPIC | 2.016e-4 | 4,960 |
| sig-5+2+1 | Five Plus | EPIC | 1.2096e-3 | 827 |
| sig-4+2+2 | Quad Plus Pairs | EPIC | 1.4256e-3 | 701 |
| sig-5+1+1+1 | Five of a Kind | RARE | 2.8224e-3 | 354 |
| sig-4+3+1 | Quad Plus Trip | RARE | 2.016e-3 | 496 |
| sig-3+3+2 | Double Trip | RARE | 2.016e-3 | 496 |
| sig-2+2+2+2 | Four Pair | RARE | 5.1912e-3 | 193 |
| sig-3+3+1+1 | Trip Pair Plus | RARE | 1.4112e-2 | 71 |
| sig-1+1+1+1+1+1+1+1 | Snowflake | RARE | 1.814394e-2 | 55 |
| sig-4+2+1+1 | Quad Plus | UNCOMMON | 2.1168e-2 | 47 |
| sig-4+1+1+1+1 | Quad | UNCOMMON | 2.1168e-2 | 47 |
| sig-3+2+2+1 | Trip Plus Pairs | UNCOMMON | 4.2336e-2 | 24 |
| sig-3+1+1+1+1+1 | Trip | UNCOMMON | 8.4672e-2 | 12 |
| sig-2+2+2+1+1 | Three Pair | UNCOMMON | 1.27008e-1 | 8 |
| sig-2+1+1+1+1+1+1 | One Pair | UNCOMMON | 1.69344e-1 | 6 |
| sig-3+2+1+1+1 | Trip Plus Pair | UNCOMMON | 1.69344e-1 | 6 |
| sig-2+2+1+1+1+1 | Two Pair | COMMON | 3.1752e-1 | 3 |

Verification: `math/analyze.mjs` computes the analytic multiset-signature
probabilities (they sum to 1) and runs a 2,000,000-roll Monte Carlo with the
exact detector the v1 artifact shipped, reporting measured versus analytic
probability per rule. This system was superseded because the decimal PRNG
missed the point of the article: real wallet generation, rarity from the
address itself.
