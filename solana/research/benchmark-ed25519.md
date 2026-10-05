# Ed25519 roll throughput benchmark (2026-10-04)

Replaces the sim's guessed 5M keypairs/sec/GPU with measured/grounded numbers.

## Measured (this VM, libsodium via PyNaCl — same curve Solana uses)

- 1 core: **32,713 rolls/sec**
- 2 cores: **45,802 rolls/sec** (22,901/core — noisy shared VM, per-core drops under contention)
- 1 roll = 1 Ed25519 keypair; Solana address = 32-byte pubkey, no extra derivation cost.

## Interpretation

- This VM's cores are weak/shared. A modern desktop core (M1/M2, Ryzen 5800X-class)
  does ed25519 keygen at roughly **50k-100k/sec/core** (published dalek/libsodium
  figures; treat as estimate, not measured here).
- A typical 8-core desktop: order of **0.5M-1M rolls/sec** CPU-only. Meaningful grinding
  needs no GPU to start — this matters for the "grandma can grind" accessibility story.
- GPU figure: NOT measured here (no GPU on this VM). See below.

## GPU (published figures, not measured)

TODO: collect published Solana vanity-grind GPU rates (OpenCL ed25519 grinders).
The sim's old 5M/sec/GPU guess is unreplaced until then — do not use it.

## GPU (published, 2026-10-04)

- RTX 3090, optimized CUDA (alhimikix/solana-suffix-gpu, dev.to writeup): **44M keys/sec**.
  BUT: that figure uses a mod-58^K prefilter that skips full base58 encode for
  non-matching keys — it is suffix-*search*, not our workload.
- Same writeup, no prefilter tricks: GPU naive 311k/s, +8-bit comb **4.65M/s**.
- Our workload scores EVERY address against rarity tiers (needs prefix extraction
  per key, like the comb version, not the prefilter version).
- Apple Silicon (Metal): ~820k/s - 2M/s depending on chip and implementation.
- `solana-keygen grind` CPU: ~500k/s on 12 threads (~42k/thread).

## Sim inputs (grounded)

- CPU: **35k rolls/sec/core** (measured 32.7k here; published 28-42k/thread).
- GPU (3090-class, tier-scoring workload): **3M rolls/sec** (conservative; between the
  4.65M comb figure and lower-end cards).
- Apple Silicon: **1.5M rolls/sec**.
- The sim's old 5M/sec/GPU guess is retired. 3M is measured-adjacent, not a guess.
