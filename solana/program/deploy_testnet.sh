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
solana program deploy $DIR/target/deploy/grindmine_program.so \
  --program-id $PROG_KEY \
  --upgrade-authority $KEYS/deploy.json \
  --url $RPC --keypair $KEYS/throwaway.json

echo "--- init (mint + config/manifest) ---"
$CLIENT init --program $PROG_ID --payer $KEYS/throwaway.json --rpc $RPC

echo "--- claim with ground key 1 ---"
KEY1=$(cat $KEYS/mined1.b64)
KEY2=$(cat $KEYS/mined2.b64)
$CLIENT claim --program $PROG_ID --payer $KEYS/throwaway.json \
  --key "$KEY1" --matched 4 --rpc $RPC

echo "--- negative: double claim (must fail) ---"
$CLIENT neg --program $PROG_ID --payer $KEYS/throwaway.json \
  --key "$KEY1" --matched 4 --case double --rpc $RPC

echo "--- negative: bad signature (must fail) ---"
$CLIENT neg --program $PROG_ID --payer $KEYS/throwaway.json \
  --key "$KEY2" --matched 4 --case badsig --rpc $RPC

echo "--- negative: pattern mismatch, matched=5 on 4-char key (must fail) ---"
$CLIENT neg --program $PROG_ID --payer $KEYS/throwaway.json \
  --key "$KEY2" --matched 5 --case mismatch --rpc $RPC

echo "--- burn upgrade authority (immutable) ---"
rm -f $KEYS/deploy.json
echo "deploy.json deleted; program is now effectively immutable."
echo DONE
