//! grindmine-client: testnet demo driver.
//!
//! Commands:
//!   init       --program <id> --payer <keyfile> [--cluster testnet|devnet] [--rpc <url>] [--yes]
//!   claim      --program <id> --payer <keyfile> (--keyfile <64B-b64> | --hits <jsonl> --address <addr>)
//!              --matched <n> [--cluster testnet|devnet] [--rpc <url>] [--yes]
//!   neg        --program <id> --payer <keyfile> (--keyfile ... | --hits ... --address ...)
//!              --matched <n> --case double|badsig|mismatch [--cluster ...] [--rpc <url>] [--yes]
//!   config-pda --program <id>
//!
//! Claim message (v2 — must match the on-chain program):
//!   MESSAGE_PREFIX_V2 || program_id || config.mint || miner_pubkey ||
//!   dest_token_account || mined_pubkey || matched
//! (all pubkeys 32 bytes raw, matched 1 byte).
//!
//! The payer is a THROWAWAY testnet wallet. Mainnet is refused outright.

use std::str::FromStr;

use borsh::BorshDeserialize;
use grindmine_program::{build_claim_ix, build_initialize_ix, claim_message, Config};
use solana_client::{
    client_error::{ClientError, ClientErrorKind},
    rpc_client::RpcClient,
};
use solana_sdk::{
    commitment_config::CommitmentConfig,
    hash::Hash,
    instruction::{AccountMeta, Instruction, InstructionError},
    message::Message,
    program_pack::Pack,
    pubkey::Pubkey,
    signature::{Keypair, Signer},
    system_instruction,
    transaction::{Transaction, TransactionError},
};

/// Fixed-point scale the program uses for the trailing multiplier (estimate only).
const MULTIPLIER_SCALE: u64 = 1_000_000;
/// spl-associated-token-account program id.
const ATA_PROGRAM_ID: &str = "ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL";

const GENESIS_TESTNET: &str = "4uhcVJyU9pJkvQyS88uQjXX3zTWB8c9EgnE7C2zX6f";
const GENESIS_DEVNET: &str = "EtWTRABZaYq6iMfeYKouRu166VU2xqa1LYK9Wj6qqb9ng";
const GENESIS_MAINNET: &str = "5eykt4UsFv6P8NJdTREpY1vzqKqZKvdpKuc147dw2N9d";

#[derive(Debug)]
enum ClaimError {
    Msg(String),
    Tx(TransactionError),
}

impl ClaimError {
    fn msg(s: impl Into<String>) -> Self {
        ClaimError::Msg(s.into())
    }
}

/// Extract a custom program error code if present, either structured or from
/// an RPC error string ("custom program error: 0x7").
fn custom_error_code(e: &ClaimError) -> Option<u32> {
    match e {
        ClaimError::Tx(TransactionError::InstructionError(_, InstructionError::Custom(c))) => {
            Some(*c)
        }
        ClaimError::Msg(s) => {
            let marker = "custom program error: 0x";
            s.find(marker).and_then(|i| {
                let hex: String = s[i + marker.len()..]
                    .chars()
                    .take_while(|c| c.is_ascii_hexdigit())
                    .collect();
                u32::from_str_radix(&hex, 16).ok()
            })
        }
        _ => None,
    }
}

/// Reject keyfiles readable by group/other. Secrets must be 0600.
fn check_keyfile_perms(path: &str) -> Result<(), String> {
    use std::os::unix::fs::MetadataExt;
    let md = std::fs::metadata(path).map_err(|_| "cannot read keyfile".to_string())?;
    if md.mode() & 0o077 != 0 {
        return Err("keyfile is readable by group/other; chmod 600 it first".to_string());
    }
    Ok(())
}

fn load_keypair(path: &str) -> Result<Keypair, String> {
    check_keyfile_perms(path)?;
    let data = std::fs::read(path).map_err(|_| "cannot read keyfile".to_string())?;
    let s = String::from_utf8_lossy(&data);
    let s = s.trim();
    // solana-keygen JSON array format
    if s.starts_with('[') {
        let bytes: Vec<u8> = serde_json::from_str(s).map_err(|_| "bad keyfile format".to_string())?;
        return Keypair::from_bytes(&bytes).map_err(|_| "bad keyfile format".to_string());
    }
    // base64 secret
    let bytes = base64::Engine::decode(&base64::engine::general_purpose::STANDARD, s)
        .map_err(|_| "bad keyfile format".to_string())?;
    Keypair::from_bytes(&bytes).map_err(|_| "bad keyfile format".to_string())
}

/// Build an RPC client guarded by cluster: the genesis hash must match the
/// declared cluster, and mainnet is refused outright (throwaway test wallet).
fn make_client(cluster: &str, rpc_override: Option<&str>) -> Result<RpcClient, String> {
    let (default_rpc, genesis) = match cluster {
        "testnet" => ("https://api.testnet.solana.com", GENESIS_TESTNET),
        "devnet" => ("https://api.devnet.solana.com", GENESIS_DEVNET),
        "mainnet-beta" => return Err("refusing mainnet: this is a testnet-only driver".to_string()),
        other => return Err(format!("unknown --cluster '{}'; want testnet|devnet", other)),
    };
    let rpc = rpc_override.unwrap_or(default_rpc).to_string();
    let client = RpcClient::new_with_commitment(rpc, CommitmentConfig::confirmed());
    let got = client
        .get_genesis_hash()
        .map_err(|e| format!("genesis check failed: {}", e))?;
    if got != Hash::from_str(genesis).unwrap() {
        return Err(format!(
            "genesis hash mismatch for cluster '{}': refusing to talk to this RPC",
            cluster
        ));
    }
    // Defense in depth: never mainnet, even if someone renames the flag.
    if got == Hash::from_str(GENESIS_MAINNET).unwrap() {
        return Err("refusing mainnet: this is a testnet-only driver".to_string());
    }
    Ok(client)
}

fn send_and_confirm(
    client: &RpcClient,
    ixs: Vec<Instruction>,
    signers: &[&Keypair],
) -> Result<String, ClaimError> {
    let blockhash = client
        .get_latest_blockhash()
        .map_err(|e| ClaimError::msg(e.to_string()))?;
    let msg = Message::new_with_blockhash(&ixs, Some(&signers[0].pubkey()), &blockhash);
    let mut tx = Transaction::new_unsigned(msg);
    tx.try_sign(signers, blockhash)
        .map_err(|e| ClaimError::msg(e.to_string()))?;
    let sig = client
        .send_and_confirm_transaction_with_spinner(&tx)
        .map_err(|e: ClientError| match e.kind {
            ClientErrorKind::TransactionError(tx_err) => ClaimError::Tx(tx_err),
            _ => ClaimError::msg(e.to_string()),
        })?;
    Ok(sig.to_string())
}

fn config_pda(program: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[b"grindmine-cfg"], program)
}

fn read_config(client: &RpcClient, program: &Pubkey) -> Result<Config, ClaimError> {
    let (pda, _) = config_pda(program);
    let data = client
        .get_account_data(&pda)
        .map_err(|e| ClaimError::msg(format!("config PDA missing: {}", e)))?;
    Config::deserialize(&mut &data[..])
        .map_err(|_| ClaimError::msg("config deserialize failed".to_string()))
}

fn ata_address(owner: &Pubkey, mint: &Pubkey) -> Pubkey {
    let ata_program = Pubkey::from_str(ATA_PROGRAM_ID).expect("ata program id");
    Pubkey::find_program_address(
        &[owner.as_ref(), spl_token::id().as_ref(), mint.as_ref()],
        &ata_program,
    )
    .0
}

fn create_ata_ix(payer: &Pubkey, ata: &Pubkey, owner: &Pubkey, mint: &Pubkey) -> Instruction {
    let ata_program = Pubkey::from_str(ATA_PROGRAM_ID).expect("ata program id");
    Instruction {
        program_id: ata_program,
        accounts: vec![
            AccountMeta::new(*payer, true),
            AccountMeta::new(*ata, false),
            AccountMeta::new_readonly(*owner, false),
            AccountMeta::new_readonly(*mint, false),
            AccountMeta::new_readonly(solana_sdk::system_program::id(), false),
            AccountMeta::new_readonly(spl_token::id(), false),
        ],
        data: vec![],
    }
}

/// Resolve the destination to the Associated Token Account for (owner, mint).
/// If the ATA already exists, deserialize it and verify mint/owner/state
/// before trusting it — a pre-planted account must never receive rewards.
fn ensure_token_account(
    client: &RpcClient,
    payer: &Keypair,
    mint: &Pubkey,
    owner: &Pubkey,
) -> Result<Pubkey, ClaimError> {
    let ata = ata_address(owner, mint);
    match client.get_account_data(&ata) {
        Ok(data) => {
            let acct = spl_token::state::Account::unpack(&data)
                .map_err(|_| ClaimError::msg("ATA exists but is not a token account"))?;
            if acct.mint != *mint {
                return Err(ClaimError::msg("ATA mint mismatch: refusing hijacked account"));
            }
            if acct.owner != *owner {
                return Err(ClaimError::msg("ATA owner mismatch: refusing hijacked account"));
            }
            if acct.state != spl_token::state::AccountState::Initialized {
                return Err(ClaimError::msg("ATA not initialized: refusing to use it"));
            }
            println!("dest token account (ATA): {}", ata);
            Ok(ata)
        }
        Err(_) => {
            let ix = create_ata_ix(&payer.pubkey(), &ata, owner, mint);
            send_and_confirm(client, vec![ix], &[payer])?;
            println!("dest token account (ATA created): {}", ata);
            Ok(ata)
        }
    }
}

/// Build the ed25519 verify instruction (must be ix #0 in the claim tx).
fn ed25519_ix(message: &[u8], signature: &[u8; 64], pubkey: &[u8; 32]) -> Instruction {
    solana_sdk::ed25519_instruction::new_ed25519_instruction_with_signature(
        message, signature, pubkey,
    )
}

/// Resolve the mined keypair: either a raw 64-byte base64 --keyfile, or a
/// --hits JSONL line selected by --address (secret_b64 = base64(seed||pubkey)).
fn resolve_mined_keypair(
    keyfile: Option<&str>,
    hits: Option<&str>,
    address: Option<&str>,
) -> Result<Keypair, String> {
    use base64::Engine;
    if let (Some(hits_path), Some(addr)) = (hits, address) {
        check_keyfile_perms(hits_path)?;
        let content =
            std::fs::read_to_string(hits_path).map_err(|_| "cannot read hits file".to_string())?;
        for line in content.lines() {
            let v: serde_json::Value =
                serde_json::from_str(line).map_err(|_| "bad hits file format".to_string())?;
            if v.get("address").and_then(|a| a.as_str()) == Some(addr) {
                let b64 = v
                    .get("secret_b64")
                    .and_then(|s| s.as_str())
                    .ok_or_else(|| "hits line missing secret_b64".to_string())?;
                let secret = base64::engine::general_purpose::STANDARD
                    .decode(b64)
                    .map_err(|_| "bad secret encoding".to_string())?;
                if secret.len() != 64 {
                    return Err("secret must be 64 bytes (seed||pubkey)".to_string());
                }
                return Keypair::from_bytes(&secret)
                    .map_err(|_| "bad secret encoding".to_string());
            }
        }
        return Err("address not found in hits file".to_string());
    }
    if let Some(path) = keyfile {
        check_keyfile_perms(path)?;
        let raw = std::fs::read_to_string(path).map_err(|_| "cannot read keyfile".to_string())?;
        let secret = base64::engine::general_purpose::STANDARD
            .decode(raw.trim())
            .map_err(|_| "bad keyfile format".to_string())?;
        if secret.len() != 64 {
            return Err("keyfile secret must be 64 bytes (seed||pubkey)".to_string());
        }
        return Keypair::from_bytes(&secret).map_err(|_| "bad keyfile format".to_string());
    }
    Err("need --keyfile <64B-b64> or --hits <jsonl> --address <addr>".to_string())
}

fn confirm_claim(dest: &Pubkey, mint: &Pubkey, matched: u8, est_payout: u64, yes: bool) -> Result<(), String> {
    if yes {
        return Ok(());
    }
    println!("About to sign a claim:");
    println!("  recipient (ATA): {}", dest);
    println!("  mint:            {}", mint);
    println!("  matched chars:   {}", matched);
    println!("  est. payout:     {} base units (estimate; multiplier moves)", est_payout);
    print!("Type YES to sign> ");
    use std::io::Write;
    std::io::stdout()
        .flush()
        .map_err(|e| e.to_string())?;
    let mut line = String::new();
    std::io::stdin()
        .read_line(&mut line)
        .map_err(|e| e.to_string())?;
    if line.trim() == "YES" {
        Ok(())
    } else {
        Err("aborted: confirmation not given".to_string())
    }
}

#[derive(Clone, Copy, PartialEq)]
enum Tamper {
    None,
    BadSig,
}

fn do_claim(
    client: &RpcClient,
    program: &Pubkey,
    payer: &Keypair,
    mined_kp: &Keypair,
    matched: u8,
    tamper: Tamper,
    yes: bool,
) -> Result<String, ClaimError> {
    let mined_pubkey = mined_kp.pubkey().to_bytes();

    let cfg = read_config(client, program)?;
    let dest = ensure_token_account(client, payer, &cfg.mint, &payer.pubkey())?;

    let message = claim_message(program, &cfg.mint, &payer.pubkey(), &dest, &mined_pubkey, matched);

    let base = cfg.payouts.get((matched as usize).saturating_sub(1)).copied().unwrap_or(0);
    let est = base.saturating_mul(cfg.multiplier) / MULTIPLIER_SCALE;
    confirm_claim(&dest, &cfg.mint, matched, est, yes).map_err(ClaimError::msg)?;

    // Optionally sign with the WRONG key (negative test: bad signature).
    let (sig_bytes, sig_pubkey) = match tamper {
        Tamper::BadSig => {
            let wrong = Keypair::new();
            (
                wrong.sign_message(&message).as_ref().try_into().unwrap(),
                wrong.pubkey().to_bytes(),
            )
        }
        _ => {
            let s: [u8; 64] = mined_kp.sign_message(&message).as_ref().try_into().unwrap();
            (s, mined_pubkey)
        }
    };

    // All claim accounts come from the program's builder, verbatim, in
    // program order. No client-side splicing.
    let claim_ix = build_claim_ix(
        program,
        &payer.pubkey(),
        &cfg.mint,
        &dest,
        mined_pubkey,
        matched,
    );

    send_and_confirm(
        client,
        vec![ed25519_ix(&message, &sig_bytes, &sig_pubkey), claim_ix],
        &[payer],
    )
}

fn cmd_init(program: Pubkey, payer_path: &str, client: &RpcClient, yes: bool) -> Result<(), String> {
    let payer = load_keypair(payer_path).map_err(|e| e)?;

    // Mint creation + program initialize in ONE transaction: no window for a
    // front-runner to initialize the config PDA with their own mint.
    let (mint_auth_pda, _) = Pubkey::find_program_address(&[b"grindmine-mintauth"], &program);
    let mint_kp = Keypair::new();
    let rent = client
        .get_minimum_balance_for_rent_exemption(82 /* Mint::LEN */)
        .map_err(|e| e.to_string())?;
    let create_mint = system_instruction::create_account(
        &payer.pubkey(),
        &mint_kp.pubkey(),
        rent,
        82 /* Mint::LEN */ as u64,
        &spl_token::id(),
    );
    let init_mint = spl_token::instruction::initialize_mint(
        &spl_token::id(),
        &mint_kp.pubkey(),
        &mint_auth_pda,
        None,
        6,
    )
    .map_err(|e| e.to_string())?;

    // Demo tier table: 4-char hits pay 1000 tokens, 5-char 58000 (index = matched-1).
    // Demo supply cap: 1B tokens (6 decimals) — enforced on-chain (red-team C1).
    let pattern = b"Gr1nd";
    let min_tier: u8 = 4;
    let payouts: [u64; 8] = [0, 0, 0, 1_000_000_000, 58_000_000_000, 0, 0, 0];
    let target: u64 = 100;
    let window_secs: i64 = 86400;
    let max_supply: u64 = 1_000_000_000_000_000;
    let init_ix = build_initialize_ix(
        &program,
        &payer.pubkey(),
        &mint_kp.pubkey(),
        pattern,
        min_tier,
        payouts,
        target,
        window_secs,
        max_supply,
    );

    println!("About to initialize: mint={} pattern=Gr1nd min_tier=4", mint_kp.pubkey());
    if !yes {
        print!("Type YES to send the atomic init transaction> ");
        use std::io::Write;
        std::io::stdout().flush().map_err(|e| e.to_string())?;
        let mut line = String::new();
        std::io::stdin().read_line(&mut line).map_err(|e| e.to_string())?;
        if line.trim() != "YES" {
            return Err("aborted: confirmation not given".to_string());
        }
    }

    let sig = send_and_confirm(client, vec![create_mint, init_mint, init_ix], &[&payer, &mint_kp])
        .map_err(|e| format!("init tx failed: {:?}", e))?;
    println!("init tx: {}", sig);

    // Read back and verify every manifest field; abort on any mismatch.
    let cfg = read_config(client, &program).map_err(|e| format!("init verify failed: {:?}", e))?;
    let mut bad = Vec::new();
    if cfg.mint != mint_kp.pubkey() { bad.push("mint"); }
    if cfg.pattern_len != pattern.len() as u8 || &cfg.pattern[..pattern.len()] != pattern { bad.push("pattern"); }
    if cfg.min_tier != min_tier { bad.push("min_tier"); }
    if cfg.payouts != payouts { bad.push("payouts"); }
    if cfg.target_claims_per_window != target { bad.push("target_claims_per_window"); }
    if cfg.window_secs != window_secs { bad.push("window_secs"); }
    if cfg.max_supply != max_supply { bad.push("max_supply"); }
    if !bad.is_empty() {
        return Err(format!("init verify FAILED on {:?}: aborting", bad));
    }
    let (cfg_pda, _) = config_pda(&program);
    println!("init verified: config PDA {} mint {}", cfg_pda, mint_kp.pubkey());
    println!("mint_auth PDA: {}", mint_auth_pda);
    Ok(())
}

fn cmd_claim(
    program: Pubkey,
    payer_path: &str,
    client: &RpcClient,
    keyfile: Option<&str>,
    hits: Option<&str>,
    address: Option<&str>,
    matched: u8,
    yes: bool,
) {
    let payer = load_keypair(payer_path).unwrap_or_else(|e| {
        eprintln!("payer: {}", e);
        std::process::exit(1);
    });
    let mined_kp = resolve_mined_keypair(keyfile, hits, address).unwrap_or_else(|e| {
        eprintln!("mined key: {}", e);
        std::process::exit(1);
    });
    match do_claim(client, &program, &payer, &mined_kp, matched, Tamper::None, yes) {
        Ok(sig) => println!("CLAIM OK: {}", sig),
        Err(e) => {
            println!("CLAIM FAILED: {:?}", e);
            std::process::exit(1);
        }
    }
}

/// Grinder program error codes (must match grindmine-program GrindError).
const ERR_PATTERN_MISMATCH: u32 = 3;
const ERR_SIGNATURE_PUBKEY_MISMATCH: u32 = 5;
const ERR_ALREADY_CLAIMED: u32 = 7;

fn expect_custom(label: &str, r: &Result<String, ClaimError>, want: u32) -> bool {
    match r {
        Err(e) => match custom_error_code(e) {
            Some(c) if c == want => {
                println!("ok: {} rejected with Custom({})", label, c);
                true
            }
            other => {
                println!(
                    "FAIL: {} rejected but with {:?}, want Custom({})",
                    label, other, want
                );
                false
            }
        },
        Ok(sig) => {
            println!("FAIL: {} was NOT rejected (tx {})", label, sig);
            false
        }
    }
}

fn cmd_neg(
    program: Pubkey,
    payer_path: &str,
    client: &RpcClient,
    keyfile: Option<&str>,
    hits: Option<&str>,
    address: Option<&str>,
    matched: u8,
    case: &str,
    yes: bool,
) {
    let payer = load_keypair(payer_path).unwrap_or_else(|e| {
        eprintln!("payer: {}", e);
        std::process::exit(1);
    });
    let mined_kp = resolve_mined_keypair(keyfile, hits, address).unwrap_or_else(|e| {
        eprintln!("mined key: {}", e);
        std::process::exit(1);
    });
    let ok = match case {
        // Submit the identical claim twice; the first MUST succeed and the
        // second MUST fail with AlreadyClaimed (Custom 7).
        "double" => {
            let first = do_claim(client, &program, &payer, &mined_kp, matched, Tamper::None, yes);
            match first {
                Ok(s) => println!("first claim (setup) ok: {}", s),
                Err(e) => {
                    println!("FAIL: setup claim did not succeed: {:?}", e);
                    std::process::exit(1);
                }
            }
            let second = do_claim(client, &program, &payer, &mined_kp, matched, Tamper::None, yes);
            expect_custom("double-claim", &second, ERR_ALREADY_CLAIMED)
        }
        // Signature made by the wrong key: precompile passes, program must
        // reject with SignaturePubkeyMismatch (Custom 5).
        "badsig" => {
            let r = do_claim(client, &program, &payer, &mined_kp, matched, Tamper::BadSig, yes);
            expect_custom("bad signature", &r, ERR_SIGNATURE_PUBKEY_MISMATCH)
        }
        // matched=5 claimed for a key that only matches 4 chars:
        // PatternMismatch (Custom 3). Caller passes a 4-char key with --matched 5.
        "mismatch" => {
            let r = do_claim(client, &program, &payer, &mined_kp, matched, Tamper::None, yes);
            expect_custom("pattern mismatch", &r, ERR_PATTERN_MISMATCH)
        }
        _ => {
            eprintln!("unknown case: {}", case);
            std::process::exit(2);
        }
    };
    if ok {
        println!("NEGATIVE TEST PASSED");
    } else {
        println!("NEGATIVE TEST FAILED");
        std::process::exit(1);
    }
}

fn print_usage() {
    eprintln!("usage:");
    eprintln!("  grindmine-client config-pda --program <id>");
    eprintln!("  grindmine-client init --program <id> --payer <keyfile> [--cluster testnet|devnet] [--rpc <url>] [--yes]");
    eprintln!("  grindmine-client claim --program <id> --payer <keyfile> (--keyfile <64B-b64> | --hits <jsonl> --address <addr>) --matched <n> [--cluster ...] [--rpc <url>] [--yes]");
    eprintln!("  grindmine-client neg --program <id> --payer <keyfile> (--keyfile ... | --hits ... --address ...) --matched <n> --case double|badsig|mismatch [--cluster ...] [--rpc <url>] [--yes]");
    eprintln!("notes: --cluster defaults to testnet; mainnet is refused. Keyfiles must be mode 0600.");
}

fn get_arg(args: &[String], name: &str) -> Option<String> {
    args.windows(2)
        .find(|w| w[0] == name)
        .and_then(|w| {
            // Reject flag-looking values (a missing value must not swallow the next flag).
            if w[1].starts_with("--") {
                None
            } else {
                Some(w[1].clone())
            }
        })
}

fn has_flag(args: &[String], name: &str) -> bool {
    args.iter().any(|a| a == name)
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        print_usage();
        std::process::exit(2);
    }
    let cmd = args[1].as_str();
    if cmd == "config-pda" {
        let program = Pubkey::from_str(&get_arg(&args, "--program").expect("--program")).unwrap();
        let (pda, _) = config_pda(&program);
        println!("{}", pda);
        return;
    }
    let program = Pubkey::from_str(&get_arg(&args, "--program").expect("--program")).unwrap();
    let payer_path = get_arg(&args, "--payer").expect("--payer");
    let cluster = get_arg(&args, "--cluster").unwrap_or_else(|| "testnet".to_string());
    let rpc_override = get_arg(&args, "--rpc");
    let yes = has_flag(&args, "--yes");
    let client = make_client(&cluster, rpc_override.as_deref()).unwrap_or_else(|e| {
        eprintln!("rpc: {}", e);
        std::process::exit(1);
    });
    match cmd {
        "init" => {
            if let Err(e) = cmd_init(program, &payer_path, &client, yes) {
                eprintln!("init: {}", e);
                std::process::exit(1);
            }
        }
        "claim" => {
            let keyfile = get_arg(&args, "--keyfile");
            let hits = get_arg(&args, "--hits");
            let address = get_arg(&args, "--address");
            let matched: u8 = get_arg(&args, "--matched").expect("--matched").parse().unwrap();
            cmd_claim(
                program,
                &payer_path,
                &client,
                keyfile.as_deref(),
                hits.as_deref(),
                address.as_deref(),
                matched,
                yes,
            );
        }
        "neg" => {
            let keyfile = get_arg(&args, "--keyfile");
            let hits = get_arg(&args, "--hits");
            let address = get_arg(&args, "--address");
            let matched: u8 = get_arg(&args, "--matched").expect("--matched").parse().unwrap();
            let case = get_arg(&args, "--case").expect("--case");
            cmd_neg(
                program,
                &payer_path,
                &client,
                keyfile.as_deref(),
                hits.as_deref(),
                address.as_deref(),
                matched,
                &case,
                yes,
            );
        }
        _ => {
            print_usage();
            std::process::exit(2);
        }
    }
}
