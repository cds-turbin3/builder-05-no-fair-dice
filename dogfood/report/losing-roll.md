# The house keeps a losing roll

**Intent.** A guess that ties the roll loses. The settle introspects the reveal, finds no win, and the stake stays with the house.

**Outcome.** The transaction succeeded.

**Source.** [`tests/gambling.rs::the_house_keeps_a_losing_roll`](../tests/gambling.rs#L420)

## Structured execution log

```
CPI Tree (1,896 BPF CU / 1,400,000 budget):
├── reveal (42 / 1,400,000 CU) dice (no CPIs)
└── resolve_bet (1,854 / 1,399,958 CU) dice (no CPIs)
```

## Sequence diagram

```mermaid
sequenceDiagram
    autonumber
    participant House
    participant dice
    House ->> dice: reveal (42cu)
    House ->> dice: resolve_bet (1854cu)
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
    Vault[(Vault)]:::writable
    Table[(Table)]:::writable
    Wager[(Wager)]:::writable
    House -->|signs| dice
    dice -->|writes| Player
    dice -->|writes| Vault
    dice -->|writes| Table
    dice -->|writes| Wager
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
