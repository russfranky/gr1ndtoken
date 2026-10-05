#!/bin/bash
# grindmine testnet deploy + e2e. Run once the throwaway wallet is funded.
# Usage: ./deploy_testnet.sh
set -e
export PATH="$HOME/.cargo/bin:$HOME/.local/share/solana/install/active_release/bin:$PATH"
DIR=~/workspace/wallet-mining-solana/program
KEYS=$DIR/keys
RPC=https://api.testnet.solana.com
CLIENT=$DIR/target/debug/grindmine-client

PROG_KEY=$DIR/target/deploy/grindmine_program-keypair.json
PROG_ID=$(solana address -k $PROG_KEY)
echo "program id: $PROG_ID"

echo "--- balance check ---"
solana balance --url $RPC $KEYS/throwaway.json

echo "--- deploy ---"
# Upgrade authority: the program stays UPGRADABLE by $KEYS/deploy.json until
# the authority is revoked on-chain (set to a burn address) or the program is
# redeployed with --final. Deleting the local keyfile does NOT revoke the
# on-chain authority (red-team C3) — never claim immutability from `rm`.
if [ "${FINAL:-0}" = "1" ]; then
  echo "FINAL=1: deploying as immutable (no upgrade authority)."
  solana program deploy $DIR/target/deploy/grindmine_program.so \
    --program-id $PROG_KEY \
    --final \
    --url $RPC --keypair $KEYS/throwaway.json
else
  echo "deploying upgradeable (testnet default); authority: $KEYS/deploy.json"
  solana program deploy $DIR/target/deploy/grindmine_program.so \
    --program-id $PROG_KEY \
    --upgrade-authority $KEYS/deploy.json \
    --url $RPC --keypair $KEYS/throwaway.json
fi

echo "--- init (mint + config/manifest) ---"
$CLIENT init --program $PROG_ID --payer $KEYS/throwaway.json --rpc $RPC

echo "--- claim with ground key 1 ---"
$CLIENT claim --program $PROG_ID --payer $KEYS/throwaway.json \
  --keyfile $KEYS/mined1.b64 --matched 4 --rpc $RPC

echo "--- negative: double claim (must fail) ---"
$CLIENT neg --program $PROG_ID --payer $KEYS/throwaway.json \
  --keyfile $KEYS/mined1.b64 --matched 4 --case double --rpc $RPC

echo "--- negative: bad signature (must fail) ---"
$CLIENT neg --program $PROG_ID --payer $KEYS/throwaway.json \
  --keyfile $KEYS/mined2.b64 --matched 4 --case badsig --rpc $RPC

echo "--- negative: pattern mismatch, matched=5 on 4-char key (must fail) ---"
$CLIENT neg --program $PROG_ID --payer $KEYS/throwaway.json \
  --keyfile $KEYS/mined2.b64 --matched 5 --case mismatch --rpc $RPC

echo "--- upgrade authority status ---"
echo "Program is UPGRADABLE by $KEYS/deploy.json unless FINAL=1 was set."
echo "Keyfile deletion does not revoke on-chain authority. Stating the"
echo "authority status honestly; immutability requires --final or an"
echo "on-chain authority burn, neither of which this script fakes."
echo DONE
