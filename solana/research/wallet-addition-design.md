# Wallet addition: send/receive design

**Status:** design doc, 2026-10-05. Not built. For the gr1ndtoken Solana track.
**Goal:** users can easily SEND and RECEIVE SOL + the mined SPL token, inside the
grind app, without the app becoming a custodian of their money.

## 1. Architecture options, ranked

### (a) Built-in wallet: the grind keypair IS the wallet

The user's vanity keypair holds SOL + GRIND directly. Claims mint to an ATA
owned by the mined address itself. One key for everything: mines, holds, sends.

- Normie simplicity: poor. Key backup is the cliff — "write down these 12
  words or lose everything" is exactly the UX crypto keeps failing at.
- Custody honesty: poor. The app holds the keys to the user's money. Any
  "we never see your keys" claim is false the moment the key is in our JS
  memory — which it must be, to sign.
- Implementation cost: highest. Full wallet: backup, restore, password
  recovery (impossible without weakening security), transaction history.
- Claim-flow fit: awkward. The program already separates payer/miner from the
  mined key (`process_claim` takes `miner` signer + `mined_pubkey` bytes).
  Collapsing them gains nothing on-chain.

### (b) Connect-external via Solana wallet-adapter (Phantom/Solflare)

The app never holds spendable keys. "Connect wallet" button normies already
know. All signing happens in the user's real wallet; the private key never
touches our code. The mined key is held **ephemerally in memory** only for the
seconds needed to build the claim's ed25519 precompile instruction, then
dropped (or offered to the trophy vault, see below).

- Normie simplicity: best. The connect button is the one crypto UX pattern
  that actually works.
- Custody honesty: best, by construction. "We cannot move your funds" is a
  true statement, not a promise.
- Implementation cost: lowest. `@solana/wallet-adapter-*` packages, a few
  days for connect + balances + send/receive.
- Claim-flow fit: good. The existing client already treats payer and mined
  key as separate signers/inputs — the adapter wallet becomes the payer and
  reward destination; the mined key signs only the claim message in memory.

### (c) Hybrid (RECOMMENDED): adapter for money, encrypted trophy vault for mined keys

Everything in (b), plus a local encrypted vault that stores **mined keypairs
as trophies** — the keeper vanity addresses from `research/vanity-hit-list.md`.
The vault never spends; it exists so rare addresses aren't lost and so claims
can be re-signed if a claim tx fails. Keys are encrypted at rest under a user
password; the app cannot spend from the vault without the password, and the
vault holds no SOL (so even a compromised vault password can't drain funds —
the trophies' value is the addresses themselves, plus any unclaimed rewards).

- Normie simplicity: good. Money UX is the familiar adapter flow; the vault
  is an opt-in "save this trophy" moment, like bookmarking.
- Custody honesty: good, if stated precisely: "Your money lives in your
  wallet, which we can't touch. Your trophy keys are encrypted on this device
  under your password — we can't read them either, but anyone with your
  device AND password can."
- Implementation cost: medium. (b) plus the vault (~1 week).
- Claim-flow fit: best. Matches the program's existing payer/mined-key
  separation exactly, and preserves the product thesis ("the miner keeps the
  vanity wallet") without making the app a bank.

**Ranking: (c) > (b) > (a).** Build (b) first, add the vault as the second
slice. Never build (a) — the app must not hold spendable keys.

## 2. Concrete design (hybrid)

### 2.1 Trophy vault: key storage

- **What goes in:** mined keypairs the user chooses to keep. Never auto-saved;
  the claim ceremony ends with "Save to trophy vault / Discard". Default to
  discard for non-keeper hits.
- **Scheme:** envelope encryption. Per-key data encryption key (DEK,
  XChaCha20-Poly1305 or AES-256-GCM via WebCrypto), DEK wrapped by a
  key-encryption key (KEK) derived from the user's vault password with
  Argon2id (memory-hard; fall back to PBKDF2-SHA256 ≥ 310k iterations where
  Argon2 isn't available). Envelope means password changes re-wrap DEKs
  instead of re-encrypting every key.
- **Where:** IndexedDB on this device only. No server exists in this
  architecture — there is nowhere to exfiltrate *to*, which is a feature.
  State it in the UI: "Stored on this device only."
- **Locking:** vault locks after 5 minutes idle and on tab hide. Lock =
  drop KEK/DEKs from memory (zeroize as far as JS allows). Unlock requires
  the password each time.
- **Import path from the grinder:** `grind.py` currently prints secrets to
  stdout (fix: `--out` writes to a file with `0600` perms, never stdout).
  The app imports via that file, then the user deletes it. No clipboard
  round-trip for full secrets.
- **What the vault is NOT:** not a spending wallet, not backed up anywhere,
  not recoverable if the password is lost. Say all three in plain words at
  creation time.

### 2.2 Balance display

- **Sources:** `getBalance` for SOL; `getParsedTokenAccountsByOwner` for SPL
  (parsed is fine for display; re-derive raw amounts from `tokenAmount`
  for anything that touches a transaction).
- **Scope:** show SOL + GRIND prominently; other SPL tokens in a collapsed
  "other tokens" list (don't drown the mining UX).
- **RPC:** user-configurable endpoint, default to a reputable public RPC.
  `confirmed` commitment for display.
- **Freshness:** websocket `accountSubscribe` on the connected wallet with a
  20s polling fallback (public websockets drop; the fallback is not optional).
  Always refresh after any send/claim, and expose a manual refresh.
- **Claim → balance:** the claim tx mints to the connected wallet's GRIND
  ATA (created on demand in the claim flow if missing). On confirmation,
  show "+N GRIND" with an explorer link, then refresh balances.

### 2.3 Send flow

1. **Input:** recipient address field. Validate with `new PublicKey(v)`
   in try/catch (zod refine, same pattern the ecosystem uses). Reject empty,
   reject self-send with a clear message (not a silent failure).
2. **Amount:** parse as decimal string → raw units via mint decimals with
   BigInt. Never float math on money. Show both: "10.5 GRIND (10,500,000
   base units)" is overkill — show "10.5 GRIND", compute in BigInt underneath.
   Pre-check: amount ≤ balance − rent-exempt minimum for SOL (don't let the
   user strand-drain their account below rent-exempt; the client code already
   learned this lesson the hard way on the fee side).
3. **Recipient ATA:** derive with `getAssociatedTokenAddressSync`. If the
   account doesn't exist, prepend `createAssociatedTokenAccountInstruction`
   (sender pays ~0.002 SOL rent) and **disclose it in the fee line**:
   "Network fee ~0.000005 SOL + 0.00204 SOL one-time account rent for the
   recipient." Surprise rent is the #1 normie complaint about SPL sends.
4. **Fee estimation:** build the full transaction, `getFeeForMessage`, show
   the number before confirm. Then `simulateTransaction` — abort with the
   simulation error in plain words if it fails.
5. **Confirm screen (mandatory, not skippable):** recipient address with
   first/last 8 chars emphasized, amount + token, total fee breakdown,
   "this cannot be undone" line. The wallet adapter (Phantom) shows its own
   approval popup after — that's the second pair of eyes, keep it.
6. **States:** pending → confirmed → finalized, with explorer link at each
   step. "Confirmed" is enough to show the new balance; say "finalized"
   when it lands (~30s).

### 2.4 Receive flow

- Show the connected wallet address with copy button + QR code. One tap to
  copy. Note under it: "Send SOL or any SPL token here. GRIND rewards from
  claims land here too."
- No receive-side transaction needed — receiving is passive. The hard part
  is making the address unmistakable (full string visible, not truncated,
  next to the QR).

### 2.5 Error states (all must be plain words, no codes)

| Condition | UX |
|---|---|
| Invalid address | Inline: "That doesn't look like a Solana address." Block send. |
| Insufficient SOL for fee | "You need ~0.00001 SOL for the network fee. You have X." |
| Insufficient token balance | "You have 42 GRIND, tried to send 50." |
| Recipient has no ATA | Disclosed as rent line (2.3), not an error. |
| RPC failure / timeout | "Couldn't reach the network. Check your connection or try another RPC endpoint." Retry button. Never auto-retry a send. |
| Simulation failure | Show the program's error in plain words; block send. |
| User rejects in wallet | Silent dismiss — not an error, just return to the form. |
| Stale blockhash | Rebuild and re-simulate automatically once, then surface. |

## 3. Security review (adversarial)

- **XSS = total compromise of in-memory keys.** Any injected script in the
  app can read the mined key during the claim ceremony and the vault when
  unlocked. Mitigations: strict Content-Security-Policy, zero third-party
  scripts, keys exist in memory only for the seconds of the claim ceremony
  and are zeroized after; vault auto-locks fast. This is the strongest
  argument for (b)/(c) over (a): less key material, less time exposed.
- **grind.py prints secrets to stdout today.** Terminal scrollback, shell
  history, log files — all become key stores. Fix before any wallet slice
  ships: `--out <file>` with `0600`, no secret on stdout, ever.
- **Clipboard attacks.** Malware swaps the recipient address between copy
  and paste. Mitigations: the mandatory confirm screen shows first/last 8
  chars large; optional saved address book so repeat sends don't re-paste.
- **RPC spoofing.** A malicious or compromised RPC can lie about balances
  (show inflated GRIND to induce a send, or hide one). Mitigations:
  reputable default endpoints, user-configurable RPC, and — critically —
  every send is simulated against the *same* RPC and then approved in the
  user's own wallet, which shows the real effects. Never display a balance
  as spendable without a fresh fetch at send time.
- **Fee griefing / drained-by-rent.** A user sending their full SOL balance
  strands the account below rent-exempt and the tx fails after they've
  mentally spent the money. Mitigation: pre-check amount ≤ balance −
  rent-exempt minimum (0.00089088 SOL for a base account) and say so.
- **Vault password brute force.** The encrypted vault sits in IndexedDB;
  anyone with device access can copy it and brute-force offline.
  Mitigations: Argon2id with real parameters (not the "fast" preset), and
  honest scoping — the vault holds trophies, not funds; recommend a
  hardware wallet for anything that matters. Never imply the vault is a
  bank vault.
- **Supply chain.** `@solana/wallet-adapter-*` and `@solana/web3.js` are
  trusted third parties now. Pin versions, review upgrades, `npm audit`
  in CI. A compromised adapter package is a straight path to draining
  every connected user.
- **Claim-message replay.** Already handled by the existing design: the
  signed message binds `program_id || mined_pubkey || matched`, so a claim
  signature can't be replayed against a different program or tier. Keep
  this property — the wallet must never sign a *narrower* message.
- **Physical device access.** Unlocked vault + device = keys. Auto-lock
  (5 min idle, on tab hide) is the mitigation; there is no better one for
  a local-first app. Say it plainly.

## 4. Build sequencing

1. **Slice 1 — connect + see + receive.** Wallet-adapter connect
   (Phantom first), SOL + GRIND balance display (websocket + polling
   fallback), receive screen (address, copy, QR). No sending. Shippable,
   demoable, near-zero custody risk.
2. **Slice 2 — send.** SOL first, then SPL. Validation, ATA handling with
   disclosed rent, fee estimation, simulation, mandatory confirm screen,
   pending/confirmed/finalized states. Fix `grind.py` stdout secret leak
   in the same slice.
3. **Slice 3 — claim into the connected wallet.** Wire the existing claim
   flow to the adapter: mined key signs the ed25519 message ephemerally in
   memory, adapter wallet pays fees and owns the reward ATA. Claim →
   "+N GRIND" → balance refresh.
4. **Slice 4 — trophy vault.** Encrypted IndexedDB vault, password setup
   with honest warnings, save/discard at claim time, auto-lock. Import
   from grinder `--out` file.
5. **Later, only if asked:** address book, custom RPC UI, transaction
   history, multi-wallet adapter list.

## 5. Open questions (owner only)

1. Is adapter-only-for-money a hard line, or do you want the app to ever
   hold spendable keys (the full built-in-wallet option (a))?
2. Phantom-first for the adapter, or multi-wallet (Solflare, etc.) from
   day one?
3. Who funds the user's first SOL — faucet link, skip it and let them
   figure it out, or something else? (Sending and claiming both need SOL
   for fees; a zero-SOL wallet is read-only until funded.)
