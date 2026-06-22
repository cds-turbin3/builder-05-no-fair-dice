//! The dice table, dealt through the testsvm-quasar engine.
//!
//! The dice game's one genuinely hard primitive is instruction introspection:
//! `resolve_bet` reads the preceding `reveal` out of the Instructions sysvar to
//! recover the preimage and confirm the house signed it. So the settle verb is a
//! single transaction carrying TWO instructions, `[reveal, resolve_bet]`, and the
//! whole point of this suite is to prove that parse works on a live engine, not
//! just byte-exact on paper.
//!
//! The vocabulary is the game: the house `opens the table` (seeds its vault), a
//! player `places a bet` (commits a stake against `sha256(preimage)`), the house
//! `reveals and settles`, or the player `walks` when the house never shows. The
//! roll is derived host-side too, so each scenario picks a guess that is meant to
//! win or lose and then asserts the table paid out accordingly.

use {
    sha2::{Digest, Sha256},
    solana_instruction::{AccountMeta, Instruction},
    solana_keypair::Keypair,
    solana_pubkey::Pubkey,
    solana_signer::Signer,
    std::str::FromStr,
    testsvm::{
        model::Transaction,
        report::{render_index, render_scenario},
        TestSVM,
    },
    testsvm_quasar::QuasarBackend,
};

const DICE_SO: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../programs/quasar-dice/target/deploy/quasar_dice.so"
);

/// The dice program's `declare_id!`; the deploy address must match so its PDAs
/// derive against it.
fn dice_id() -> Pubkey {
    Pubkey::from_str("8hjbFtnfY87ZzpEpx26u5tx4KjxkrqiWGUEcWFbDNn7h").unwrap()
}
fn instructions_sysvar() -> Pubkey {
    Pubkey::from_str("Sysvar1nstructions1111111111111111111111111").unwrap()
}
fn system_program() -> Pubkey {
    Pubkey::from_str("11111111111111111111111111111111").unwrap()
}

const BANKROLL: u64 = 100_000_000_000;
const STAKE: u64 = 50_000_000; // 0.05 SOL, above the program's 0.01 minimum
const PLAYER_FUNDS: u64 = 5_000_000_000;
/// Must match the program's `REFUND_TIMEOUT_SLOTS`.
const REFUND_TIMEOUT_SLOTS: u64 = 1000;
/// The player's open entropy. Fixed here so the suite can compute the roll and
/// pick a winning or losing guess; in the wild the player chooses it freshly.
const PLAYER_ENTROPY: [u8; 32] = [0x5a; 32];

// --- the odds (mirrors the program's roll derivation) ----------------------

/// `sha256(preimage)`, the commitment a bet locks in.
fn commit(preimage: &[u8; 32]) -> [u8; 32] {
    let mut h = Sha256::new();
    h.update(preimage);
    h.finalize().into()
}

/// The roll the program derives: `sha256(preimage ++ entropy)`, split into two
/// little-endian u128 halves, added (wrapping), mod 100, plus one. Range 1..=100.
fn roll_of(preimage: &[u8; 32]) -> u8 {
    let mut buf = [0u8; 64];
    buf[..32].copy_from_slice(preimage);
    buf[32..].copy_from_slice(&PLAYER_ENTROPY);
    let mut hasher = Sha256::new();
    hasher.update(buf);
    let h: [u8; 32] = hasher.finalize().into();
    let lower = u128::from_le_bytes(h[0..16].try_into().unwrap());
    let upper = u128::from_le_bytes(h[16..32].try_into().unwrap());
    (lower.wrapping_add(upper).wrapping_rem(100) as u8) + 1
}

/// Find a `[k; 32]` preimage whose roll satisfies `want`.
fn preimage_where(want: impl Fn(u8) -> bool) -> ([u8; 32], u8) {
    for k in 0u8..=255 {
        let p = [k; 32];
        let r = roll_of(&p);
        if want(r) {
            return (p, r);
        }
    }
    panic!("no preimage [k; 32] matched the predicate");
}

// --- PDAs ------------------------------------------------------------------

fn vault_of(house: &Pubkey, id: &Pubkey) -> Pubkey {
    Pubkey::find_program_address(&[b"vault", house.as_ref()], id).0
}
fn bet_of(vault: &Pubkey, player: &Pubkey, seed: u64, id: &Pubkey) -> Pubkey {
    Pubkey::find_program_address(
        &[
            b"bet",
            vault.as_ref(),
            player.as_ref(),
            seed.to_le_bytes().as_ref(),
        ],
        id,
    )
    .0
}

// --- instruction layouts ([discriminator] ++ packed LE args) ---------------

fn initialize_ix(id: Pubkey, house: Pubkey, vault: Pubkey, amount: u64) -> Instruction {
    let mut data = vec![0u8];
    data.extend_from_slice(&amount.to_le_bytes());
    Instruction {
        program_id: id,
        accounts: vec![
            AccountMeta::new(house, true),
            AccountMeta::new(vault, false),
            AccountMeta::new_readonly(system_program(), false),
        ],
        data,
    }
}

#[allow(clippy::too_many_arguments)]
fn place_bet_ix(
    id: Pubkey,
    player: Pubkey,
    house: Pubkey,
    vault: Pubkey,
    bet: Pubkey,
    seed: u64,
    amount: u64,
    guess: u8,
    commitment: [u8; 32],
    entropy: [u8; 32],
) -> Instruction {
    let mut data = vec![1u8];
    data.extend_from_slice(&seed.to_le_bytes());
    data.extend_from_slice(&amount.to_le_bytes());
    data.push(guess);
    data.extend_from_slice(&commitment);
    data.extend_from_slice(&entropy);
    Instruction {
        program_id: id,
        accounts: vec![
            AccountMeta::new(player, true),
            AccountMeta::new_readonly(house, false),
            AccountMeta::new(vault, false),
            AccountMeta::new(bet, false),
            AccountMeta::new_readonly(system_program(), false),
        ],
        data,
    }
}

fn reveal_ix(id: Pubkey, house: Pubkey, preimage: [u8; 32]) -> Instruction {
    let mut data = vec![2u8];
    data.extend_from_slice(&preimage);
    Instruction {
        program_id: id,
        accounts: vec![AccountMeta::new_readonly(house, true)],
        data,
    }
}

fn resolve_ix(
    id: Pubkey,
    house: Pubkey,
    player: Pubkey,
    vault: Pubkey,
    bet: Pubkey,
    seed: u64,
) -> Instruction {
    let mut data = vec![3u8];
    data.extend_from_slice(&seed.to_le_bytes());
    Instruction {
        program_id: id,
        accounts: vec![
            AccountMeta::new(house, true),
            AccountMeta::new(player, false),
            AccountMeta::new(vault, false),
            AccountMeta::new(bet, false),
            AccountMeta::new_readonly(instructions_sysvar(), false),
            AccountMeta::new_readonly(system_program(), false),
        ],
        data,
    }
}

fn refund_ix(
    id: Pubkey,
    player: Pubkey,
    house: Pubkey,
    vault: Pubkey,
    bet: Pubkey,
    seed: u64,
) -> Instruction {
    let mut data = vec![4u8];
    data.extend_from_slice(&seed.to_le_bytes());
    Instruction {
        program_id: id,
        accounts: vec![
            AccountMeta::new(player, true),
            AccountMeta::new_readonly(house, false),
            AccountMeta::new(vault, false),
            AccountMeta::new(bet, false),
            AccountMeta::new_readonly(system_program(), false),
        ],
        data,
    }
}

// --- the gambling DSL ------------------------------------------------------

/// A house with a funded vault, ready to take bets.
struct Table {
    id: Pubkey,
    house: Keypair,
    vault: Pubkey,
}

/// The house opens the table and seeds its vault with the bankroll.
fn open_table(backend: &mut QuasarBackend) -> Table {
    let id = dice_id();
    backend.deploy_from_file(&id, DICE_SO, "dice");

    // Name the instructions and errors so the report reads in the game's terms:
    // the sequence diagram shows `reveal` / `resolve_bet`, and a failed settle
    // names the error instead of a bare `0x4`. (Quasar maps `#[error_code]` to
    // `Custom(ordinal)`, no Anchor-style 6000 offset.)
    for (disc, name) in [
        (0u8, "initialize"),
        (1, "place_bet"),
        (2, "reveal"),
        (3, "resolve_bet"),
        (4, "refund_bet"),
    ] {
        backend.register_instruction_name(&id, &[disc], name);
    }
    for (code, name) in [
        (3u32, "BadSignature"),
        (4, "CommitRevealMismatch"),
        (5, "Overflow"),
        (6, "TimeoutNotReached"),
        (7, "NotReveal"),
    ] {
        backend.register_error_name(&id, code, name);
    }

    let house = backend.actor("House", BANKROLL + 1_000_000_000);
    let vault = vault_of(&house.pubkey(), &id);
    backend.register_alias(&vault, "Vault");

    let ix = initialize_ix(id, house.pubkey(), vault, BANKROLL);
    let tx = backend.send(&[ix], &[&house]);
    println!("\n=== the house opens the table ===\n{}", tx.pretty_cpi_tree());
    assert!(tx.error.is_none(), "open table: {:?}", tx.error);
    assert_eq!(
        backend.get_account(&vault).map(|a| a.lamports),
        Some(BANKROLL),
        "the vault holds the bankroll"
    );

    Table { id, house, vault }
}

/// A player commits a stake against `sha256(preimage)` with a chosen guess.
/// Returns the wager PDA.
#[allow(clippy::too_many_arguments)]
fn place_bet(
    backend: &mut QuasarBackend,
    table: &Table,
    player: &Keypair,
    seed: u64,
    guess: u8,
    preimage: &[u8; 32],
) -> Pubkey {
    let bet = bet_of(&table.vault, &player.pubkey(), seed, &table.id);
    backend.register_alias(&bet, "Wager");

    let ix = place_bet_ix(
        table.id,
        player.pubkey(),
        table.house.pubkey(),
        table.vault,
        bet,
        seed,
        STAKE,
        guess,
        commit(preimage),
        PLAYER_ENTROPY,
    );
    let tx = backend.send(&[ix], &[player]);
    println!(
        "\n=== the player places a bet (guess {guess}) ===\n{}",
        tx.pretty_cpi_tree()
    );
    assert!(tx.error.is_none(), "place bet: {:?}", tx.error);
    bet
}

/// The house reveals the preimage and settles in one transaction:
/// `[reveal, resolve_bet]`. resolve_bet introspects the reveal. This is the verb
/// the whole suite exists to exercise.
fn reveal_and_settle(
    backend: &mut QuasarBackend,
    table: &Table,
    player: &Pubkey,
    bet: Pubkey,
    seed: u64,
    preimage: &[u8; 32],
) -> Transaction {
    let reveal = reveal_ix(table.id, table.house.pubkey(), *preimage);
    let resolve = resolve_ix(table.id, table.house.pubkey(), *player, table.vault, bet, seed);
    let tx = backend.send(&[reveal, resolve], &[&table.house]);
    println!(
        "\n=== the house reveals and settles ===\n{}",
        tx.pretty_cpi_tree()
    );
    tx
}

fn lamports(backend: &QuasarBackend, pk: &Pubkey) -> u64 {
    backend.get_account(pk).map(|a| a.lamports).unwrap_or(0)
}

/// Render one scenario's decisive transaction to a crime-scene page and return
/// its index row. The page carries the structured log, sequence diagram, and the
/// authority + ownership graphs the program's own pass/fail cannot show.
fn write_page(
    dir: &str,
    file: &str,
    title: &str,
    intent: &str,
    test_fn: &str,
    tx: &Transaction,
) -> (String, String, String) {
    let outcome = if tx.error.is_none() { "succeeded" } else { "failed" };
    let manifest = env!("CARGO_MANIFEST_DIR");
    std::fs::write(
        format!("{dir}/{file}"),
        render_scenario(manifest, title, intent, "tests/gambling.rs", test_fn, tx),
    )
    .unwrap();
    (file.into(), title.into(), outcome.into())
}

// --- the scenarios ---------------------------------------------------------

#[test]
fn the_house_pays_a_winning_roll() {
    let mut backend = QuasarBackend::new();
    let table = open_table(&mut backend);

    let player = backend.actor("Player", PLAYER_FUNDS);
    let player_start = lamports(&backend, &player.pubkey());

    // A roll the player can beat: guess one above it.
    let (preimage, roll) = preimage_where(|r| r <= 98);
    let guess = roll + 1;
    let seed = 1;

    let bet = place_bet(&mut backend, &table, &player, seed, guess, &preimage);
    let tx = reveal_and_settle(&mut backend, &table, &player.pubkey(), bet, seed, &preimage);
    assert!(tx.error.is_none(), "settle a winner: {:?}", tx.error);

    // payout = stake * (10000 - 150) / (guess - 1) / 100; rent nets out on close.
    let payout = (STAKE as u128 * (10_000 - 150) / (guess as u128 - 1) / 100) as u64;
    assert_eq!(
        lamports(&backend, &player.pubkey()),
        player_start - STAKE + payout,
        "the player collected the payout, net of the stake"
    );
    assert_eq!(lamports(&backend, &bet), 0, "the wager is closed");
}

#[test]
fn the_house_keeps_a_losing_roll() {
    let mut backend = QuasarBackend::new();
    let table = open_table(&mut backend);

    let player = backend.actor("Player", PLAYER_FUNDS);
    let player_start = lamports(&backend, &player.pubkey());
    let vault_start = lamports(&backend, &table.vault);

    // A guess that ties the roll loses; the house wins ties.
    let (preimage, roll) = preimage_where(|r| (1..=99).contains(&r));
    let guess = roll;
    let seed = 2;

    let bet = place_bet(&mut backend, &table, &player, seed, guess, &preimage);
    let tx = reveal_and_settle(&mut backend, &table, &player.pubkey(), bet, seed, &preimage);
    assert!(tx.error.is_none(), "settle a loser: {:?}", tx.error);

    assert_eq!(
        lamports(&backend, &player.pubkey()),
        player_start - STAKE,
        "the player is out only the stake"
    );
    assert_eq!(
        lamports(&backend, &table.vault),
        vault_start + STAKE,
        "the house kept the stake"
    );
    assert_eq!(lamports(&backend, &bet), 0, "the wager is closed");
}

#[test]
fn a_switched_preimage_is_caught() {
    let mut backend = QuasarBackend::new();
    let table = open_table(&mut backend);

    let player = backend.actor("Player", PLAYER_FUNDS);

    // The player commits to one preimage; the house tries to reveal another.
    let (committed, _) = preimage_where(|r| r <= 98);
    let mut switched = committed;
    switched[0] ^= 0xff;
    let seed = 3;

    let bet = place_bet(&mut backend, &table, &player, seed, 50, &committed);
    let tx = reveal_and_settle(&mut backend, &table, &player.pubkey(), bet, seed, &switched);
    println!("settle error: {:?}", tx.error);
    assert!(
        tx.error.is_some(),
        "a preimage that does not open the commitment must be rejected"
    );
    assert!(
        lamports(&backend, &bet) > 0,
        "the wager survives a failed settle (atomic)"
    );
}

#[test]
fn the_house_never_shows() {
    let mut backend = QuasarBackend::new();
    let table = open_table(&mut backend);

    let player = backend.actor("Player", PLAYER_FUNDS);
    let player_start = lamports(&backend, &player.pubkey());

    let (preimage, _) = preimage_where(|r| r <= 98);
    let seed = 4;
    let placed_at = backend.clock().slot;
    let bet = place_bet(&mut backend, &table, &player, seed, 50, &preimage);

    // Time passes; the house never reveals. After the timeout, the player walks.
    backend.warp_to_slot(placed_at + REFUND_TIMEOUT_SLOTS + 1);
    let ix = refund_ix(
        table.id,
        player.pubkey(),
        table.house.pubkey(),
        table.vault,
        bet,
        seed,
    );
    let tx = backend.send(&[ix], &[&player]);
    println!(
        "\n=== the house never shows; the player walks ===\n{}",
        tx.pretty_cpi_tree()
    );
    assert!(tx.error.is_none(), "refund: {:?}", tx.error);

    assert_eq!(
        lamports(&backend, &player.pubkey()),
        player_start,
        "the player is made whole"
    );
    assert_eq!(lamports(&backend, &bet), 0, "the wager is closed");
}

/// Replays each scenario and renders its decisive transaction to a crime-scene
/// page under `report/`, plus an index. The settle pages are the showcase: the
/// sequence diagram and CPI tree show `resolve_bet` reaching back to the `reveal`
/// it introspects, which a boolean `is_ok()` never could.
#[test]
fn deal_the_table_and_report() {
    let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/report");
    std::fs::create_dir_all(dir).unwrap();
    let mut entries: Vec<(String, String, String)> = vec![];

    // The house pays a winning roll: the settle introspects the reveal and pays.
    {
        let mut b = QuasarBackend::new();
        let table = open_table(&mut b);
        let player = b.actor("Player", PLAYER_FUNDS);
        let (preimage, roll) = preimage_where(|r| r <= 98);
        let bet = place_bet(&mut b, &table, &player, 1, roll + 1, &preimage);
        let tx = reveal_and_settle(&mut b, &table, &player.pubkey(), bet, 1, &preimage);
        entries.push(write_page(
            dir,
            "winning-roll.md",
            "The house pays a winning roll",
            "A player beats the roll. The house reveals the preimage and settles in one \
             transaction; `resolve_bet` introspects the preceding `reveal` and pays out.",
            "the_house_pays_a_winning_roll",
            &tx,
        ));
    }

    // The house keeps a losing roll: a tie loses; the stake stays.
    {
        let mut b = QuasarBackend::new();
        let table = open_table(&mut b);
        let player = b.actor("Player", PLAYER_FUNDS);
        let (preimage, roll) = preimage_where(|r| (1..=99).contains(&r));
        let bet = place_bet(&mut b, &table, &player, 2, roll, &preimage);
        let tx = reveal_and_settle(&mut b, &table, &player.pubkey(), bet, 2, &preimage);
        entries.push(write_page(
            dir,
            "losing-roll.md",
            "The house keeps a losing roll",
            "A guess that ties the roll loses. The settle introspects the reveal, finds no \
             win, and the stake stays with the house.",
            "the_house_keeps_a_losing_roll",
            &tx,
        ));
    }

    // A switched preimage is caught: the commitment does not open.
    {
        let mut b = QuasarBackend::new();
        let table = open_table(&mut b);
        let player = b.actor("Player", PLAYER_FUNDS);
        let (committed, _) = preimage_where(|r| r <= 98);
        let mut switched = committed;
        switched[0] ^= 0xff;
        let bet = place_bet(&mut b, &table, &player, 3, 50, &committed);
        let tx = reveal_and_settle(&mut b, &table, &player.pubkey(), bet, 3, &switched);
        entries.push(write_page(
            dir,
            "switched-preimage.md",
            "A switched preimage is caught",
            "The house reveals a preimage that does not open the commitment. `resolve_bet` \
             recomputes sha256 and rejects the settle; the wager survives.",
            "a_switched_preimage_is_caught",
            &tx,
        ));
    }

    // The house never shows: the player reclaims after the timeout.
    {
        let mut b = QuasarBackend::new();
        let table = open_table(&mut b);
        let player = b.actor("Player", PLAYER_FUNDS);
        let (preimage, _) = preimage_where(|r| r <= 98);
        let placed_at = b.clock().slot;
        let bet = place_bet(&mut b, &table, &player, 4, 50, &preimage);
        b.warp_to_slot(placed_at + REFUND_TIMEOUT_SLOTS + 1);
        let ix = refund_ix(
            table.id,
            player.pubkey(),
            table.house.pubkey(),
            table.vault,
            bet,
            4,
        );
        let tx = b.send(&[ix], &[&player]);
        entries.push(write_page(
            dir,
            "refund.md",
            "The house never shows",
            "The house never reveals. After the timeout the player reclaims the stake and the \
             wager closes.",
            "the_house_never_shows",
            &tx,
        ));
    }

    let index = render_index(
        "Quasar dice: the table, dealt",
        "Every scenario the dice game plays through the testsvm-quasar engine. Each page carries \
         the structured execution log, a sequence diagram, and the authority + ownership graphs \
         that the program's own pass/fail assertion cannot show.",
        &entries,
    );
    std::fs::write(format!("{dir}/index.md"), index).unwrap();
    println!("wrote {} scenario pages + index.md to {dir}", entries.len());
}
