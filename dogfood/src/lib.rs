//! Dogfood harness for the Quasar dice game: drives `programs/quasar-dice`
//! through the testsvm-quasar engine. The scenarios live in `tests/gambling.rs`,
//! played as a gambling table (the house opens, a player bets, the house reveals
//! and settles) so the run reads as the game it models.

// The instruction client, generated at build time by `build.rs` straight from
// the program source (`../programs/quasar-dice/src`) and written to OUT_DIR
// (regenerated whenever the program changes, so it cannot drift). One struct per
// instruction; `ix()` builds the `Instruction`, `alias_all` names the accounts.
// The `#[allow]` rides the include because the generated file carries no inner
// attributes of its own (so it stays droppable into a module).
#[allow(dead_code, unused_imports)]
pub mod client {
    include!(concat!(env!("OUT_DIR"), "/generated_client.rs"));
}
