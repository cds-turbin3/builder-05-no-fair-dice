# The house pays a winning roll

**Intent.** A player beats the roll. The house reveals the preimage and settles in one transaction; `resolve_bet` introspects the preceding `reveal` and pays out.

**Outcome.** The transaction succeeded.

**Source.** [`tests/gambling.rs::the_house_pays_a_winning_roll`](../tests/gambling.rs#L416)

## Structured execution log

```
CPI Tree (3,485 BPF CU / 1,400,000 budget):
├── reveal (43 / 1,400,000 CU) dice (no CPIs)
└── resolve_bet (3,442 / 1,399,957 CU) dice
    └── System
```

## Sequence diagram

```mermaid
sequenceDiagram
    autonumber
    participant House
    participant dice
    participant System
    House ->> dice: reveal (43cu)
    House ->> dice: resolve_bet (3442cu)
    dice ->> System: Transfer
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
    Player[(Player)]:::writable
    Vault([Vault]):::signer
    Table[(Table)]:::writable
    Wager[(Wager)]:::writable
    System[System]:::program
    House -->|signs| dice
    Vault -->|signs| System
    dice -->|writes| Player
    dice -->|writes| Vault
    dice -->|writes| Table
    dice -->|writes| Wager
    System -->|writes| Player
```

## Ownership graph

Which program owns each account the transaction wrote.

```mermaid
flowchart LR
    classDef owner fill:#cce5ff,stroke:#007bff;
    classDef account fill:#fff3cd,stroke:#ffc107;
    System[System]:::owner
    House[(House)]:::account
    Player[(Player)]:::account
    Vault[(Vault)]:::account
    Table[(Table)]:::account
    Wager[(Wager)]:::account
    System -->|owns| House
    System -->|owns| Player
    System -->|owns| Vault
    System -->|owns| Table
    System -->|owns| Wager
```
