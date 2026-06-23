//! Generate / check / explain the dice execution report. The CLI dispatch lives
//! in the framework (`testsvm::report::run_cli`); this only supplies the
//! configured `Reporter` (the record corpus + group order) and the heading/intro.
//! It is an example, not a `[[bin]]`, so it sees the dev-dependency
//! `testsvm-quasar` whose graph unifies the solana feature `testsvm` needs to
//! build (a bin's graph would not).
//!
//! Run it after the scenario records have been written:
//!   cargo test                          # `deal_the_table_and_report` writes target/test-results/*.json + report/*.md
//!   cargo run --example report          # folds them: index.md + fingerprint.txt + baseline.tar.gz
//!   cargo run --example report -- --check    # CI gate: diff vs the committed fingerprint (exit 1)
//!   cargo run --example report -- --explain  # what changed vs the committed baseline.tar.gz

use testsvm::report::{run_cli, Reporter};

const HEADING: &str = "Quasar dice: the table, dealt";
const INTRO: &str = "Every scenario the dice game plays through the testsvm-quasar engine, folded \
from the per-test records. Each row links back to the test that produced it; each page carries the \
structured execution log, a sequence diagram, and the authority + ownership graphs the program's \
own pass/fail assertion cannot show. The fingerprint below is the behavioral signature, byte-stable \
across runs (seeded actors), so it doubles as the regression and mutant-kill oracle.";

fn main() {
    let m = env!("CARGO_MANIFEST_DIR");
    let reporter = Reporter::from_dir(&format!("{m}/target/test-results")).group_order(&["Dice"]);
    run_cli(&reporter, &format!("{m}/report"), HEADING, INTRO);
}
