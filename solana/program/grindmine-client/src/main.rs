//! grindmine-client: testnet demo driver.
//!
//! Commands:
//!   init   --program <id> --payer <keyfile> --pattern Gr1nd --min-tier 4
//!   claim  --program <id> --payer <keyfile> --key <base64-secret> --matched 4
//!   neg    --program <id> --payer <keyfile> --key <base64-secret> --matched 4 --case double|badsig|mismatch
//!
//! The payer is a THROWAWAY testnet wallet. Never mainnet.

use std::str::FromStr;

use borsh::BorshDeserialize;
use grindmine_program::{build_claim_ix, build_initialize_ix, claim_message, Config};
use solana_client::rpc_client::RpcClient;
use solana_sdk::{
    commitment_config::CommitmentConfig,
    instruction::Instruction,
    message::Message,
    pubkey::Pubkey,
    signature::{Keypair, Signer},
    system_instruction,
    transaction::Transaction,
};

fn load_keypair(path: &str) -> Keypair {
    let data = std::fs::read(path).expect("read keyfile");
    let s = String::from_utf8_lossy(&data);
    let s = s.trim();
    // solana-keygen JSON array format
    if s.starts_with('[') {
        let bytes: Vec<u8> = serde_json::from_str(s).expect("parse keyfile json");
        return Keypair::from_bytes(&bytes).expect("keypair from bytes");
    }
    // base64 secret
    let bytes = base64::Engine::decode(&base64::engine::general_purpose::STANDARD, s)
        .expect("base64 keyfile");
    Keypair::from_bytes(&bytes).expect("keypair from bytes")
}

fn send_and_confirm(
    client: &RpcClient,
    ixs: Vec<Instruction>,
    signers: &[&Keypair],
) -> Result<String, String> {
    let blockhash = client.get_latest_blockhash().map_err(|e| e.to_string())?;
    let msg = Message::new_with_blockhash(
        &ixs,
        Some(&signers[0].pubkey()),
        &blockhash,
    );
    let mut tx = Transaction::new_unsigned(msg);
    tx.try_sign(signers, blockhash).map_err(|e| e.to_string())?;
    let sig = client
        .send_and_confirm_transaction_with_spinner(&tx)
        .map_err(|e| e.to_string())?;
    Ok(sig.to_string())
}

fn config_pda(program: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[b"grindmine-cfg"], program)
}

fn read_config(client: &RpcClient, program: &Pubkey) -> Config {
    let (pda, _) = config_pda(program);
    let data = client.get_account_data(&pda).expect("config account");
    Config::deserialize(&mut &data[..]).expect("deserialize config")
}

fn cmd_init(program: Pubkey, payer_path: &str, rpc: &str) {
    let client = RpcClient::new_with_commitment(rpc.to_string(), CommitmentConfig::confirmed());
    let payer = load_keypair(payer_path);

    // 1. Create the SPL mint with the program's mint-auth PDA as authority.
    let (mint_auth_pda, _) = Pubkey::find_program_address(&[b"grindmine-mintauth"], &program);
    let mint_kp = Keypair::new();
    let rent = client.get_minimum_balance_for_rent_exemption(82 /* Mint::LEN */).unwrap();
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
    .unwrap();
    let sig = send_and_confirm(&client, vec![create_mint, init_mint], &[&payer, &mint_kp])
        .expect("create mint");
    println!("mint: {}  (tx {})", mint_kp.pubkey(), sig);

    // 2. Initialize the program config / manifest.
    //    Demo tier table: 4-char "Gr1n" hits pay 1000 tokens, 5-char "Gr1nd" 58000.
    let payouts: [u64; 8] = [0, 0, 0, 1_000_000_000, 58_000_000_000, 0, 0, 0];
    let ix = build_initialize_ix(
        &program,
        &payer.pubkey(),
        &mint_kp.pubkey(),
        b"Gr1nd",
        4,
        payouts,
        100,    // target claims per window
        86400,  // 1-day window
    );
    let sig = send_and_confirm(&client, vec![ix], &[&payer]).expect("initialize");
    let (cfg_pda, _) = config_pda(&program);
    println!("config/manifest PDA: {}  (tx {})", cfg_pda, sig);
    println!("mint_auth PDA: {}", mint_auth_pda);
    // Persist the mint address for later steps.
    std::fs::write("/tmp/grindmine_mint.txt", mint_kp.pubkey().to_string()).unwrap();
}

/// Ensure a plain SPL token account exists for (mint, owner); create+init if missing.
/// The account keypair is persisted so repeated runs reuse it.
fn ensure_token_account(
    client: &RpcClient,
    payer: &Keypair,
    mint: &Pubkey,
    owner: &Pubkey,
) -> Pubkey {
    const ACCT_LEN: usize = 165; // spl_token::state::Account::LEN
    let kp_path = "/tmp/grindmine_tacct.json";
    let token_kp: Keypair = std::fs::read(kp_path)
        .ok()
        .and_then(|d| {
            let v: Vec<u8> = serde_json::from_slice(&d).ok()?;
            Keypair::from_bytes(&v).ok()
        })
        .unwrap_or_else(|| {
            let kp = Keypair::new();
            std::fs::write(kp_path, serde_json::to_string(&kp.to_bytes().to_vec()).unwrap())
                .expect("write tacct key");
            kp
        });
    if client.get_account_data(&token_kp.pubkey()).is_ok() {
        return token_kp.pubkey();
    }
    let rent = client
        .get_minimum_balance_for_rent_exemption(ACCT_LEN)
        .unwrap();
    let create = system_instruction::create_account(
        &payer.pubkey(),
        &token_kp.pubkey(),
        rent,
        ACCT_LEN as u64,
        &spl_token::id(),
    );
    let init = spl_token::instruction::initialize_account(
        &spl_token::id(),
        &token_kp.pubkey(),
        mint,
        owner,
    )
    .unwrap();
    send_and_confirm(client, vec![create, init], &[payer, &token_kp]).expect("create token acct");
    println!("dest token account: {}", token_kp.pubkey());
    token_kp.pubkey()
}

/// Build the ed25519 verify instruction (must be ix #0 in the claim tx).
fn ed25519_ix(message: &[u8], signature: &[u8; 64], pubkey: &[u8; 32]) -> Instruction {
    solana_sdk::ed25519_instruction::new_ed25519_instruction_with_signature(
        message, signature, pubkey,
    )
}

fn do_claim(
    client: &RpcClient,
    program: &Pubkey,
    payer: &Keypair,
    mined_secret_b64: &str,
    matched: u8,
    tamper: Tamper,
) -> Result<String, String> {
    use base64::Engine;
    let secret = base64::engine::general_purpose::STANDARD
        .decode(mined_secret_b64.trim())
        .map_err(|e| e.to_string())?;
    let mined_kp = Keypair::from_bytes(&secret).map_err(|e| e.to_string())?;
    let mined_pubkey = mined_kp.pubkey().to_bytes();

    let cfg = read_config(client, program);
    let dest = ensure_token_account(client, payer, &cfg.mint, &payer.pubkey());

    let message = claim_message(program, &mined_pubkey, matched);
    // Optionally sign with the WRONG key (negative test: bad signature).
    let (sig_bytes, sig_pubkey) = match tamper {
        Tamper::BadSig => {
            let wrong = Keypair::new();
            (wrong.sign_message(&message).as_ref().try_into().unwrap(), wrong.pubkey().to_bytes())
        }
        _ => {
            let s: [u8; 64] = mined_kp.sign_message(&message).as_ref().try_into().unwrap();
            (s, mined_pubkey)
        }
    };

    let mut claim_ix = build_claim_ix(program, &payer.pubkey(), mined_pubkey, matched, &dest);
    // Insert mint (writable) and dest (writable) at the placeholder positions.
    claim_ix.accounts.insert(
        3,
        solana_sdk::instruction::AccountMeta::new(cfg.mint, false),
    );
    claim_ix.accounts.insert(
        4,
        solana_sdk::instruction::AccountMeta::new(dest, false),
    );

    let sig_str = send_and_confirm(
        client,
        vec![ed25519_ix(&message, &sig_bytes, &sig_pubkey), claim_ix],
        &[payer],
    )?;
    Ok(sig_str)
}

#[derive(Clone, Copy, PartialEq)]
enum Tamper {
    None,
    BadSig,
}

fn cmd_claim(program: Pubkey, payer_path: &str, rpc: &str, key_b64: &str, matched: u8) {
    let client = RpcClient::new_with_commitment(rpc.to_string(), CommitmentConfig::confirmed());
    let payer = load_keypair(payer_path);
    match do_claim(&client, &program, &payer, key_b64, matched, Tamper::None) {
        Ok(sig) => println!("CLAIM OK: {}", sig),
        Err(e) => {
            println!("CLAIM FAILED: {}", e);
            std::process::exit(1);
        }
    }
}

fn cmd_neg(program: Pubkey, payer_path: &str, rpc: &str, key_b64: &str, matched: u8, case: &str) {
    let client = RpcClient::new_with_commitment(rpc.to_string(), CommitmentConfig::confirmed());
    let payer = load_keypair(payer_path);
    let (ok, label) = match case {
        // Submit the identical claim twice; the second must fail (AlreadyClaimed).
        "double" => {
            let first = do_claim(&client, &program, &payer, key_b64, matched, Tamper::None);
            match first {
                Ok(s) => println!("first claim (setup): {}", s),
                Err(e) => {
                    // If the first already fails, the key was claimed before; the
                    // negative property we want is that a repeat fails.
                    println!("first claim failed (already claimed?): {}", e);
                }
            }
            let second = do_claim(&client, &program, &payer, key_b64, matched, Tamper::None);
            (second.is_err(), "double-claim rejected")
        }
        // Signature made by the wrong key: the ed25519 precompile fails.
        "badsig" => {
            let r = do_claim(&client, &program, &payer, key_b64, matched, Tamper::BadSig);
            (r.is_err(), "bad signature rejected")
        }
        // matched=5 claimed for a key that only matches 4 chars: pattern mismatch.
        // (Caller passes a 4-char key with --matched 5.)
        "mismatch" => {
            let r = do_claim(&client, &program, &payer, key_b64, matched, Tamper::None);
            (r.is_err(), "pattern mismatch rejected")
        }
        _ => {
            eprintln!("unknown case: {}", case);
            std::process::exit(2);
        }
    };
    if ok {
        println!("NEGATIVE TEST PASSED: {}", label);
    } else {
        println!("NEGATIVE TEST FAILED: {} was NOT rejected", label);
        std::process::exit(1);
    }
}

fn print_usage() {
    eprintln!("usage:");
    eprintln!("  grindmine-client init --program <id> --payer <keyfile> [--rpc <url>]");
    eprintln!("  grindmine-client claim --program <id> --payer <keyfile> --key <b64secret> --matched <n> [--rpc <url>]");
    eprintln!("  grindmine-client neg --program <id> --payer <keyfile> --key <b64secret> --matched <n> --case double|badsig|mismatch [--rpc <url>]");
}

fn get_arg(args: &[String], name: &str) -> Option<String> {
    args.windows(2)
        .find(|w| w[0] == name)
        .map(|w| w[1].clone())
}

fn main() {
    // serde_json is used for solana-keygen keyfile parsing.
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        print_usage();
        std::process::exit(2);
    }
    let cmd = args[1].as_str();
    let program = Pubkey::from_str(&get_arg(&args, "--program").expect("--program")).unwrap();
    let payer_path = get_arg(&args, "--payer").expect("--payer");
    let rpc = get_arg(&args, "--rpc").unwrap_or_else(|| "https://api.testnet.solana.com".into());
    match cmd {
        "init" => cmd_init(program, &payer_path, &rpc),
        "claim" => {
            let key = get_arg(&args, "--key").expect("--key");
            let matched: u8 = get_arg(&args, "--matched").expect("--matched").parse().unwrap();
            cmd_claim(program, &payer_path, &rpc, &key, matched);
        }
        "neg" => {
            let key = get_arg(&args, "--key").expect("--key");
            let matched: u8 = get_arg(&args, "--matched").expect("--matched").parse().unwrap();
            let case = get_arg(&args, "--case").expect("--case");
            cmd_neg(program, &payer_path, &rpc, &key, matched, &case);
        }
        _ => {
            print_usage();
            std::process::exit(2);
        }
    }
}
