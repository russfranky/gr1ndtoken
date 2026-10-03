# Wallet Mining Rarity POC

Interactive proof of concept for the wallet-mining mechanic described at
https://russfranky.substack.com/p/wallet-mining-in-gaming

## What it demonstrates

Every roll generates a real Ethereum wallet in the browser: 32 CSPRNG bytes
as a secp256k1 private key, public key derivation, keccak256, and the last 20
bytes as the address, with EIP-55 checksum formatting. Rarity comes from the
address itself: the first 8 hex chars are classified into 26 pattern classes
(6 aggregate tiers) with exact counts over the 2^32 prefix space
(see `../math/probability-tables.md`). The page also
includes a rarity spectrum, a mining log, a recalibration panel (rolls per
action, 1-(1-p)^k), a distribution lab that runs real keygen batches against
the exact expectations, a pattern atlas, and an offline RNG panel with a
determinism replay check. A known-vector proof (privkey 0x01) is shown on the
page.

## How to run

Open `wallet-mining-rarity-poc.html` in any modern browser. No server, no
build step, no network. The crypto libraries are inlined, so airplane mode is
safe and the page's own network-call counter should stay at 0.

## What it does NOT do

This is a demo of the rarity mechanic only. There are no on-chain claims, no
server-signed tickets, no drand seed, and no World ID checks. Those are the
integration plan in `../docs/` and are not implemented here. Nothing leaves
the browser; generated keys exist only in the page session.
