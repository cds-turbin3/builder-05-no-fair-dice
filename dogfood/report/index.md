# Quasar dice: the table, dealt

Every scenario the dice game plays through the testsvm-quasar engine, folded from the per-test records. Each row links back to the test that produced it; each page carries the structured execution log, a sequence diagram, and the authority + ownership graphs the program's own pass/fail assertion cannot show. The fingerprint below is the behavioral signature, byte-stable across runs (seeded actors), so it doubles as the regression and mutant-kill oracle.

**6 scenarios.**

## Dice

| Scenario | Verdict | Summary | Source |
|---|---|---|---|
| The house pays a winning roll | ✅ passed | succeeded | [tests/gambling.rs::the_house_pays_a_winning_roll](../tests/gambling.rs#L302-L322) |
| The house keeps a losing roll | ✅ passed | succeeded | [tests/gambling.rs::the_house_keeps_a_losing_roll](../tests/gambling.rs#L325-L350) |
| A switched preimage is caught | ✅ passed | rejected: custom program error: 0x4 | [tests/gambling.rs::a_switched_preimage_is_caught](../tests/gambling.rs#L353-L374) |
| The house never shows | ✅ passed | succeeded | [tests/gambling.rs::the_house_never_shows](../tests/gambling.rs#L377-L412) |
| The house closes an empty table | ✅ passed | succeeded | [tests/gambling.rs::the_house_closes_an_empty_table](../tests/gambling.rs#L415-L439) |
| A close-and-reopen grind is caught | ✅ passed | rejected: custom program error: 0x8 | [tests/gambling.rs::a_close_and_reopen_grind_is_caught](../tests/gambling.rs#L442-L470) |
