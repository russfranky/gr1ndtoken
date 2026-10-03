// Rarity math verification for the wallet-mining POC.
// 1) Analytic multiset-signature probabilities for 8 digits (must sum to 1).
// 2) Monte Carlo with the exact detector the artifact will ship (positional
//    rules first, then multiset ladder) -> measured residual probability per rule.
import { createHash } from 'crypto';

function mulberry32(seed) {
  let a = seed >>> 0;
  return function () {
    a |= 0; a = (a + 0x6D2B79F5) | 0;
    let t = Math.imul(a ^ (a >>> 15), 1 | a);
    t = (t + Math.imul(t ^ (t >>> 7), 61 | t)) ^ t;
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
  };
}

// ---- detector (MUST match the artifact's classify() exactly) ----
function classify(d) {
  const s = d.join('');
  if (/^(\d)\1{7}$/.test(s)) return 'OCTAD';
  let up = true, dn = true;
  for (let i = 1; i < 8; i++) {
    if (d[i] !== d[i - 1] + 1) up = false;
    if (d[i] !== d[i - 1] - 1) dn = false;
  }
  if (up || dn) return 'PERFECT_RUN';
  if (d[0] === d[1] && d[1] === d[2] && d[2] === d[3] &&
      d[4] === d[5] && d[5] === d[6] && d[6] === d[7] && d[0] !== d[4]) return 'DOUBLE_QUAD';
  let pal = true;
  for (let i = 0; i < 4; i++) if (d[i] !== d[7 - i]) pal = false;
  if (pal) return 'MIRROR';
  let tw = true;
  for (let i = 0; i < 4; i++) if (d[i] !== d[i + 4]) tw = false;
  if (tw) return 'TWIN_HALVES';
  const counts = {};
  d.forEach(x => counts[x] = (counts[x] || 0) + 1);
  return 'SIG:' + Object.values(counts).sort((a, b) => b - a).join('+');
}

// ---- analytic signature probabilities ----
function fact(n) { let r = 1; for (let i = 2; i <= n; i++) r *= i; return r; }
// all integer partitions of 8 with at most 10 parts
function partitions(n, max = n, prefix = []) {
  if (n === 0) return [prefix];
  const out = [];
  for (let c = Math.min(max, n); c >= 1; c--) out.push(...partitions(n - c, c, [...prefix, c]));
  return out;
}
function perm10(k) { let r = 1; for (let i = 0; i < k; i++) r *= (10 - i); return r; }
const sigs = partitions(8).filter(p => p.length <= 10);
let sum = 0;
const analytic = {};
for (const p of sigs) {
  const k = p.length;
  const aut = {};
  p.forEach(c => aut[c] = (aut[c] || 0) + 1);
  let autF = 1;
  Object.values(aut).forEach(m => autF *= fact(m));
  const digitAssign = perm10(k) / autF;
  let posF = fact(8);
  p.forEach(c => posF /= fact(c));
  const prob = digitAssign * posF / 1e8;
  analytic[p.join('+')] = prob;
  sum += prob;
}
console.log('signatures:', sigs.length, 'sum of analytic p =', sum.toFixed(12));

// ---- Monte Carlo ----
const N = 2000000;
const rng = mulberry32(0xC0FFEE);
const hits = {};
for (let i = 0; i < N; i++) {
  const d = [];
  for (let j = 0; j < 8; j++) d.push(Math.floor(rng() * 10));
  const r = classify(d);
  hits[r] = (hits[r] || 0) + 1;
}
const order = ['OCTAD', 'PERFECT_RUN', 'DOUBLE_QUAD', 'MIRROR', 'TWIN_HALVES',
  ...sigs.map(p => 'SIG:' + p.join('+'))];
console.log('\nrule | measured p | 1 in N | analytic sig p (for SIG rules)');
for (const r of order) {
  const c = hits[r] || 0;
  const p = c / N;
  const oneIn = p > 0 ? Math.round(1 / p).toLocaleString('en-US') : 'never seen';
  const sig = r.startsWith('SIG:') ? r.slice(4) : null;
  const ap = sig ? analytic[sig].toExponential(3) : '-';
  console.log(`${r.padEnd(16)} ${p.toExponential(3).padStart(10)}  1 in ${oneIn.padStart(14)}  analytic=${ap}`);
}
