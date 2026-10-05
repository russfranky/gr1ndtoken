//! grindmine-program: wallet-mining claim verifier (testnet demo).
//!
//! Miners grind Ed25519 keypairs OFF-CHAIN (free rolls). To claim, the miner
//! submits the mined pubkey plus an Ed25519 signature (made with the mined
//! secret key) over a program-bound message. The program:
//!   1. verifies the Ed25519 precompile instruction (proves ownership),
//!   2. base58-encodes the pubkey in-program and checks the tier pattern,
//!   3. rejects double-claims via a PDA registry keyed by the mined pubkey,
//!   4. mints SPL rewards with a smooth difficulty multiplier.
//!
//! No admin instructions exist after `initialize`: the program is
//! admin-free. `initialize` also enforces pattern validity — a miner
//! grinding an impossible pattern must never be payable, so impossible
//! patterns are rejected at deploy time, not claim time.

use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    clock::Clock,
    entrypoint,
    entrypoint::ProgramResult,
    instruction::{AccountMeta, Instruction},
    msg,
    program::{invoke, invoke_signed},
    program_error::ProgramError,
    program_pack::Pack,
    pubkey::Pubkey,
    rent::Rent,
    sysvar::{self, Sysvar},
};

// ---------------------------------------------------------------- seeds

const SEED_CONFIG: &[u8] = b"grindmine-cfg";
const SEED_CLAIM: &[u8] = b"grindmine-claim";
const SEED_MINT_AUTH: &[u8] = b"grindmine-mintauth";

const CONFIG_DISCRIMINATOR: [u8; 8] = *b"GRINDM01";
const CLAIM_DISCRIMINATOR: [u8; 8] = *b"GRINDCLM";

/// v2: binds program_id + mint + miner + dest, killing testnet->mainnet
/// claim replay (mint differs per deployment) and claim theft.
const MESSAGE_PREFIX: &[u8] = b"GRINDMINE_CLAIM_v2";

/// Base58 alphabet (Bitcoin/Solana). Excludes 0, O, I, l.
const B58_ALPHABET: &[u8; 58] =
    b"123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz";
//
// NOTE (was B58_FIRST18): a previous version rejected first chars outside
// "123456789ABCDEFGHJ" as "impossible". That was wrong: ~5.8% of pubkeys
// encode to <=43 chars and can lead with ANY base58 char (~0.1% each); the
// 44-char majority leads with 2..=H (~5.8% each, J ~1.3%). No first char is
// impossible, so the validator accepts all base58 chars everywhere.

const MULTIPLIER_SCALE: u64 = 1_000_000;
const MULTIPLIER_MIN: u64 = 100_000; // 0.1x
const MULTIPLIER_MAX: u64 = 10_000_000; // 10x

// ---------------------------------------------------------------- build gates
//
// The initialize authority MUST NOT be the testnet placeholder in a mainnet
// binary: whoever holds that throwaway key would own the launch.
// Build with --features mainnet (requires setting the real authority below)
// or --features testnet (the default).

#[cfg(not(any(feature = "mainnet", feature = "testnet")))]
compile_error!("grindmine-program must be built with --features mainnet or --features testnet");

#[cfg(all(feature = "mainnet", feature = "testnet"))]
compile_error!("features \"mainnet\" and \"testnet\" are mutually exclusive");

/// Throwaway testnet payer bytes. A mainnet build embedding these would hand
/// the launch to whoever holds a leaked testnet key.
const TESTNET_AUTHORITY_PLACEHOLDER: [u8; 32] = [
    239, 208, 248, 158, 161, 85, 36, 245, 115, 35, 216, 90, 182, 141, 16, 17,
    170, 138, 229, 146, 162, 44, 93, 136, 85, 207, 137, 225, 11, 53, 101, 54,
]; // H99FnaQtUqGh7BrN3hxQUKo3MDyrMYvwPA1XxfFHjL21 (throwaway testnet)

#[cfg(feature = "mainnet")]
const fn bytes_is_zero(a: &[u8; 32]) -> bool {
    let mut i = 0;
    let mut acc = 0u8;
    while i < 32 {
        acc |= a[i];
        i += 1;
    }
    acc == 0
}

#[cfg(feature = "mainnet")]
const fn bytes_equal(a: &[u8; 32], b: &[u8; 32]) -> bool {
    let mut i = 0;
    let mut diff = 0u8;
    while i < 32 {
        diff |= a[i] ^ b[i];
        i += 1;
    }
    diff == 0
}

/// The ONLY key allowed to call `initialize`. Without this, anyone can
/// front-run the deploy, initialize first with hostile params, and
/// permanently hijack the launch (red-team C2).
#[cfg(feature = "testnet")]
pub const INITIALIZE_AUTHORITY: Pubkey =
    Pubkey::new_from_array(TESTNET_AUTHORITY_PLACEHOLDER);

/// Mainnet authority. The build FAILS until this is replaced with the real
/// launch initializer's bytes (enforced by the const assertions below).
#[cfg(feature = "mainnet")]
const MAINNET_INITIALIZE_AUTHORITY: [u8; 32] = [0u8; 32];

#[cfg(feature = "mainnet")]
pub const INITIALIZE_AUTHORITY: Pubkey =
    Pubkey::new_from_array(MAINNET_INITIALIZE_AUTHORITY);

#[cfg(feature = "mainnet")]
const _ASSERT_MAINNET_AUTHORITY_SET: () = {
    assert!(
        !bytes_is_zero(&MAINNET_INITIALIZE_AUTHORITY),
        "MAINNET_INITIALIZE_AUTHORITY is unset: replace with the real launch initializer before building mainnet"
    );
    assert!(
        !bytes_equal(&MAINNET_INITIALIZE_AUTHORITY, &TESTNET_AUTHORITY_PLACEHOLDER),
        "MAINNET_INITIALIZE_AUTHORITY must not be the testnet placeholder bytes"
    );
};

// ---------------------------------------------------------------- errors

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum GrindError {
    InvalidPattern = 0,
    PatternTooLong = 1,
    BadTier = 2,
    PatternMismatch = 3,
    BadEd25519Instruction = 4,
    SignaturePubkeyMismatch = 5,
    MessageMismatch = 6,
    AlreadyClaimed = 7,
    BadMintAuthority = 8,
    BadMint = 9,
    BadDestination = 10,
    BadConfigAccount = 11,
    ArithmeticOverflow = 12,
    BadInitializer = 13,
    SupplyExhausted = 14,
    BadInitParams = 15,
    AlreadyInitialized = 16,
    MintPreminted = 17,
    MintFreezeAuthority = 18,
    BadMintDecimals = 19,
    PdaConflict = 20,
}

impl From<GrindError> for ProgramError {
    fn from(e: GrindError) -> Self {
        ProgramError::Custom(e as u32)
    }
}

// ---------------------------------------------------------------- state

#[derive(BorshSerialize, BorshDeserialize, Debug, Clone)]
pub struct Config {
    pub discriminator: [u8; 8],
    pub mint: Pubkey,
    pub mint_auth_bump: u8,
    pub config_bump: u8,
    pub pattern: [u8; 8],
    pub pattern_len: u8,
    pub min_tier: u8,
    /// Base payout (mint base units) per matched-char count, index = matched-1.
    pub payouts: [u64; 8],
    /// Difficulty multiplier, fixed point MULTIPLIER_SCALE.
    pub multiplier: u64,
    pub target_claims_per_window: u64,
    pub window_secs: i64,
    pub window_start: i64,
    pub claims_in_window: u64,
    pub total_claims: u64,
    /// Hard supply cap (mint base units). Enforced in process_claim: minting
    /// stops when minted_total would exceed this. Red-team C1.
    pub max_supply: u64,
    pub minted_total: u64,
}

impl Config {
    pub const LEN: usize = 8 + 32 + 1 + 1 + 8 + 1 + 1 + 64 + 8 + 8 + 8 + 8 + 8 + 8 + 8 + 8;
}

#[derive(BorshSerialize, BorshDeserialize, Debug, Clone)]
pub struct ClaimRecord {
    pub discriminator: [u8; 8],
    pub bump: u8,
}

impl ClaimRecord {
    pub const LEN: usize = 9;
}

// ---------------------------------------------------------------- helpers

/// Validate a tier pattern. Rejects unsatisfiable patterns:
/// - empty or longer than 8 chars
/// - any char outside the base58 alphabet (0, O, I, l can never appear)
///
/// Any base58 char is a legal first char: ~5.8% of pubkeys encode to <=43
/// chars and can lead with anything, so no first char is "impossible".
pub fn validate_pattern(pattern: &[u8]) -> Result<(), GrindError> {
    if pattern.is_empty() || pattern.len() > 8 {
        return Err(GrindError::PatternTooLong);
    }
    if !pattern.iter().all(|c| B58_ALPHABET.contains(c)) {
        return Err(GrindError::InvalidPattern);
    }
    Ok(())
}

/// Validate initialize() params. Pure function, unit-tested. Bad economics
/// set here are permanent — no admin instruction exists to fix them later
/// (red-team H5/M4).
pub fn validate_init_params(
    pattern: &[u8],
    min_tier: u8,
    payouts: &[u64; 8],
    target_claims_per_window: u64,
    window_secs: i64,
    max_supply: u64,
) -> Result<(), GrindError> {
    if pattern.is_empty() || pattern.len() > 8 {
        return Err(GrindError::BadInitParams.into());
    }
    if min_tier == 0 || (min_tier as usize) > pattern.len() {
        return Err(GrindError::BadInitParams.into());
    }
    if target_claims_per_window == 0 || window_secs <= 0 || max_supply == 0 {
        return Err(GrindError::BadInitParams.into());
    }
    // Every payable tier (min_tier..=pattern_len) must have a nonzero payout,
    // and at the 0.1x multiplier floor the effective payout must still be
    // >= 1 base unit: a zero payout reverts the claim, the window never
    // advances, and the tier bricks forever (no admin fix exists).
    for i in (min_tier as usize)..=(pattern.len()) {
        let p = payouts[i - 1] as u128;
        if p == 0 {
            return Err(GrindError::BadInitParams.into());
        }
        if p * (MULTIPLIER_MIN as u128) / (MULTIPLIER_SCALE as u128) < 1 {
            return Err(GrindError::BadInitParams.into());
        }
    }
    // The richest possible single claim must fit inside the cap, or the top
    // tier is unclaimable-from-day-one (and the supply tail starves it first).
    let top = payouts[pattern.len() - 1] as u128;
    if top * (MULTIPLIER_MAX as u128) / (MULTIPLIER_SCALE as u128) > max_supply as u128 {
        return Err(GrindError::BadInitParams.into());
    }
    Ok(())
}

/// Base58-encode 32 bytes (no heap; stack buffer, max 44 chars).
pub fn base58_encode_32(input: &[u8; 32], out: &mut [u8; 44]) -> usize {
    // Count leading zero bytes -> '1's.
    let mut zeros = 0;
    for b in input.iter() {
        if *b == 0 {
            zeros += 1;
        } else {
            break;
        }
    }
    // Convert base256 to base58 into a temp little-endian digit vec.
    let mut digits = [0u8; 44];
    let mut digit_len = 0usize;
    for b in input.iter() {
        let mut carry = *b as u32;
        let mut i = 0;
        while i < digit_len || carry != 0 {
            let v = (if i < digit_len { digits[i] as u32 } else { 0 }) * 256 + carry;
            if i < digit_len {
                digits[i] = (v % 58) as u8;
            } else {
                digits[digit_len] = (v % 58) as u8;
                digit_len += 1;
            }
            carry = v / 58;
            i += 1;
        }
    }
    let mut n = 0;
    for _ in 0..zeros {
        out[n] = b'1';
        n += 1;
    }
    for i in (0..digit_len).rev() {
        out[n] = B58_ALPHABET[digits[i] as usize];
        n += 1;
    }
    n
}

fn read_u16_le(data: &[u8], at: usize) -> Result<u16, GrindError> {
    data.get(at..at + 2)
        .ok_or(GrindError::BadEd25519Instruction)
        .map(|s| u16::from_le_bytes([s[0], s[1]]))
}

/// Verify the Ed25519 precompile instruction at `index` in the transaction:
/// it must be a single-signature verify whose pubkey == `expected_pubkey`
/// and whose message == `expected_message`. (The precompile itself already
/// enforced cryptographic validity — the tx could not have executed
/// otherwise. We bind its fields to this claim.)
pub fn verify_ed25519_instruction(
    instructions_sysvar: &AccountInfo,
    index: usize,
    expected_pubkey: &[u8; 32],
    expected_message: &[u8],
) -> Result<(), GrindError> {
    let ix = solana_instructions_sysvar::load_instruction_at_checked(index, instructions_sysvar)
        .map_err(|_| GrindError::BadEd25519Instruction)?;
    if ix.program_id != solana_program::ed25519_program::id() {
        return Err(GrindError::BadEd25519Instruction);
    }
    let d = &ix.data;
    if d.len() < 16 {
        return Err(GrindError::BadEd25519Instruction);
    }
    if d[0] != 1 {
        return Err(GrindError::BadEd25519Instruction);
    }
    let sig_offset = read_u16_le(d, 2)? as usize;
    let sig_ix_index = read_u16_le(d, 4)?;
    let pubkey_offset = read_u16_le(d, 6)? as usize;
    let pubkey_ix_index = read_u16_le(d, 8)?;
    let msg_offset = read_u16_le(d, 10)? as usize;
    let msg_size = read_u16_le(d, 12)? as usize;
    let msg_ix_index = read_u16_le(d, 14)?;
    // All three must reference the ed25519 instruction's own data.
    if sig_ix_index != u16::MAX || pubkey_ix_index != u16::MAX || msg_ix_index != u16::MAX {
        return Err(GrindError::BadEd25519Instruction);
    }
    let sig_end = sig_offset.checked_add(64).ok_or(GrindError::BadEd25519Instruction)?;
    let pk_end = pubkey_offset.checked_add(32).ok_or(GrindError::BadEd25519Instruction)?;
    let msg_end = msg_offset.checked_add(msg_size).ok_or(GrindError::BadEd25519Instruction)?;
    if d.len() < sig_end.max(pk_end).max(msg_end) {
        return Err(GrindError::BadEd25519Instruction);
    }
    if &d[pubkey_offset..pk_end] != expected_pubkey {
        return Err(GrindError::SignaturePubkeyMismatch);
    }
    if &d[msg_offset..msg_end] != expected_message {
        return Err(GrindError::MessageMismatch);
    }
    Ok(())
}

/// The message a miner signs. Binds EVERYTHING an attacker could swap:
/// program, mint (kills testnet->mainnet replay: mint differs per
/// deployment), the fee-paying miner, the reward destination, the mined key,
/// and the tier. A stolen signature cannot be replayed for a different
/// recipient, payer, or deployment.
///
/// Byte layout (179 bytes):
///   0..18    MESSAGE_PREFIX ("GRINDMINE_CLAIM_v2")
///   18..50   program_id
///   50..82   config.mint
///   82..114  miner (fee payer) pubkey
///   114..146 dest token account pubkey
///   146..178 mined pubkey
///   178      matched tier
fn expected_message(
    program_id: &Pubkey,
    mint: &Pubkey,
    miner: &Pubkey,
    dest: &Pubkey,
    mined: &[u8; 32],
    matched: u8,
) -> Vec<u8> {
    let mut m = Vec::with_capacity(MESSAGE_PREFIX.len() + 32 * 5 + 1);
    m.extend_from_slice(MESSAGE_PREFIX);
    m.extend_from_slice(program_id.as_ref());
    m.extend_from_slice(mint.as_ref());
    m.extend_from_slice(miner.as_ref());
    m.extend_from_slice(dest.as_ref());
    m.extend_from_slice(mined);
    m.push(matched);
    m
}

/// Create a PDA-owned account, tolerating a pre-funded (griefed) target.
///
/// `system_instruction::create_account` fails if the target already holds
/// lamports, so anyone can brick `initialize` — or one specific claim — for
/// the price of a dust transfer: the config/claim PDA addresses are
/// derivable by anyone, and deploy-then-initialize cannot be atomic.
///
/// If the target is pre-funded we take it over instead of failing: top it up
/// to rent-exemption with a plain transfer, then `allocate` + `assign` via
/// the PDA seeds. This only proceeds for a plain system-owned account with
/// empty (dust-grief) or exactly-sized data; anything else fails safe with
/// `PdaConflict` rather than corrupting state. Callers must still guard
/// re-initialization / double-claim before calling.
fn create_pda_account<'a>(
    payer: &AccountInfo<'a>,
    target: &AccountInfo<'a>,
    system_program: &AccountInfo<'a>,
    program_id: &Pubkey,
    space: u64,
    seeds: &[&[u8]],
) -> ProgramResult {
    let need = Rent::get()?.minimum_balance(space as usize);
    if target.lamports() == 0 {
        // Fresh account: the normal path.
        return invoke_signed(
            &solana_program::system_instruction::create_account(
                payer.key,
                target.key,
                need,
                space,
                program_id,
            ),
            &[payer.clone(), target.clone(), system_program.clone()],
            &[seeds],
        );
    }
    // Pre-funded: only a plain system-owned account can be taken over.
    if *target.owner != solana_program::system_program::id() {
        return Err(GrindError::PdaConflict.into());
    }
    let data_len = target.data_len();
    if data_len != 0 && data_len != space as usize {
        return Err(GrindError::PdaConflict.into());
    }
    let top_up = need.saturating_sub(target.lamports());
    if top_up > 0 {
        invoke(
            &solana_program::system_instruction::transfer(payer.key, target.key, top_up),
            &[payer.clone(), target.clone()],
        )?;
    }
    if data_len == 0 {
        invoke_signed(
            &solana_program::system_instruction::allocate(target.key, space),
            &[target.clone(), system_program.clone()],
            &[seeds],
        )?;
    }
    invoke_signed(
        &solana_program::system_instruction::assign(target.key, program_id),
        &[target.clone(), system_program.clone()],
        &[seeds],
    )?;
    Ok(())
}

/// Pure difficulty-adjustment step, unit-tested. Returns (new_multiplier,
/// new_window_start). No-op when the window has not elapsed.
///
/// - Normalizes by elapsed windows, so a long lull counts as many windows
///   instead of one (a single quiet window must not slam 1x -> 10x).
/// - Bounds the per-adjustment step to [0.5x, 2x] of the previous
///   multiplier. Hits never expire, so an unbounded jump lets miners hoard
///   through a lull and dump at the ceiling.
/// - Absolute clamp [MULTIPLIER_MIN, MULTIPLIER_MAX].
pub fn adjust_multiplier(
    multiplier: u64,
    target_claims_per_window: u64,
    claims_in_window: u64,
    window_secs: i64,
    window_start: i64,
    now: i64,
) -> (u64, i64) {
    if now - window_start < window_secs {
        return (multiplier, window_start);
    }
    let elapsed = (now - window_start).max(0) as u64;
    let elapsed_windows = (elapsed / window_secs.max(1) as u64).max(1);
    let per_window = ((claims_in_window as u128) / (elapsed_windows as u128)).max(1);
    let raw = (multiplier as u128).saturating_mul(target_claims_per_window as u128) / per_window;
    let lo = (multiplier as u128) / 2;
    let hi = (multiplier as u128).saturating_mul(2);
    let stepped = raw.clamp(lo, hi);
    ((stepped as u64).clamp(MULTIPLIER_MIN, MULTIPLIER_MAX), now)
}

// ---------------------------------------------------------------- instructions

fn process_initialize(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    pattern: Vec<u8>,
    min_tier: u8,
    payouts: [u64; 8],
    target_claims_per_window: u64,
    window_secs: i64,
    max_supply: u64,
) -> ProgramResult {
    let iter = &mut accounts.iter();
    let payer = next_account_info(iter)?;
    let config_ai = next_account_info(iter)?;
    let mint_ai = next_account_info(iter)?;
    let system_program = next_account_info(iter)?;

    // C2: only the hardcoded launch deployer may initialize. Anyone else
    // front-running this instruction would permanently hijack the launch.
    if !payer.is_signer || *payer.key != INITIALIZE_AUTHORITY {
        return Err(GrindError::BadInitializer.into());
    }

    validate_pattern(&pattern)?;
    if min_tier == 0 || min_tier as usize > pattern.len() {
        return Err(GrindError::BadTier.into());
    }
    validate_init_params(&pattern, min_tier, &payouts, target_claims_per_window, window_secs, max_supply)?;

    let (config_pda, config_bump) = Pubkey::find_program_address(&[SEED_CONFIG], program_id);
    if config_pda != *config_ai.key {
        return Err(GrindError::BadConfigAccount.into());
    }
    // Re-initialization is forbidden. The only way this PDA is owned by us
    // is a completed prior initialize (failed txs roll back atomically), so
    // there is no safe "resume" case — and the authority must not get a
    // second chance to rewrite the economics.
    if *config_ai.owner == *program_id {
        return Err(GrindError::AlreadyInitialized.into());
    }
    let (mint_auth_pda, mint_auth_bump) = Pubkey::find_program_address(&[SEED_MINT_AUTH], program_id);

    // The mint must exist and name our PDA as its authority.
    if *mint_ai.owner != spl_token::id() {
        return Err(GrindError::BadMint.into());
    }
    let mint_data = mint_ai.try_borrow_data()?;
    let mint = spl_token::state::Mint::unpack(&mint_data).map_err(|_| GrindError::BadMint)?;
    drop(mint_data);
    match mint.mint_authority {
        solana_program::program_option::COption::Some(a) if a == mint_auth_pda => {}
        _ => return Err(GrindError::BadMintAuthority.into()),
    }
    // The mint must be pristine: a premint is a hidden team allocation, a
    // freeze authority is holder-freeze power, and decimals are what the
    // manifest prices against. Without these the "fair launch" is docs-only.
    if !mint.is_initialized {
        return Err(GrindError::BadMint.into());
    }
    if mint.supply != 0 {
        msg!("mint has pre-existing supply: {}", mint.supply);
        return Err(GrindError::MintPreminted.into());
    }
    if mint.freeze_authority.is_some() {
        return Err(GrindError::MintFreezeAuthority.into());
    }
    if mint.decimals != 6 {
        return Err(GrindError::BadMintDecimals.into());
    }

    // Create the config PDA (rent-exempt). Tolerates a griefed pre-funded
    // PDA instead of bricking the launch (see create_pda_account).
    create_pda_account(
        payer,
        config_ai,
        system_program,
        program_id,
        Config::LEN as u64,
        &[SEED_CONFIG, &[config_bump]],
    )?;

    let mut pat = [0u8; 8];
    pat[..pattern.len()].copy_from_slice(&pattern);
    let clock = Clock::get()?;
    let cfg = Config {
        discriminator: CONFIG_DISCRIMINATOR,
        mint: *mint_ai.key,
        mint_auth_bump,
        config_bump,
        pattern: pat,
        pattern_len: pattern.len() as u8,
        min_tier,
        payouts,
        multiplier: MULTIPLIER_SCALE,
        target_claims_per_window,
        window_secs,
        window_start: clock.unix_timestamp,
        claims_in_window: 0,
        total_claims: 0,
        max_supply,
        minted_total: 0,
    };
    cfg.serialize(&mut &mut config_ai.try_borrow_mut_data()?[..])?;
    msg!("grindmine initialized: pattern={} min_tier={}",
        String::from_utf8_lossy(&pattern), min_tier);
    Ok(())
}

fn process_claim(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    mined_pubkey: [u8; 32],
    matched: u8,
) -> ProgramResult {
    let iter = &mut accounts.iter();
    let miner = next_account_info(iter)?; // signer, fee payer, reward dest owner
    let config_ai = next_account_info(iter)?;
    let claim_record_ai = next_account_info(iter)?;
    let mint_ai = next_account_info(iter)?;
    let dest_ai = next_account_info(iter)?;
    let _mint_auth_ai = next_account_info(iter)?;
    let instructions_sysvar = next_account_info(iter)?;
    let token_program = next_account_info(iter)?;
    let system_program = next_account_info(iter)?;

    if !miner.is_signer {
        return Err(ProgramError::MissingRequiredSignature);
    }

    // Load config.
    let config_data = config_ai.try_borrow_data()?;
    let mut config = Config::deserialize(&mut &config_data[..])
        .map_err(|_| GrindError::BadConfigAccount)?;
    drop(config_data);
    if config.discriminator != CONFIG_DISCRIMINATOR {
        return Err(GrindError::BadConfigAccount.into());
    }
    let (config_pda, _) = Pubkey::find_program_address(&[SEED_CONFIG], program_id);
    if config_pda != *config_ai.key || config.mint != *mint_ai.key {
        return Err(GrindError::BadConfigAccount.into());
    }
    if *token_program.key != spl_token::id() {
        return Err(GrindError::BadMint.into());
    }

    // Tier bounds.
    if matched < config.min_tier || matched > config.pattern_len {
        return Err(GrindError::BadTier.into());
    }

    // 1. Ownership: ed25519 precompile instruction must be ix #0 and bind
    //    (program, mint, miner, dest, pubkey, tier) to this claim. The
    //    signature is worthless for any other recipient, payer, or
    //    deployment — observers cannot steal it.
    let message = expected_message(
        program_id,
        &config.mint,
        miner.key,
        dest_ai.key,
        &mined_pubkey,
        matched,
    );
    verify_ed25519_instruction(instructions_sysvar, 0, &mined_pubkey, &message)?;

    // 2. Pattern: base58(pubkey) must start with pattern[..matched].
    let mut enc = [0u8; 44];
    let enc_len = base58_encode_32(&mined_pubkey, &mut enc);
    if (matched as usize) > enc_len {
        return Err(GrindError::PatternMismatch.into());
    }
    if &enc[..matched as usize] != &config.pattern[..matched as usize] {
        return Err(GrindError::PatternMismatch.into());
    }

    // 3. Double-claim: the registry PDA must not exist yet.
    let (claim_pda, claim_bump) =
        Pubkey::find_program_address(&[SEED_CLAIM, &mined_pubkey], program_id);
    if claim_pda != *claim_record_ai.key {
        return Err(GrindError::BadConfigAccount.into());
    }
    // Owned by us <=> a completed prior claim (failed txs roll back, so a
    // half-written record is impossible). A griefed pre-funded PDA is
    // system-owned and falls through to create_pda_account, which takes it
    // over when safe or fails with PdaConflict.
    if *claim_record_ai.owner == *program_id {
        return Err(GrindError::AlreadyClaimed.into());
    }

    // 4. Destination must be a token account for our mint owned by the miner.
    let dest_data = dest_ai.try_borrow_data()?;
    let dest = spl_token::state::Account::unpack(&dest_data).map_err(|_| GrindError::BadDestination)?;
    drop(dest_data);
    if dest.mint != config.mint || dest.owner != *miner.key {
        return Err(GrindError::BadDestination.into());
    }

    // 5. Difficulty: smooth adjustment from trailing participation.
    // Bounded per step and normalized by elapsed windows (see
    // adjust_multiplier): a quiet window can no longer slam 1x -> 10x for
    // hoard-and-dump miners.
    let clock = Clock::get()?;
    if clock.unix_timestamp - config.window_start >= config.window_secs {
        let (new_mult, new_start) = adjust_multiplier(
            config.multiplier,
            config.target_claims_per_window,
            config.claims_in_window,
            config.window_secs,
            config.window_start,
            clock.unix_timestamp,
        );
        if new_mult != config.multiplier {
            msg!("difficulty adjusted: multiplier={}", new_mult);
        }
        config.multiplier = new_mult;
        config.window_start = new_start;
        config.claims_in_window = 0;
    }

    // 6. Payout and mint.
    let base = config.payouts[(matched - 1) as usize];
    let payout = (base as u128)
        .saturating_mul(config.multiplier as u128)
        / (MULTIPLIER_SCALE as u128);
    let payout = u64::try_from(payout).map_err(|_| GrindError::ArithmeticOverflow)?;
    if payout == 0 {
        return Err(GrindError::ArithmeticOverflow.into());
    }

    // C1: hard supply cap. Minting stops when the cap is reached, no matter
    // what the multiplier says. Without this the "fixed supply" claim is
    // docs-only and every valid claim mints forever.
    //
    // Tail behavior: the final claim mints whatever is left, closing the cap
    // exactly instead of failing top-tier claims while low-tier dust still
    // fits (which would strand a remainder and publish stealable
    // signatures on every failed attempt).
    let remaining = config.max_supply.saturating_sub(config.minted_total);
    if remaining == 0 {
        return Err(GrindError::SupplyExhausted.into());
    }
    let payout = payout.min(remaining);
    config.minted_total = config.minted_total.saturating_add(payout);

    let mint_auth_seeds: &[&[u8]] = &[SEED_MINT_AUTH, &[config.mint_auth_bump]];
    invoke_signed(
        &spl_token::instruction::mint_to(
            token_program.key,
            mint_ai.key,
            dest_ai.key,
            &Pubkey::find_program_address(&[SEED_MINT_AUTH], program_id).0,
            &[],
            payout,
        )?,
        &[
            mint_ai.clone(),
            dest_ai.clone(),
            _mint_auth_ai.clone(),
            token_program.clone(),
        ],
        &[mint_auth_seeds],
    )?;

    // 7. Mark claimed (registry write) + counters. Tolerates a griefed
    // pre-funded claim PDA (see create_pda_account); a genuinely
    // already-claimed hit was rejected in step 3 above.
    create_pda_account(
        miner,
        claim_record_ai,
        system_program,
        program_id,
        ClaimRecord::LEN as u64,
        &[SEED_CLAIM, &mined_pubkey, &[claim_bump]],
    )?;
    let rec = ClaimRecord { discriminator: CLAIM_DISCRIMINATOR, bump: claim_bump };
    rec.serialize(&mut &mut claim_record_ai.try_borrow_mut_data()?[..])?;

    config.claims_in_window = config.claims_in_window.saturating_add(1);
    config.total_claims = config.total_claims.saturating_add(1);
    config.serialize(&mut &mut config_ai.try_borrow_mut_data()?[..])?;

    msg!("claim ok: matched={} payout={}", matched, payout);
    Ok(())
}

// ---------------------------------------------------------------- entrypoint

entrypoint!(process_instruction);

pub fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    input: &[u8],
) -> ProgramResult {
    let (tag, rest) = input.split_first().ok_or(ProgramError::InvalidInstructionData)?;
    match tag {
        // initialize | pattern_len:u8 | pattern[..len] | min_tier:u8 |
        //             payouts:[u64;8] | target:u64 | window_secs:i64 | max_supply:u64
        0 => {
            if rest.len() < 2 {
                return Err(ProgramError::InvalidInstructionData);
            }
            let plen = rest[0] as usize;
            if rest.len() < 1 + plen + 1 + 64 + 8 + 8 + 8 {
                return Err(ProgramError::InvalidInstructionData);
            }
            let pattern = rest[1..1 + plen].to_vec();
            let min_tier = rest[1 + plen];
            let mut payouts = [0u64; 8];
            let mut off = 1 + plen + 1;
            for p in payouts.iter_mut() {
                *p = u64::from_le_bytes(rest[off..off + 8].try_into().unwrap());
                off += 8;
            }
            let target = u64::from_le_bytes(rest[off..off + 8].try_into().unwrap());
            off += 8;
            let window_secs = i64::from_le_bytes(rest[off..off + 8].try_into().unwrap());
            off += 8;
            let max_supply = u64::from_le_bytes(rest[off..off + 8].try_into().unwrap());
            process_initialize(program_id, accounts, pattern, min_tier, payouts, target, window_secs, max_supply)
        }
        // claim | mined_pubkey:[u8;32] | matched:u8
        1 => {
            if rest.len() != 33 {
                return Err(ProgramError::InvalidInstructionData);
            }
            let mut mined = [0u8; 32];
            mined.copy_from_slice(&rest[..32]);
            let matched = rest[32];
            process_claim(program_id, accounts, mined, matched)
        }
        _ => Err(ProgramError::InvalidInstructionData),
    }
}

/// Build an `initialize` instruction (used by the client).
pub fn build_initialize_ix(
    program_id: &Pubkey,
    payer: &Pubkey,
    mint: &Pubkey,
    pattern: &[u8],
    min_tier: u8,
    payouts: [u64; 8],
    target_claims_per_window: u64,
    window_secs: i64,
    max_supply: u64,
) -> Instruction {
    let (config_pda, _) = Pubkey::find_program_address(&[SEED_CONFIG], program_id);
    let mut data = vec![0u8, pattern.len() as u8];
    data.extend_from_slice(pattern);
    data.push(min_tier);
    for p in payouts {
        data.extend_from_slice(&p.to_le_bytes());
    }
    data.extend_from_slice(&target_claims_per_window.to_le_bytes());
    data.extend_from_slice(&window_secs.to_le_bytes());
    data.extend_from_slice(&max_supply.to_le_bytes());
    Instruction {
        program_id: *program_id,
        accounts: vec![
            AccountMeta::new(*payer, true),
            AccountMeta::new(config_pda, false),
            AccountMeta::new_readonly(*mint, false),
            AccountMeta::new_readonly(solana_program::system_program::id(), false),
        ],
        data,
    }
}

/// Build a `claim` instruction (used by the client). Emits ALL accounts in
/// the exact order `process_claim` expects — no placeholder positions for
/// the client to splice (a misordered claim fails after the precompile
/// passes, leaking the signature).
///
/// Account order:
///   0. miner (signer, writable)
///   1. config PDA (writable)
///   2. claim record PDA (writable)
///   3. mint (writable)
///   4. dest token account (writable)
///   5. mint authority PDA (readonly)
///   6. instructions sysvar (readonly)
///   7. SPL token program (readonly)
///   8. system program (readonly)
pub fn build_claim_ix(
    program_id: &Pubkey,
    miner: &Pubkey,
    mint: &Pubkey,
    dest_token_account: &Pubkey,
    mined_pubkey: [u8; 32],
    matched: u8,
) -> Instruction {
    let (config_pda, _) = Pubkey::find_program_address(&[SEED_CONFIG], program_id);
    let mined = Pubkey::new_from_array(mined_pubkey);
    let (claim_pda, _) = Pubkey::find_program_address(&[SEED_CLAIM, mined.as_ref()], program_id);
    let (mint_auth_pda, _) = Pubkey::find_program_address(&[SEED_MINT_AUTH], program_id);
    let mut data = vec![1u8];
    data.extend_from_slice(&mined_pubkey);
    data.push(matched);
    Instruction {
        program_id: *program_id,
        accounts: vec![
            AccountMeta::new(*miner, true),
            AccountMeta::new(config_pda, false),
            AccountMeta::new(claim_pda, false),
            AccountMeta::new(*mint, false),
            AccountMeta::new(*dest_token_account, false),
            AccountMeta::new_readonly(mint_auth_pda, false),
            AccountMeta::new_readonly(sysvar::instructions::id(), false),
            AccountMeta::new_readonly(spl_token::id(), false),
            AccountMeta::new_readonly(solana_program::system_program::id(), false),
        ],
        data,
    }
}

/// The canonical message a miner signs: prefix || program_id || mint ||
/// miner || dest_token_account || mined_pubkey || matched.
///
/// The miner must sign AFTER choosing payer and destination; the client flow
/// must collect those first. See `expected_message` for the byte layout.
pub fn claim_message(
    program_id: &Pubkey,
    mint: &Pubkey,
    miner: &Pubkey,
    dest_token_account: &Pubkey,
    mined_pubkey: &[u8; 32],
    matched: u8,
) -> Vec<u8> {
    expected_message(program_id, mint, miner, dest_token_account, mined_pubkey, matched)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_patterns_accepted() {
        assert!(validate_pattern(b"Gr1nd").is_ok());
        assert!(validate_pattern(b"G").is_ok());
        assert!(validate_pattern(b"12345678").is_ok());
    }

    #[test]
    fn rare_first_chars_accepted() {
        // No first char is impossible: ~5.8% of pubkeys encode to <=43 chars
        // and can lead with ANY base58 char (~0.1% each). Zebra/grind/zzzzz
        // are rare, not unsatisfiable — the old B58_FIRST18 model was wrong.
        assert!(validate_pattern(b"Zebra").is_ok());
        assert!(validate_pattern(b"grind").is_ok());
        assert!(validate_pattern(b"zzzzz").is_ok());
        assert!(validate_pattern(b"1abc").is_ok());
    }

    #[test]
    fn non_base58_chars_rejected() {
        // 0, O, I, l are not in the base58 alphabet at all.
        assert_eq!(validate_pattern(b"G0ld"), Err(GrindError::InvalidPattern));
        assert_eq!(validate_pattern(b"GOld"), Err(GrindError::InvalidPattern));
        assert_eq!(validate_pattern(b"GIld"), Err(GrindError::InvalidPattern));
        assert_eq!(validate_pattern(b"Glld"), Err(GrindError::InvalidPattern));
    }

    #[test]
    fn base58_encode_matches_known_vector() {
        // System program address: 11111111111111111111111111111111
        let mut enc = [0u8; 44];
        let n = base58_encode_32(&[0u8; 32], &mut enc);
        assert_eq!(&enc[..n], b"11111111111111111111111111111111");
    }

    #[test]
    fn base58_encode_roundtrip_random() {
        // Cross-check against a second implementation path (u128 chunks are
        // overkill; instead verify decode(encode(x)) == x via manual decode).
        let input = [0xABu8; 32];
        let mut enc = [0u8; 44];
        let n = base58_encode_32(&input, &mut enc);
        // decode back
        let mut num: [u32; 9] = [0; 9]; // 288 bits
        for c in &enc[..n] {
            let v = B58_ALPHABET.iter().position(|b| b == c).unwrap() as u32;
            let mut carry = v;
            for w in num.iter_mut() {
                let t = (*w as u64) * 58 + carry as u64;
                *w = (t & 0xFFFF_FFFF) as u32;
                carry = (t >> 32) as u32;
            }
        }
        let mut back = [0u8; 32];
        for (i, w) in num.iter().take(8).enumerate() {
            back[i * 4..i * 4 + 4].copy_from_slice(&w.to_le_bytes());
        }
        // num is little-endian u32 words; input was big-endian bytes
        back.reverse();
        assert_eq!(back, input);
    }

    #[test]
    fn initialize_authority_is_set() {
        // The launch build must not ship with a placeholder authority:
        // anyone could call initialize() otherwise (red-team C2).
        assert_ne!(INITIALIZE_AUTHORITY, Pubkey::default());
    }

    #[test]
    fn init_params_reject_degenerate() {
        let payouts = [0, 0, 0, 1_000u64, 58_000, 0, 0, 0];
        assert!(validate_init_params(b"Gr1nd", 4, &payouts, 100, 86400, 1_000_000).is_ok());
        // zero target / window / supply
        assert_eq!(
            validate_init_params(b"Gr1nd", 4, &payouts, 0, 86400, 1_000_000),
            Err(GrindError::BadInitParams)
        );
        assert_eq!(
            validate_init_params(b"Gr1nd", 4, &payouts, 100, 0, 1_000_000),
            Err(GrindError::BadInitParams)
        );
        assert_eq!(
            validate_init_params(b"Gr1nd", 4, &payouts, 100, -5, 1_000_000),
            Err(GrindError::BadInitParams)
        );
        assert_eq!(
            validate_init_params(b"Gr1nd", 4, &payouts, 100, 86400, 0),
            Err(GrindError::BadInitParams)
        );
        // zero payout on a payable tier (4-char tier here)
        let zero_payout = [0u64; 8];
        assert_eq!(
            validate_init_params(b"Gr1nd", 4, &zero_payout, 100, 86400, 1_000_000),
            Err(GrindError::BadInitParams)
        );
    }

    #[test]
    fn init_params_reject_dust_tier_payout() {
        // At the 0.1x multiplier floor a base payout < 10 yields payout 0:
        // the claim reverts, the window never advances, the tier bricks.
        let dust = [0, 0, 0, 5u64, 58_000, 0, 0, 0];
        assert_eq!(
            validate_init_params(b"Gr1nd", 4, &dust, 100, 86400, 1_000_000),
            Err(GrindError::BadInitParams)
        );
        // Boundary: payout 10 * 0.1x = 1 base unit, the minimum viable.
        let ok = [0, 0, 0, 10u64, 58_000, 0, 0, 0];
        assert!(validate_init_params(b"Gr1nd", 4, &ok, 100, 86400, 1_000_000).is_ok());
    }

    #[test]
    fn init_params_reject_top_tier_over_cap() {
        // Richest possible single claim (10x ceiling) must fit the cap.
        let fat = [0, 0, 0, 1_000u64, 200_000, 0, 0, 0];
        assert_eq!(
            validate_init_params(b"Gr1nd", 4, &fat, 100, 86400, 1_000_000),
            Err(GrindError::BadInitParams)
        );
    }

    #[test]
    fn claim_message_layout() {
        let program_id = Pubkey::new_from_array([1u8; 32]);
        let mint = Pubkey::new_from_array([2u8; 32]);
        let miner = Pubkey::new_from_array([3u8; 32]);
        let dest = Pubkey::new_from_array([4u8; 32]);
        let mined = [5u8; 32];
        let m = claim_message(&program_id, &mint, &miner, &dest, &mined, 4);
        assert_eq!(m.len(), 179);
        assert_eq!(&m[0..18], b"GRINDMINE_CLAIM_v2");
        assert_eq!(&m[18..50], &[1u8; 32]);
        assert_eq!(&m[50..82], &[2u8; 32]);
        assert_eq!(&m[82..114], &[3u8; 32]);
        assert_eq!(&m[114..146], &[4u8; 32]);
        assert_eq!(&m[146..178], &[5u8; 32]);
        assert_eq!(m[178], 4);
        // Any bound field changing changes the message (no replay swaps).
        let m2 = claim_message(&program_id, &mint, &miner, &Pubkey::new_from_array([9u8; 32]), &mined, 4);
        assert_ne!(m, m2);
    }

    #[test]
    fn multiplier_step_bounded() {
        // Quiet window: raw math wants 1x -> 100x, step bound caps at 2x.
        let (m, _) = adjust_multiplier(1_000_000, 100, 0, 86_400, 0, 86_400);
        assert_eq!(m, 2_000_000);
        // Busy window: raw math wants 1x -> 0.1x, step bound floors at 0.5x.
        let (m, _) = adjust_multiplier(1_000_000, 100, 1_000, 86_400, 0, 86_400);
        assert_eq!(m, 500_000);
        // Long lull counts as many windows, not one: 0 claims over 5 windows
        // still only steps 2x per adjustment (not a single 100x slam).
        let (m, _) = adjust_multiplier(1_000_000, 100, 0, 86_400, 0, 5 * 86_400);
        assert_eq!(m, 2_000_000);
        // On-target: no change.
        let (m, _) = adjust_multiplier(1_000_000, 100, 100, 86_400, 0, 86_400);
        assert_eq!(m, 1_000_000);
        // Window not elapsed: no-op.
        let (m, s) = adjust_multiplier(1_000_000, 100, 0, 86_400, 1_000, 2_000);
        assert_eq!((m, s), (1_000_000, 1_000));
        // Absolute ceiling still holds.
        let (m, _) = adjust_multiplier(9_000_000, 100, 0, 86_400, 0, 86_400);
        assert_eq!(m, 10_000_000);
    }

    #[test]
    fn config_len_matches_fields() {
        // max_supply + minted_total added for the C1 supply cap.
        assert_eq!(Config::LEN, 180);
    }
}
