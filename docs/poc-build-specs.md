# POC build specs

Two build specs, in order. Spec v1 built the first POC around 8-digit decimal mine numbers from a seeded PRNG. Spec v2 rebuilt it around real Ethereum wallet generation, which is the point of the article. Only v2 is current.

---

## Spec v1: decimal mine numbers (SUPERSEDED)

Preserved verbatim as the historical record, with inline notes marking what the rebuild changed.

# Build spec: Wallet Mining Rarity POC

> [SUPERSEDED by spec v2: the v1 decimal PRNG mechanic was replaced with real Ethereum wallet generation. Kept here as the historical record.]

## 1. Concept
Proof of concept for https://russfranky.substack.com/p/wallet-mining-in-gaming ("Wallet Mining" for Online Games).
The article's idea: every game action mines numbers (wallet addresses); a number matching a
"rare" pattern means the player mined a rare item. Developers reshape the experience with
multiple rolls per action. This POC demonstrates the rarity-mapping mechanic: roll 8-digit
mine numbers, detect digit patterns, map each pattern to a tier on a full rarity spectrum,
and do it all with an offline-capable seeded RNG. The on-chain claim phase from the article
(signature, smart-contract verification, asset transfer) is OUT OF SCOPE. Say so in a footer note.

## 2. Page sections (top to bottom, single column, max-width ~880px)
1. Mine-number card (the hero; homage to the "Your X Number" dialog in the reference screenshot)
2. Rarity spectrum bar with a marker on the current roll's tier
3. Mining log (recent rolls) + session-best line
4. Recalibration panel ("rolls per action" slider; the article's "multiple rolls reshape the experience")
5. Distribution lab (run 10K / 100K / 1M rolls, histogram of observed vs expected tier shares)
6. Pattern atlas (table of all 26 patterns)
7. Offline RNG panel (algorithm, seed controls, determinism replay check, network-call counter)
8. Footer scope note + link to the article

> [SUPERSEDED by spec v2 section 3: the card now shows a real derived Ethereum address, not an 8-digit decimal number. Refresh became Roll/Reroll, and Share was removed.]

## 3. Mine-number card (match the screenshot's look)
- Dark rounded card (~28px radius, near-black #141416 on a #0a0a0b page).
- Title "Your Mine Number", small X (collapses the card to a slim bar; clicking the bar reopens).
- Subtitle: "Your number is your mined claim. Anyone holding it can verify the roll. Share only numbers you trust."
- Big number in an inset pill, format XXXX-XXXX, tabular monospace numerals, wide letter spacing, copy icon button.
- Below the number: tier pill (colored, see tokens) + pattern name + odds ("1 in 55,556").
- Toggles (green when on, like the screenshot): "Mining enabled" (off disables Refresh), "Hide number from others" (masks digits as XXXX-XXXX with bullets).
- Buttons row: "Refresh" (dark pill with a circular-arrows icon; re-rolls with a ~400ms digit-scramble animation) and "Share" (white pill, black text; copies "Mine number 4829-4829 (Twin Halves, Legendary, 1 in 10,101)" and briefly shows "Copied").
- "Session best" line: rarest pattern rolled this session, or "No rolls yet".

> [Spec v2 section 4 PROPOSED replacing the 26 decimal patterns with leading-zero tiers, but that proposal was never implemented. The shipped build keeps 26 pattern classes, now over eight hex chars of a real derived address. See math/probability-tables.md.]

## 4. Rarity engine (canonical; the page must implement EXACTLY this)
8 digits, each 0-9. Rules evaluate top down; the number takes the FIRST matching pattern.

```js
function classify(d){ // d = array of 8 digit ints
  const s = d.join('');
  if (/^(\d)\1{7}$/.test(s)) return 'perfect-eight';
  let up = true, dn = true;
  for (let i = 1; i < 8; i++){ if (d[i] !== d[i-1] + 1) up = false; if (d[i] !== d[i-1] - 1) dn = false; }
  if (up || dn) return 'perfect-run';
  if (d[0]===d[1] && d[1]===d[2] && d[2]===d[3] && d[4]===d[5] && d[5]===d[6] && d[6]===d[7] && d[0]!==d[4]) return 'double-quad';
  let pal = true; for (let i = 0; i < 4; i++) if (d[i] !== d[7-i]) pal = false;
  if (pal) return 'mirror';
  let tw = true; for (let i = 0; i < 4; i++) if (d[i] !== d[i+4]) tw = false;
  if (tw) return 'twin-halves';
  const c = {}; d.forEach(x => c[x] = (c[x]||0)+1);
  return 'sig-' + Object.values(c).sort((a,b)=>b-a).join('+'); // e.g. sig-4+2+2
}
```

Pattern table (id | display name | tier | exact probability | 1 in N | example number).
Probabilities are exact counts over all 100,000,000 possible numbers, verified by an
independent 2,000,000-roll offline simulation. Embed these exact values:

| id | name | tier | p | 1 in N | example |
|---|---|---|---|---|---|
| perfect-eight | Perfect Eight | MYTHIC | 1.0e-7 | 10,000,000 | 7777-7777 |
| perfect-run | Perfect Run | MYTHIC | 6.0e-8 | 16,666,667 | 1234-5678 |
| double-quad | Double Quad | LEGENDARY | 9.0e-7 | 1,111,111 | 1111-2222 |
| sig-7+1 | Seven of a Kind | LEGENDARY | 7.2e-6 | 138,889 | 7777-7774 |
| sig-6+2 | Six Plus Pair | LEGENDARY | 1.8e-5 | 55,556 | 5555-5519 |
| sig-4+4 | Quad Pair | LEGENDARY | 2.61e-5 | 38,314 | 8811-8833 |
| sig-5+3 | Full House Five | LEGENDARY | 5.04e-5 | 19,841 | 9999-9221 |
| mirror | Mirror | LEGENDARY | 9.99e-5 | 10,010 | 1234-4321 |
| twin-halves | Twin Halves | LEGENDARY | 9.9e-5 | 10,101 | 4829-4829 |
| sig-6+1+1 | Six Plus Singles | EPIC | 2.016e-4 | 4,960 | 7777-7712 |
| sig-5+2+1 | Five Plus | EPIC | 1.2096e-3 | 827 | 3333-3221 |
| sig-4+2+2 | Quad Plus Pairs | EPIC | 1.4256e-3 | 701 | 6633-6622 |
| sig-5+1+1+1 | Five of a Kind | RARE | 2.8224e-3 | 354 | 9999-9213 |
| sig-4+3+1 | Quad Plus Trip | RARE | 2.016e-3 | 496 | 7777-3331 |
| sig-3+3+2 | Double Trip | RARE | 2.016e-3 | 496 | 4445-5521 |
| sig-2+2+2+2 | Four Pair | RARE | 5.1912e-3 | 193 | 1122-3344 |
| sig-3+3+1+1 | Trip Pair Plus | RARE | 1.4112e-2 | 71 | 7778-8812 |
| sig-1+1+1+1+1+1+1+1 | Snowflake | RARE | 1.814394e-2 | 55 | 1023-9485 |
| sig-4+2+1+1 | Quad Plus | UNCOMMON | 2.1168e-2 | 47 | 5555-2318 |
| sig-4+1+1+1+1 | Quad | UNCOMMON | 2.1168e-2 | 47 | 2222-1837 |
| sig-3+2+2+1 | Trip Plus Pairs | UNCOMMON | 4.2336e-2 | 24 | 9998-8771 |
| sig-3+1+1+1+1+1 | Trip | UNCOMMON | 8.4672e-2 | 12 | 5551-2348 |
| sig-2+2+2+1+1 | Three Pair | UNCOMMON | 1.27008e-1 | 8 | 1122-3345 |
| sig-2+1+1+1+1+1+1 | One Pair | UNCOMMON | 1.69344e-1 | 6 | 1134-5789 |
| sig-3+2+1+1+1 | Trip Plus Pair | UNCOMMON | 1.69344e-1 | 6 | 4445-5123 |
| sig-2+2+1+1+1+1 | Two Pair | COMMON | 3.1752e-1 | 3 | 1123-4578 |

Tier totals (show in the spectrum section): MYTHIC 1.6e-7 (1 in 6,250,000) | LEGENDARY
3.015e-4 (1 in 3,317) | EPIC 2.8368e-3 (1 in 353) | RARE 4.430154e-2 (1 in 22.6) |
UNCOMMON 0.63504 (about 1 in 1.6) | COMMON 0.31752 (about 1 in 3.1).

## 5. Rarity spectrum bar
Six equal-width segments in tier order (Common to Mythic), tier colors, each labeled with
tier name and per-roll odds. A marker (caret) sits under the current roll's tier and moves on Refresh.

## 6. Recalibration panel
Log-scale slider "Rolls per action": 1 to 5000. For each tier compute chance of at least one
hit per action: P = 1 - (1 - p)^k. Table columns: Tier | Per-roll odds | Chance per action
| Expected actions to first hit (1/P, formatted as "1 in N"). This is the article's
"multiple rolls reshape the player experience", made concrete.

## 7. Distribution lab
Buttons: 10K, 100K, 1M rolls. Runs chunked (async, with a progress bar) on its OWN PRNG
stream (seeded from the session seed, so it never disturbs the mine-number stream).
Histogram: one row per tier, bar = observed share on a log scale, marker line = expected
share, plus observed counts. Caption: "Mythic lands about once per 6.25M rolls, so a 1M-roll
run usually sees zero. That absence is the design working." Also show the rarest pattern seen.

> [SUPERSEDED by spec v2 section 8: the panel now covers real CSPRNG mode plus a seeded demo mode, not mulberry32 alone.]

## 8. Offline RNG panel
- Algorithm note: "mulberry32, a tiny seeded PRNG. Every roll on this page comes from it. No server, no network."
```js
function mulberry32(seed){ let a = seed >>> 0; return function(){ a |= 0; a = (a + 0x6D2B79F5) | 0; let t = Math.imul(a ^ (a >>> 15), 1 | a); t = (t + Math.imul(t ^ (t >>> 7), 61 | t)) ^ t; return ((t ^ (t >>> 14)) >>> 0) / 4294967296; }; }
```
- Seed row: current seed shown as 8 hex chars; text input + "Apply seed" (resets the roll stream and clears the log); "New random seed" (from crypto.getRandomValues).
- "Replay check" button: re-instantiates the PRNG from the seed used for the logged rolls, regenerates them, and reports e.g. "10/10 rolls reproduced exactly". This proves determinism.
- Network-proof badge: wrap window.fetch, XMLHttpRequest.prototype.open, and WebSocket to count calls; display "Network calls made by this page: N" (expect 0) plus an online/offline indicator from navigator.onLine. Caption: "Turn on airplane mode. Everything still works."

## 9. Mining log
Last 12 rolls: number, pattern name, tier pill. Newest on top.

## 10. Design tokens and copy rules
- Page bg #0a0a0b, cards #141416, inset pill #232327, text #f5f5f5, secondary #a1a1aa.
- Toggle green #34d399. Tier colors: COMMON #9ca3af, UNCOMMON #4ade80, RARE #60a5fa, EPIC #c084fc, LEGENDARY #fbbf24, MYTHIC gradient #f43f5e to #e879f9.
- Font: system stack; tabular numerals for numbers.
- Copy rules: short sentences, plain words. NEVER use em-dashes; use commas, colons, or periods.
- Footer scope note: "Scope: this POC demonstrates the rarity-mapping mechanic from the article: numbers, patterns, and a full rarity spectrum on an offline RNG. On-chain claiming (message signing, smart-contract verification, asset transfer) is phase two of the article and is out of scope here." Then link: "Read the article: Wallet Mining for Online Games" -> https://russfranky.substack.com/p/wallet-mining-in-gaming
- Also link the article once near the top under the title.

## 11. Acceptance criteria
- Refresh produces XXXX-XXXX numbers; each shows the correct pattern per classify() and the exact probability from the table above.
- Built-in self-test (runs on load, logs to console): classify each of the 26 example numbers and assert it returns the listed id. Any failure must be visible in console.
- The distribution lab's observed shares converge toward the table values as N grows.
- Replay check reports full reproduction.
- Network-call counter stays 0 during normal use.
- No em-dashes anywhere in visible copy.


---

## Spec v2: real Ethereum wallet generation (PARTIALLY IMPLEMENTED)

The real-keygen half of this spec is what the shipped POC implements (CSPRNG
private keys, secp256k1, keccak256, EIP-55). The rarity half (section 4,
leading-zero tiers) was NEVER implemented: the shipped build classifies the
first 8 address hex chars into the same 26 pattern classes as v1, with exact
counts over the 2^32 prefix space. See math/probability-tables.md.

# Wallet-gen v2 spec: rebuild the POC around REAL Ethereum wallet generation

## Why
The v1 POC rolls 8-digit decimal numbers with a seeded PRNG and classifies digit patterns.
That misses the entire point of the article (russfranky.substack.com/p/wallet-mining-in-gaming):
every roll must GENERATE A REAL ETHEREUM WALLET (secp256k1 keypair -> keccak256 -> 20-byte
address) and rarity must come from the ADDRESS ITSELF, exactly the article's "rare prefix"
mechanic. Rebuild the core mechanic accordingly. Everything else (card chrome, spectrum bar,
mining log, recalibration panel, distribution lab, pattern atlas, offline RNG panel, scope
note, unix module structure, unit tests, no em-dashes in copy) stays, adapted as below.

## 1. Wallet module (new: `wallet.js`, pure functions, no DOM)
- Bundle and INLINE these two MIT libraries into the page (no CDN, no fetch; the offline
  requirement stands): `@noble/secp256k1` (getPublicKey) and `@noble/hashes` (keccak_256).
- `generatePrivateKey(random32)`: takes 32 bytes, returns them with validation (must be
  non-zero and < secp256k1 curve order n; if invalid, draw again).
- `addressFromPrivateKey(privBytes)`: `pub = getPublicKey(priv, uncompressed=false->65 bytes
  with 0x04 prefix)`; `hash = keccak_256(pub.slice(1))`; return `'0x' + hex(hash.slice(-20))`.
  This is EXACTLY how Ethereum addresses are derived. No shortcuts.
- Canonical test vector (must pass in unit tests AND the in-page self-test):
  private key `0x0000000000000000000000000000000000000000000000000000000000000001`
  -> address `0x7E5F4552091A69125d5DfCb7b8C2659029395Bdf`
  (verified against an independent Node run, 2026-10-03).
- Measured performance (Node, 2026-10-03): ~0.91 ms per wallet. Budget the UI on ~1-2 ms
  per roll in-browser. Interactive rolling is synchronous and instant; the lab chunks.

## 2. Private key sourcing (two modes, user-visible)
- REAL mode (default): 32 bytes from `crypto.getRandomValues`. This is what real wallets use.
- SEEDED DEMO mode: bytes drawn from the mulberry32(seed) stream, for the determinism replay
  check. The offline RNG panel explains both modes and which is active. Replay check:
  re-instantiate the seeded stream and regenerate the logged addresses; report n/n reproduced.

## 3. The mine card
- Title: "Your Mined Address". Subtitle: "This is a real Ethereum address, generated on
  this device. A rare prefix means a rare find. Anyone can verify it."
- Display the address abbreviated as `0x7E5F...95Bdf` (first 6 incl. 0x, ellipsis, last 4),
  tabular monospace, with a copy button copying the FULL address.
- Below: tier pill + pattern name + `% chance` + `1 in N` (keep the % display from the
  current build).
- Keep: Roll/Reroll button behavior, mining log, session best. Share stays removed.

## 4. Rarity tiers: leading-zero nibble count (PROPOSAL - never implemented)

> STATUS (verified 2026-10-03 against the shipped artifact export): this section
> is a proposal only. The shipped POC does NOT use leading-zero tiers; it
> classifies the first 8 address hex chars into 26 pattern classes with exact
> counts over 2^32 (see math/probability-tables.md, "Current system").
Count k = number of leading `0` nibbles in the 40-hex-char address (after 0x).
P(exactly k) = 15 * 16^-(k+1); P(k>=5) = 16^-5. Verified by a 20,000-wallet Monte Carlo
(2026-10-03): k=0: 93.86% (expect 93.75), k=1: 5.77% (5.86), k=2: 0.36% (0.39),
k=3: 0.02% (0.024). Embed these EXACT values:

| Tier      | Pattern            | P           | 1 in N    | %          | Real example (generated 2026-10-03)          |
|-----------|--------------------|-------------|-----------|------------|----------------------------------------------|
| COMMON    | No leading zero    | 0.9375      | 1.07      | 93.75%     | 0x42f2e6a0de53ca55b3755ea1074fe8d6b0486c3d |
| UNCOMMON  | 1 leading zero     | 0.05859375  | 17        | 5.86%      | 0x0a2a987ec4bbfd9f7bf05f10116b3b0f48404fcc |
| RARE      | 2 leading zeros    | 0.003662109 | 273       | 0.366%     | 0x00f43f5c2b9c0bdbd5ccbe6884f97b888ed1ee5b |
| EPIC      | 3 leading zeros    | 0.000228882 | 4,369     | 0.0229%    | 0x000872f8887f051d30e320368ab6c7cee62e1718 |
| LEGENDARY | 4 leading zeros    | 1.43051e-5  | 69,906    | 0.00143%   | (too rare to hand-pick; lab finds them)      |
| MYTHIC    | 5+ leading zeros   | 9.53674e-7  | 1,048,576 | 0.0000954% | (too rare to hand-pick; lab finds them)      |

Probabilities sum to exactly 1. The unit tests must assert this.

## 5. Pattern badges (flavor, do NOT affect tier; show the "different patterns" idea)
Detect on the 40-char hex and show as small badges under the address. No odds claimed.
- "Vanity word": contains `dead`, `beef`, `cafe`, or `face` (case-insensitive).
- "Bookends": first 2 hex chars equal the last 2.
- "Mirror ends": first 4 hex chars equal the reverse of the last 4.
- "Quad run": 4 or more identical nibbles consecutively anywhere.
- "Digits only": all 40 chars are 0-9 (no a-f).
- "Lucky 1337": contains `1337`.
The atlas gets a second table listing these badges with one-line rules (mark as illustrative, no probabilities).

## 6. Distribution lab
Real keygen. Batch buttons: 100 / 1,000 / 10,000 rolls, chunked with a progress bar
(10,000 takes roughly 10-20 seconds; that is fine and honest). Histogram of the six
tiers vs the EXACT expectations from section 4. Caption: "Prefix probabilities are exact
(16^-k), so even small runs validate the math. Mythic needs about a million rolls on
average; the lab is not expected to find one."

## 7. Recalibration panel
Unchanged, with the section-4 probabilities. P(at least one tier-T hit in k rolls) =
1-(1-p)^k, slider 1..5000 log. This is the article's "multiple rolls reshape the player
experience," now on real wallet math.

## 8. Offline RNG panel
Update: explains real mode (CSPRNG, what production wallets use) vs seeded demo mode
(mulberry32, for the replay check). Keep the network-call counter (expect 0; the noble
libraries are inlined, nothing is fetched). Keep the determinism replay check against
seeded mode.

## 9. Unit tests (extend the shipped suite; all green before finishing)
- Canonical vector: privkey 0x01 -> 0x7E5F4552091A69125d5DfCb7b8C2659029395Bdf.
- keccak_256("") == c5d2460186f7233c927e7db2dcc703c0e500b653ca82273b7bfad8045d85a470.
- Leading-zero classifier on synthetic addresses: '0x0000ab...' -> k=4 tier LEGENDARY, etc.
- Tier table sums to exactly 1.
- Seeded mode determinism: same seed -> same private-key byte sequence -> same addresses.
- Badge detectors on synthetic addresses (e.g., an address containing 'dead' gets the badge).
- Roll rejects invalid private keys (zero, >= curve order) and redraws.

## 10. Acceptance criteria
- Every Roll generates a REAL secp256k1 keypair and derives the address with keccak256;
  the in-page self-test proves it with the canonical vector on every load (console + a
  small "self-test passed" line in the offline panel).
- [Proposal acceptance, not shipped] Tier shown always matches the leading-zero count; % and 1-in-N match section 4.
- Lab histogram converges to the exact expectations as N grows.
- Airplane mode: everything works, network counter stays 0.
- No em-dashes in visible copy. Unix module structure kept (`wallet.js` added; classifier
  now classifies addresses, pure function).
