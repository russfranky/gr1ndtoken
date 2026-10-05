#!/bin/bash
# grindmine testnet deploy + e2e. Run once the throwaway wallet is funded.
#
# Usage:
#   ./deploy_testnet.sh
#   FINAL=1 ./deploy_testnet.sh                      # finalize (immutable) — see below
#   HITS_FILE=~/.config/grindmine/grind-hits.jsonl \
#   GRIND_ADDR1=<4-char hit> GRIND_ADDR2=<4-char hit> ./deploy_testnet.sh
#
# FINAL=1 deploys with --final (no upgrade authority). It is REFUSED unless
# initialize has already succeeded for this program id: finalizing before a
# successful init (e.g. a griefed config PDA) = dead launch.
set -euo pipefail
export PATH="$HOME/.cargo/bin:$HOME/.local/share/solana/install/active_release/bin:$PATH"
DIR=~/workspace/wallet-mining-solana/program
KEYS="$DIR/keys"
RPC=https://api.testnet.solana.com
CLIENT="$DIR/target/debug/grindmine-client"
SO="$DIR/target/deploy/grindmine_program.so"

PROG_KEY="$DIR/target/deploy/grindmine_program-keypair.json"
PROG_ID="$(solana address -k "$PROG_KEY")"
echo "program id: $PROG_ID"

echo "--- balance check ---"
solana balance --url "$RPC" "$KEYS/throwaway.json"

CFG_PDA="$("$CLIENT" config-pda --program "$PROG_ID")"
echo "config PDA: $CFG_PDA"

if [ "${FINAL:-0}" = "1" ]; then
  if ! solana account "$CFG_PDA" --url "$RPC" --output json >/dev/null 2>&1; then
    echo "REFUSING FINAL=1: config PDA $CFG_PDA does not exist." >&2
    echo "initialize must succeed BEFORE finalizing; a griefed PDA + --final = dead launch." >&2
    exit 1
  fi
  echo "FINAL=1: config exists, deploying as immutable (no upgrade authority)."
  solana program deploy "$SO" \
    --program-id "$PROG_KEY" \
    --final \
    --url "$RPC" --keypair "$KEYS/throwaway.json"
else
  echo "deploying upgradeable (testnet default); authority: $KEYS/deploy.json"
  solana program deploy "$SO" \
    --program-id "$PROG_KEY" \
    --upgrade-authority "$KEYS/deploy.json" \
    --url "$RPC" --keypair "$KEYS/throwaway.json"
fi

echo "--- verify deployed program ---"
TMPD="$(mktemp -d)"
trap 'rm -rf "$TMPD"' EXIT
solana program dump "$PROG_ID" "$TMPD/deployed.so" --url "$RPC" >/dev/null
if ! cmp -s "$SO" "$TMPD/deployed.so"; then
  echo "DEPLOYED BINARY DIFFERS from local build $SO" >&2
  exit 1
fi
echo "deployed binary matches local build."

SHOW_JSON="$(solana program show "$PROG_ID" --url "$RPC" --output json)"
ACTUAL_AUTH="$(python3 -c 'import json,sys; print(json.load(sys.stdin).get("authority") or "")' <<<"$SHOW_JSON")"
if [ "${FINAL:-0}" = "1" ]; then
  if [ -n "$ACTUAL_AUTH" ]; then
    echo "expected no upgrade authority with FINAL=1, found: $ACTUAL_AUTH" >&2
    exit 1
  fi
  echo "upgrade authority: none (immutable)."
else
  EXPECTED_AUTH="$(solana address -k "$KEYS/deploy.json")"
  if [ "$ACTUAL_AUTH" != "$EXPECTED_AUTH" ]; then
    echo "upgrade authority mismatch: on-chain=$ACTUAL_AUTH want=$EXPECTED_AUTH" >&2
    exit 1
  fi
  echo "upgrade authority: $ACTUAL_AUTH (as expected)."
fi

echo "--- init (mint + config/manifest, atomic) ---"
"$CLIENT" init --program "$PROG_ID" --payer "$KEYS/throwaway.json" \
  --cluster testnet --rpc "$RPC" --yes

# Claim keys come from the grinder's hits file (64-byte secrets, new contract).
HITS_FILE="${HITS_FILE:-$HOME/.config/grindmine/grind-hits.jsonl}"
ADDR1="${GRIND_ADDR1:?set GRIND_ADDR1 to a 4-char hit address from \$HITS_FILE}"
ADDR2="${GRIND_ADDR2:?set GRIND_ADDR2 to a second 4-char hit address from \$HITS_FILE}"

echo "--- claim with ground key 1 ---"
"$CLIENT" claim --program "$PROG_ID" --payer "$KEYS/throwaway.json" \
  --hits "$HITS_FILE" --address "$ADDR1" --matched 4 \
  --cluster testnet --rpc "$RPC" --yes

echo "--- negative: double claim (must fail AlreadyClaimed) ---"
"$CLIENT" neg --program "$PROG_ID" --payer "$KEYS/throwaway.json" \
  --hits "$HITS_FILE" --address "$ADDR1" --matched 4 --case double \
  --cluster testnet --rpc "$RPC" --yes

echo "--- negative: bad signature (must fail SignaturePubkeyMismatch) ---"
"$CLIENT" neg --program "$PROG_ID" --payer "$KEYS/throwaway.json" \
  --hits "$HITS_FILE" --address "$ADDR2" --matched 4 --case badsig \
  --cluster testnet --rpc "$RPC" --yes

echo "--- negative: pattern mismatch, matched=5 on 4-char key (must fail PatternMismatch) ---"
"$CLIENT" neg --program "$PROG_ID" --payer "$KEYS/throwaway.json" \
  --hits "$HITS_FILE" --address "$ADDR2" --matched 5 --case mismatch \
  --cluster testnet --rpc "$RPC" --yes

echo DONE
