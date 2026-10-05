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
    program::{invoke_signed},
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

const MESSAGE_PREFIX: &[u8] = b"GRINDMINE_CLAIM_v1";

/// Base58 alphabet (Bitcoin/Solana). Excludes 0, O, I, l.
const B58_ALPHABET: &[u8; 58] =
    b"123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz";
/// First base58 char of a 32-byte address is range-limited: 32 bytes cannot
/// fill the full base58 space (58^44 > 2^256 > 58^43), so the leading digit
/// is capped at index 17.
const B58_FIRST18: &[u8; 18] = b"123456789ABCDEFGHJ";

const MULTIPLIER_SCALE: u64 = 1_000_000;
const MULTIPLIER_MIN: u64 = 100_000; // 0.1x
const MULTIPLIER_MAX: u64 = 10_000_000; // 10x

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
}

impl Config {
    pub const LEN: usize = 8 + 32 + 1 + 1 + 8 + 1 + 1 + 64 + 8 + 8 + 8 + 8 + 8 + 8;
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

/// Validate a tier pattern. Rejects impossible patterns:
/// - empty or longer than 8 chars
/// - any char outside the base58 alphabet (0, O, I, l can never appear)
/// - first char outside the 18 symbols a 32-byte address can lead with
pub fn validate_pattern(pattern: &[u8]) -> Result<(), GrindError> {
    if pattern.is_empty() || pattern.len() > 8 {
        return Err(GrindError::PatternTooLong);
    }
    if !B58_FIRST18.contains(&pattern[0]) {
        return Err(GrindError::InvalidPattern);
    }
    if !pattern.iter().all(|c| B58_ALPHABET.contains(c)) {
        return Err(GrindError::InvalidPattern);
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

fn expected_message(program_id: &Pubkey, mined: &[u8; 32], matched: u8) -> Vec<u8> {
    let mut m = Vec::with_capacity(MESSAGE_PREFIX.len() + 32 + 32 + 1);
    m.extend_from_slice(MESSAGE_PREFIX);
    m.extend_from_slice(program_id.as_ref());
    m.extend_from_slice(mined);
    m.push(matched);
    Ok::<Vec<u8>, GrindError>(m).unwrap_or_default()
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
) -> ProgramResult {
    let iter = &mut accounts.iter();
    let payer = next_account_info(iter)?;
    let config_ai = next_account_info(iter)?;
    let mint_ai = next_account_info(iter)?;
    let system_program = next_account_info(iter)?;

    validate_pattern(&pattern)?;
    if min_tier == 0 || min_tier as usize > pattern.len() {
        return Err(GrindError::BadTier.into());
    }

    let (config_pda, config_bump) = Pubkey::find_program_address(&[SEED_CONFIG], program_id);
    if config_pda != *config_ai.key {
        return Err(GrindError::BadConfigAccount.into());
    }
    let (mint_auth_pda, mint_auth_bump) = Pubkey::find_program_address(&[SEED_MINT_AUTH], program_id);

    // The mint must exist and name our PDA as its authority.
    let mint_data = mint_ai.try_borrow_data()?;
    let mint = spl_token::state::Mint::unpack(&mint_data).map_err(|_| GrindError::BadMint)?;
    drop(mint_data);
    match mint.mint_authority {
        solana_program::program_option::COption::Some(a) if a == mint_auth_pda => {}
        _ => return Err(GrindError::BadMintAuthority.into()),
    }

    // Create the config account (rent-exempt).
    let rent = Rent::get()?;
    let lamports = rent.minimum_balance(Config::LEN);
    invoke_signed(
        &solana_program::system_instruction::create_account(
            payer.key,
            config_ai.key,
            lamports,
            Config::LEN as u64,
            program_id,
        ),
        &[payer.clone(), config_ai.clone(), system_program.clone()],
        &[&[SEED_CONFIG, &[config_bump]]],
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
    //    (pubkey, message) to this claim.
    let message = expected_message(program_id, &mined_pubkey, matched);
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
    if !claim_record_ai.data_is_empty() {
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
    let clock = Clock::get()?;
    if clock.unix_timestamp - config.window_start >= config.window_secs {
        let actual = config.claims_in_window.max(1);
        let new_mult = (config.multiplier as u128)
            .saturating_mul(config.target_claims_per_window as u128)
            / (actual as u128);
        config.multiplier = (new_mult as u64).clamp(MULTIPLIER_MIN, MULTIPLIER_MAX);
        config.window_start = clock.unix_timestamp;
        config.claims_in_window = 0;
        msg!("difficulty adjusted: multiplier={}", config.multiplier);
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

    // 7. Mark claimed (registry write) + counters.
    let rent = Rent::get()?;
    let lamports = rent.minimum_balance(ClaimRecord::LEN);
    invoke_signed(
        &solana_program::system_instruction::create_account(
            miner.key,
            claim_record_ai.key,
            lamports,
            ClaimRecord::LEN as u64,
            program_id,
        ),
        &[miner.clone(), claim_record_ai.clone(), system_program.clone()],
        &[&[SEED_CLAIM, &mined_pubkey, &[claim_bump]]],
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
        //             payouts:[u64;8] | target:u64 | window_secs:i64
        0 => {
            if rest.len() < 2 {
                return Err(ProgramError::InvalidInstructionData);
            }
            let plen = rest[0] as usize;
            if rest.len() < 1 + plen + 1 + 64 + 8 + 8 {
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
            process_initialize(program_id, accounts, pattern, min_tier, payouts, target, window_secs)
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

/// Build a `claim` instruction (used by the client).
pub fn build_claim_ix(
    program_id: &Pubkey,
    miner: &Pubkey,
    mined_pubkey: [u8; 32],
    matched: u8,
    _dest_token_account: &Pubkey,
) -> Instruction {
    let (config_pda, _) = Pubkey::find_program_address(&[SEED_CONFIG], program_id);
    let mined = Pubkey::new_from_array(mined_pubkey);
    let (claim_pda, _) = Pubkey::find_program_address(&[SEED_CLAIM, mined.as_ref()], program_id);
    let (mint_auth_pda, _) = Pubkey::find_program_address(&[SEED_MINT_AUTH], program_id);
    // NOTE: mint + token program are appended by the client after reading config.
    let mut data = vec![1u8];
    data.extend_from_slice(&mined_pubkey);
    data.push(matched);
    Instruction {
        program_id: *program_id,
        accounts: vec![
            AccountMeta::new(*miner, true),
            AccountMeta::new(config_pda, false),
            AccountMeta::new(claim_pda, false),
            // mint (writable) -- filled by client
            // dest (writable) -- filled by client
            AccountMeta::new_readonly(mint_auth_pda, false),
            AccountMeta::new_readonly(sysvar::instructions::id(), false),
            AccountMeta::new_readonly(spl_token::id(), false),
            AccountMeta::new_readonly(solana_program::system_program::id(), false),
        ],
        data,
    }
}

/// The canonical message a miner signs: prefix || program_id || pubkey || matched.
pub fn claim_message(program_id: &Pubkey, mined_pubkey: &[u8; 32], matched: u8) -> Vec<u8> {
    expected_message(program_id, mined_pubkey, matched)
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
    fn impossible_first_char_rejected() {
        // 'Z', 'g', 'z' can never lead a 32-byte base58 address.
        assert_eq!(validate_pattern(b"Zebra"), Err(GrindError::InvalidPattern));
        assert_eq!(validate_pattern(b"grind"), Err(GrindError::InvalidPattern));
        assert_eq!(validate_pattern(b"zzzzz"), Err(GrindError::InvalidPattern));
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
}
