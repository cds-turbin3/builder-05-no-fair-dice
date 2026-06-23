# A close-and-reopen grind is caught

**Intent.** Once a player has bet, the table is claimed. The house's attempt to close it (to reopen with a substituted commitment after seeing the entropy) is rejected with `TableInUse`.

**Outcome.** The transaction failed: `custom program error: 0x8`.

**Source.** [`tests/gambling.rs::a_close_and_reopen_grind_is_caught`](../tests/gambling.rs#L442)

## Structured execution log

```
CPI Tree (347 BPF CU / 1,400,000 budget):
└── close_table FAILED: TableInUse (0x8) (347 / 1,400,000 CU) dice (no CPIs)
```

## Sequence diagram

```mermaid
sequenceDiagram
    autonumber
    participant House
    participant dice
    House ->> dice: close_table (347cu)
    rect rgb(255, 220, 220)
    note over dice: ✗ TableInUse (0x8)
    end
```

## Authority graph

Who signed for what; an `invoke_signed` PDA appears as its own authority.

```mermaid
flowchart LR
    classDef signer fill:#d4edda,stroke:#28a745;
    classDef program fill:#cce5ff,stroke:#007bff;
    classDef writable fill:#fff3cd,stroke:#ffc107;
    dice[dice]:::program
    House([House]):::signer
    Table[(Table)]:::writable
    House -->|signs| dice
    dice -->|writes| Table
```

## Ownership graph

Which program owns each account the transaction wrote.

```mermaid
flowchart LR
    classDef owner fill:#cce5ff,stroke:#007bff;
    classDef account fill:#fff3cd,stroke:#ffc107;
    System[System]:::owner
    House[(House)]:::account
    dice[dice]:::owner
    Table[(Table)]:::account
    System -->|owns| House
    dice -->|owns| Table
```
