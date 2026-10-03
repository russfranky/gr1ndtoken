// Benchmark real ETH wallet generation: privkey -> secp256k1 pubkey -> keccak256 -> address.
// Also verifies the canonical test vector: privkey 0x01 -> 0x7E5F4552091A69125d5DfCb7b8C2659029395Bdf.
import { getPublicKey } from '@noble/secp256k1';
import { keccak_256 } from '@noble/hashes/sha3.js';
import { bytesToHex } from '@noble/hashes/utils.js';

function addressFromPriv(privBytes) {
  const pub = getPublicKey(privBytes, false); // uncompressed, 65 bytes, 0x04 prefix
  const hash = keccak_256(pub.slice(1));
  return '0x' + bytesToHex(hash.slice(-20));
}

// test vector
const tv = addressFromPriv(new Uint8Array(31).fill(0).length ? (() => { const b = new Uint8Array(32); b[31] = 1; return b; })() : null);
console.log('test vector privkey=0x01 ->', tv);
console.log('expected            -> 0x7E5F4552091A69125d5DfCb7b8C2659029395Bdf');
console.log('MATCH:', tv.toLowerCase() === '0x7e5f4552091a69125d5dfcb7b8c2659029395bdf');

// benchmark
import { randomBytes } from 'crypto';
const N = 2000;
const t0 = process.hrtime.bigint();
for (let i = 0; i < N; i++) addressFromPriv(randomBytes(32));
const t1 = process.hrtime.bigint();
const msPer = Number(t1 - t0) / 1e6 / N;
console.log(`\n${N} wallets in ${(Number(t1 - t0) / 1e6).toFixed(0)} ms => ${msPer.toFixed(3)} ms/wallet`);
console.log(`=> 100 rolls ~${(msPer * 100 / 1000).toFixed(1)}s, 1000 rolls ~${(msPer * 1000 / 1000).toFixed(0)}s, 10000 rolls ~${(msPer * 10000 / 1000).toFixed(0)}s`);

// leading-zero distribution sanity over 20k wallets
const buckets = {};
const M = 20000;
for (let i = 0; i < M; i++) {
  const a = addressFromPriv(randomBytes(32)).slice(2);
  let k = 0;
  while (a[k] === '0') k++;
  buckets[k] = (buckets[k] || 0) + 1;
}
console.log('\nleading-zero nibbles over', M, 'real wallets:', JSON.stringify(buckets));
console.log('expected P(k>=1)≈6.25%, P(k>=2)≈0.39%, P(k>=3)≈0.024%');
