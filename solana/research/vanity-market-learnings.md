# Learnings from vanity.market (2026-10-04)

Ethereum vanity address generator on Golem Network (distributed compute) + ARKIV.
https://vanity.market/

## 1. Interactive rarity demo — steal this
"See it in Action": type a hex prefix -> sample address with the match highlighted
+ "Difficulty: Approx. 1 in 65,536 combinations". The user sets the pattern, the
site shows the math. For grindtokens this is the rarity explorer: the
human-readable twin of the machine-readable manifest. Agents read the manifest;
humans play with the explorer. Same numbers underneath.

## 2. Rarity in plain language
"1 in N combinations", not 58^n or hashes/sec. Our tier communication should
lead with this phrasing everywhere (site, explorer, manifest human summary).

## 3. Our trust story is simpler than theirs
Their whole "Secure by Design" section exists because grinding on someone
else's computer creates a key-custody problem (solved via salt-splitting: the
final key is only ever assembled on your device). We don't have this problem:
miners grind their own keys locally. "No key custody question, ever" is a
one-line advantage worth stating.

## 4. Budget-first CLI maps to agents
Their CLI takes `--budget-limit 10` — compute spend as the primary input.
Agents think in budgets too. The manifest should speak budget-in, EV-out.

## 5. Provider leaderboard — maybe later
"See who computes the most hashes, offers the best prices." A grinder
leaderboard is a proven viral element in this space, but it cuts against the
anon launch. Parked, not rejected.

## Not taking
The visual design (plain dev-tool site). The ideas are the value.
