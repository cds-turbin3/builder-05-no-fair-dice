# The house closes an empty table

**Intent.** A table opened but never bet against. The house reclaims its rent with `close_table`.

**Outcome.** The transaction succeeded.

**Source.** [`tests/gambling.rs::the_house_closes_an_empty_table`](../tests/gambling.rs#L404)

## Structured execution log

```
CPI Tree (371 BPF CU / 1,400,000 budget):
└── close_table (371 / 1,400,000 CU) dice (no CPIs)
```

## Sequence diagram

```mermaid
sequenceDiagram
    autonumber
    participant House
    participant dice
    House ->> dice: close_table (371cu)
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
    Table[(Table)]:::account
    System -->|owns| House
    System -->|owns| Table
```
