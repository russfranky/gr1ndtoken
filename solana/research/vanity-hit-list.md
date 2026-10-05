# Ultra-Rare Vanity Keeper List

Patterns worth KEEPING after the mining — not just claiming for rewards, but holding
as identity, flex, or resale. All are base58 prefixes (case-sensitive).
Base58 excludes `0`, `O`, `I`, `l` — anything containing those letters is impossible
(there is no `007`, no `MOON`, no `HODL`; that is why the launch address is `Gr1nd`).

## Rarity math

P(k-char prefix) = 58^-k. Expected time to hit:

| chars | combinations | @ 3M rolls/s (GPU) | @ 32.7k rolls/s (CPU) |
|-------|--------------|--------------------|-----------------------|
| 4 | 11.3M | ~4 sec | ~6 min |
| 5 | 656M | ~4 min | ~6 hrs |
| 6 | 38.1B | ~3.5 hrs | ~13.5 days |
| 7 | 2.21T | ~8.5 days | ~2.1 yrs |
| 8 | 128T | ~1.35 yrs | ~124 yrs |

Sweet spot for keepers: 6-char words (a GPU finds one in an afternoon).
7-char is a trophy. 8-char is win-the-lottery territory.

## Words of power

Real words, readable at a glance. The closer to a name or title, the more keepable.

- `WEALTH` (6) — the blunt one
- `TREASURY` (8) — trophy tier; whoever holds this holds the word
- `VAULT` (5) — short, perfect for a cold wallet
- `LEDGER` (6)
- `WALLET` (6) — the generic-flex
- `CASH` (4) — short and clean
- `RARE` (4) — self-describing
- `LEGEND` (6)
- `ETERNAL` (7) — trophy
- `HARVEST` (7) — trophy
- `STELLAR` (7) — trophy
- `ANARCHY` (7) — trophy
- `MANTRA` (6)
- `DHARMA` (6)
- `KARMA` (5)
- `SUTRA` (5)
- `GALAXY` (6)
- `NEBULA` (6)
- `QUASAR` (6)
- `PULSAR` (6)
- `LUNAR` (5)
- `DAWN` (4)
- `DUSK` (4)
- `ZEN` (3) — tiny, elegant

## Mythic / creature names

- `KRAKEN` (6)
- `CERBERUS` (8) — trophy
- `MEDUSA` (6)
- `WYVERN` (6)
- `PEGASUS` (7) — trophy
- `CENTAUR` (7) — trophy
- `ANGEL` (5)

## Blades and hunters

- `KATANA` (6)
- `DAGGER` (6)
- `SABER` (5)
- `HUNTER` (6)
- `SLAYER` (6)
- `REAPER` (6)
- `SPECTRE` (7) — trophy

## Crypto-native

Words the space already values. These read as insider flex.

- `WHALE` (5)
- `QUEEN` (5)
- `ALPHA` (5)
- `BETA` (4)
- `GAMMA` (5)
- `DELTA` (5)
- `SAFU` (4)
- `REKT` (4)
- `PUMP` (4)
- `DUMP` (4)
- `PWNED` (5)
- `HACK` (4)
- `STAKE` (5)
- `FARM` (4)
- `HANDS` (5) — diamond hands, minus the diamond
- `PAPER` (5)
- `BULL` (4)
- `BEAR` (4)
- `ATH` (3)
- `ATL` (3)
- `FUD` (3)
- `GM` (2) — the shortest flex on this list
- `PYTH` (4)
- `JUP` (3)
- `APEX` (4)
- `ATLAS` (5)
- `AURUM` (5) — latin for gold (since `GOLD` is impossible)
- `ARGENTUM` (8) — latin for silver; trophy
- `QWERTY` (6)
- `ASDF` (4)
- `GGWP` (4)

## Numbers

Digits allowed: 1-9 only (`0` is excluded — `1000`, `420`, `007` are all impossible).

- `777` / `7777` — the classic lucky hit
- `888` / `8888` — prosperity flex
- `1111` / `111111` — the purest repeat
- `9999`
- `1337` / `31337` — leet, the hacker's number
- `80085` — juvenile, will absolutely be kept
- `42` — the answer
- `69` — inevitable
- `404` — not found
- `314` — pi
- `2718` — e
- `1618` — golden ratio

## Repeats and patterns

Visually striking even without meaning. Repeats are the most obviously "mined" look.

- `AAAA` / `AAAAAA`
- `ZZZZ`
- `XXXX`
- `QQQQ`
- `ABBA`
- `KAYAK` (5) — palindrome
- `RACECAR` (7) — palindrome trophy

## Trophy tier (8-char, ~1.35 GPU-years each)

The ones that would headline a marketplace: `TREASURY`, `CERBERUS`, `ARGENTUM`.
Expected value says almost nobody finds one — that is exactly why they are trophies.

## Keeper's note

A kept vanity address is only as valuable as its key is secret. See the sealed-key
design note: claim-by-signature never exposes the key, and an optional attested
"sealed" tier can bind a public hardware attestation to ultra-rare hits.
