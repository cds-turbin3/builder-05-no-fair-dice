//! The dice table, dealt through the testsvm-quasar engine.
//!
//! The dice game's one genuinely hard primitive is instruction introspection:
//! `resolve_bet` reads the preceding `reveal` out of the Instructions sysvar to
//! recover the preimage and confirm the house signed it. So the settle verb is a
//! single transaction carrying TWO instructions, `[reveal, resolve_bet]`, and the
//! whole point of this suite is to prove that parse works on a live engine.
//!
//! The commitment is on-chain: the house `opens the table` (funds the vault and
//! posts `sha256(preimage)`), then a player `places a bet` against that open
//! table, the house `reveals and settles`, or the player `walks` after a timeout.
//! The roll is derived host-side too, so each scenario picks a guess meant to win
//! or lose and asserts the table paid out accordingly.

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

/// `sha256(preimage)`, the commitment the house posts when it opens a table.
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
fn table_of(house: &Pubkey, table_seed: u64, id: &Pubkey) -> Pubkey {
    Pubkey::find_program_address(
        &[b"table", house.as_ref(), table_seed.to_le_bytes().as_ref()],
        id,
    )
    .0
}
fn bet_of(house: &Pubkey, table_seed: u64, player: &Pubkey, id: &Pubkey) -> Pubkey {
    Pubkey::find_program_address(
        &[
            b"bet",
            house.as_ref(),
            table_seed.to_le_bytes().as_ref(),
            player.as_ref(),
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

fn open_table_ix(
    id: Pubkey,
    house: Pubkey,
    table: Pubkey,
    table_seed: u64,
    commitment: [u8; 32],
) -> Instruction {
    let mut data = vec![5u8];
    data.extend_from_slice(&table_seed.to_le_bytes());
    data.extend_from_slice(&commitment);
    Instruction {
        program_id: id,
        accounts: vec![
            AccountMeta::new(house, true),
            AccountMeta::new(table, false),
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
    table: Pubkey,
    bet: Pubkey,
    table_seed: u64,
    amount: u64,
    guess: u8,
    entropy: [u8; 32],
) -> Instruction {
    let mut data = vec![1u8];
    data.extend_from_slice(&table_seed.to_le_bytes());
    data.extend_from_slice(&amount.to_le_bytes());
    data.push(guess);
    data.extend_from_slice(&entropy);
    Instruction {
        program_id: id,
        accounts: vec![
            AccountMeta::new(player, true),
            AccountMeta::new_readonly(house, false),
            AccountMeta::new(vault, false),
            AccountMeta::new(table, false),
            AccountMeta::new(bet, false),
            AccountMeta::new_readonly(system_program(), false),
        ],
        data,
    }
}

fn close_table_ix(id: Pubkey, house: Pubkey, table: Pubkey, table_seed: u64) -> Instruction {
    let mut data = vec![6u8];
    data.extend_from_slice(&table_seed.to_le_bytes());
    Instruction {
        program_id: id,
        accounts: vec![
            AccountMeta::new(house, true),
            AccountMeta::new(table, false),
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

#[allow(clippy::too_many_arguments)]
fn resolve_ix(
    id: Pubkey,
    house: Pubkey,
    player: Pubkey,
    vault: Pubkey,
    table: Pubkey,
    bet: Pubkey,
    table_seed: u64,
) -> Instruction {
    let mut data = vec![3u8];
    data.extend_from_slice(&table_seed.to_le_bytes());
    Instruction {
        program_id: id,
        accounts: vec![
            AccountMeta::new(house, true),
            AccountMeta::new(player, false),
            AccountMeta::new(vault, false),
            AccountMeta::new(table, false),
            AccountMeta::new(bet, false),
            AccountMeta::new_readonly(instructions_sysvar(), false),
            AccountMeta::new_readonly(system_program(), false),
        ],
        data,
    }
}

#[allow(clippy::too_many_arguments)]
fn refund_ix(
    id: Pubkey,
    player: Pubkey,
    house: Pubkey,
    vault: Pubkey,
    table: Pubkey,
    bet: Pubkey,
    table_seed: u64,
) -> Instruction {
    let mut data = vec![4u8];
    data.extend_from_slice(&table_seed.to_le_bytes());
    Instruction {
        program_id: id,
        accounts: vec![
            AccountMeta::new(player, true),
            AccountMeta::new(house, false),
            AccountMeta::new(vault, false),
            AccountMeta::new(table, false),
            AccountMeta::new(bet, false),
            AccountMeta::new_readonly(system_program(), false),
        ],
        data,
    }
}

// --- the gambling DSL ------------------------------------------------------

/// An open round: a funded house with a posted commitment, ready to take a bet.
struct Table {
    id: Pubkey,
    house: Keypair,
    vault: Pubkey,
    seed: u64,
    pubkey: Pubkey,
    /// The house's secret, revealed at settle. `commit(preimage)` is what the
    /// table posted on-chain.
    preimage: [u8; 32],
}

/// The house funds its vault and opens a table by posting `sha256(preimage)`
/// on-chain — the commit, before any bet exists.
fn open_table(backend: &mut QuasarBackend, table_seed: u64, preimage: [u8; 32]) -> Table {
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
        (5, "open_table"),
        (6, "close_table"),
    ] {
        backend.register_instruction_name(&id, &[disc], name);
    }
    for (code, name) in [
        (3u32, "BadSignature"),
        (4, "CommitRevealMismatch"),
        (5, "Overflow"),
        (6, "TimeoutNotReached"),
        (7, "NotReveal"),
        (8, "TableInUse"),
    ] {
        backend.register_error_name(&id, code, name);
    }

    let house = backend.actor("House", BANKROLL + 1_000_000_000);
    let vault = vault_of(&house.pubkey(), &id);
    backend.register_alias(&vault, "Vault");

    let init = backend.send(&[initialize_ix(id, house.pubkey(), vault, BANKROLL)], &[&house]);
    assert!(init.error.is_none(), "fund vault: {:?}", init.error);

    let table = table_of(&house.pubkey(), table_seed, &id);
    backend.register_alias(&table, "Table");
    let tx = backend.send(
        &[open_table_ix(id, house.pubkey(), table, table_seed, commit(&preimage))],
        &[&house],
    );
    println!(
        "\n=== the house opens the table (commits sha256(preimage)) ===\n{}",
        tx.pretty_cpi_tree()
    );
    assert!(tx.error.is_none(), "open table: {:?}", tx.error);

    Table {
        id,
        house,
        vault,
        seed: table_seed,
        pubkey: table,
        preimage,
    }
}

/// A player bets against the open table with a chosen guess.
fn place_bet(backend: &mut QuasarBackend, table: &Table, player: &Keypair, guess: u8) -> Pubkey {
    let bet = bet_of(&table.house.pubkey(), table.seed, &player.pubkey(), &table.id);
    backend.register_alias(&bet, "Wager");

    let ix = place_bet_ix(
        table.id,
        player.pubkey(),
        table.house.pubkey(),
        table.vault,
        table.pubkey,
        bet,
        table.seed,
        STAKE,
        guess,
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

/// The house reveals `reveal_preimage` and settles in one transaction:
/// `[reveal, resolve_bet]`. resolve_bet introspects the reveal. Pass
/// `table.preimage` for an honest settle, or a different value to forge one.
fn reveal_and_settle(
    backend: &mut QuasarBackend,
    table: &Table,
    player: &Pubkey,
    bet: Pubkey,
    reveal_preimage: &[u8; 32],
) -> Transaction {
    let reveal = reveal_ix(table.id, table.house.pubkey(), *reveal_preimage);
    let resolve = resolve_ix(
        table.id,
        table.house.pubkey(),
        *player,
        table.vault,
        table.pubkey,
        bet,
        table.seed,
    );
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
/// its index row.
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
    let (preimage, roll) = preimage_where(|r| r <= 98);
    let mut backend = QuasarBackend::new();
    let table = open_table(&mut backend, 1, preimage);

    let player = backend.actor("Player", PLAYER_FUNDS);
    let player_start = lamports(&backend, &player.pubkey());

    let guess = roll + 1; // beat the roll
    let bet = place_bet(&mut backend, &table, &player, guess);
    let tx = reveal_and_settle(&mut backend, &table, &player.pubkey(), bet, &table.preimage);
    assert!(tx.error.is_none(), "settle a winner: {:?}", tx.error);

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
    let (preimage, roll) = preimage_where(|r| (1..=99).contains(&r));
    let mut backend = QuasarBackend::new();
    let table = open_table(&mut backend, 2, preimage);

    let player = backend.actor("Player", PLAYER_FUNDS);
    let player_start = lamports(&backend, &player.pubkey());
    let vault_start = lamports(&backend, &table.vault);

    let guess = roll; // a tie loses; the house wins ties
    let bet = place_bet(&mut backend, &table, &player, guess);
    let tx = reveal_and_settle(&mut backend, &table, &player.pubkey(), bet, &table.preimage);
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
    let (preimage, _) = preimage_where(|r| r <= 98);
    let mut backend = QuasarBackend::new();
    let table = open_table(&mut backend, 3, preimage);

    let player = backend.actor("Player", PLAYER_FUNDS);
    let bet = place_bet(&mut backend, &table, &player, 50);

    // The house tries to reveal a preimage that does not open the commitment.
    let mut switched = table.preimage;
    switched[0] ^= 0xff;
    let tx = reveal_and_settle(&mut backend, &table, &player.pubkey(), bet, &switched);
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
    let (preimage, _) = preimage_where(|r| r <= 98);
    let mut backend = QuasarBackend::new();
    let table = open_table(&mut backend, 4, preimage);

    let player = backend.actor("Player", PLAYER_FUNDS);
    let player_start = lamports(&backend, &player.pubkey());

    let placed_at = backend.clock().slot;
    let bet = place_bet(&mut backend, &table, &player, 50);

    // Time passes; the house never reveals. After the timeout, the player walks.
    backend.warp_to_slot(placed_at + REFUND_TIMEOUT_SLOTS + 1);
    let ix = refund_ix(
        table.id,
        player.pubkey(),
        table.house.pubkey(),
        table.vault,
        table.pubkey,
        bet,
        table.seed,
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

#[test]
fn the_house_closes_an_empty_table() {
    // A table opened but never bet against: the house reclaims its rent.
    let (preimage, _) = preimage_where(|r| r <= 98);
    let mut backend = QuasarBackend::new();
    let table = open_table(&mut backend, 7, preimage);
    let house_before = lamports(&backend, &table.house.pubkey());

    let ix = close_table_ix(table.id, table.house.pubkey(), table.pubkey, table.seed);
    let tx = backend.send(&[ix], &[&table.house]);
    println!(
        "\n=== the house closes an empty table ===\n{}",
        tx.pretty_cpi_tree()
    );
    assert!(tx.error.is_none(), "close an empty table: {:?}", tx.error);
    assert_eq!(lamports(&backend, &table.pubkey), 0, "the table is closed");
    assert!(
        lamports(&backend, &table.house.pubkey()) > house_before,
        "the table's rent returned to the house"
    );
}

#[test]
fn a_close_and_reopen_grind_is_caught() {
    // Once a player has bet, the house must not be able to close the table and
    // reopen it with a commitment ground against the now-visible entropy. The
    // claim guard rejects the close, so that substitution is impossible.
    let (preimage, _) = preimage_where(|r| r <= 98);
    let mut backend = QuasarBackend::new();
    let table = open_table(&mut backend, 8, preimage);

    let player = backend.actor("Player", PLAYER_FUNDS);
    let bet = place_bet(&mut backend, &table, &player, 50); // claims the table

    let ix = close_table_ix(table.id, table.house.pubkey(), table.pubkey, table.seed);
    let tx = backend.send(&[ix], &[&table.house]);
    println!("close error: {:?}", tx.error);
    assert!(
        tx.error.is_some(),
        "a claimed table cannot be closed (TableInUse), so the house cannot reopen to grind"
    );
    assert!(
        lamports(&backend, &table.pubkey) > 0,
        "the table survives the rejected close"
    );
    assert!(lamports(&backend, &bet) > 0, "the wager survives");
}

/// Replays each scenario and renders its decisive transaction to a crime-scene
/// page under `report/`, plus an index.
#[test]
fn deal_the_table_and_report() {
    let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/report");
    std::fs::create_dir_all(dir).unwrap();
    let mut entries: Vec<(String, String, String)> = vec![];

    // The house pays a winning roll.
    {
        let (preimage, roll) = preimage_where(|r| r <= 98);
        let mut b = QuasarBackend::new();
        let table = open_table(&mut b, 1, preimage);
        let player = b.actor("Player", PLAYER_FUNDS);
        let bet = place_bet(&mut b, &table, &player, roll + 1);
        let tx = reveal_and_settle(&mut b, &table, &player.pubkey(), bet, &table.preimage);
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

    // The house keeps a losing roll.
    {
        let (preimage, roll) = preimage_where(|r| (1..=99).contains(&r));
        let mut b = QuasarBackend::new();
        let table = open_table(&mut b, 2, preimage);
        let player = b.actor("Player", PLAYER_FUNDS);
        let bet = place_bet(&mut b, &table, &player, roll);
        let tx = reveal_and_settle(&mut b, &table, &player.pubkey(), bet, &table.preimage);
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

    // A switched preimage is caught.
    {
        let (preimage, _) = preimage_where(|r| r <= 98);
        let mut b = QuasarBackend::new();
        let table = open_table(&mut b, 3, preimage);
        let player = b.actor("Player", PLAYER_FUNDS);
        let bet = place_bet(&mut b, &table, &player, 50);
        let mut switched = table.preimage;
        switched[0] ^= 0xff;
        let tx = reveal_and_settle(&mut b, &table, &player.pubkey(), bet, &switched);
        entries.push(write_page(
            dir,
            "switched-preimage.md",
            "A switched preimage is caught",
            "The house reveals a preimage that does not open the table's commitment. \
             `resolve_bet` recomputes sha256 and rejects the settle; the wager survives.",
            "a_switched_preimage_is_caught",
            &tx,
        ));
    }

    // The house never shows.
    {
        let (preimage, _) = preimage_where(|r| r <= 98);
        let mut b = QuasarBackend::new();
        let table = open_table(&mut b, 4, preimage);
        let player = b.actor("Player", PLAYER_FUNDS);
        let placed_at = b.clock().slot;
        let bet = place_bet(&mut b, &table, &player, 50);
        b.warp_to_slot(placed_at + REFUND_TIMEOUT_SLOTS + 1);
        let ix = refund_ix(
            table.id,
            player.pubkey(),
            table.house.pubkey(),
            table.vault,
            table.pubkey,
            bet,
            table.seed,
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

    // The house closes an empty table.
    {
        let (preimage, _) = preimage_where(|r| r <= 98);
        let mut b = QuasarBackend::new();
        let table = open_table(&mut b, 7, preimage);
        let ix = close_table_ix(table.id, table.house.pubkey(), table.pubkey, table.seed);
        let tx = b.send(&[ix], &[&table.house]);
        entries.push(write_page(
            dir,
            "close-empty-table.md",
            "The house closes an empty table",
            "A table opened but never bet against. The house reclaims its rent with `close_table`.",
            "the_house_closes_an_empty_table",
            &tx,
        ));
    }

    // A close-and-reopen grind is caught.
    {
        let (preimage, _) = preimage_where(|r| r <= 98);
        let mut b = QuasarBackend::new();
        let table = open_table(&mut b, 8, preimage);
        let player = b.actor("Player", PLAYER_FUNDS);
        let _bet = place_bet(&mut b, &table, &player, 50);
        let ix = close_table_ix(table.id, table.house.pubkey(), table.pubkey, table.seed);
        let tx = b.send(&[ix], &[&table.house]);
        entries.push(write_page(
            dir,
            "grind-caught.md",
            "A close-and-reopen grind is caught",
            "Once a player has bet, the table is claimed. The house's attempt to close it (to \
             reopen with a substituted commitment after seeing the entropy) is rejected with \
             `TableInUse`.",
            "a_close_and_reopen_grind_is_caught",
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
