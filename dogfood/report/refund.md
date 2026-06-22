# The house never shows

**Intent.** The house never reveals. After the timeout the player reclaims the stake and the wager closes.

**Outcome.** The transaction succeeded.

**Source.** [`tests/gambling.rs::the_house_never_shows`](../tests/gambling.rs#L366)

## Structured execution log

```
CPI Tree (2,461 BPF CU / 1,400,000 budget):
└── refund_bet (2,461 / 1,400,000 CU) dice
    └── System
```

## Sequence diagram

```mermaid
sequenceDiagram
    autonumber
    participant Player
    participant dice
    participant System
    Player ->> dice: refund_bet (2461cu)
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
    Player([Player]):::signer
    House[(House)]:::writable
    Vault([Vault]):::signer
    Table[(Table)]:::writable
    Wager[(Wager)]:::writable
    System[System]:::program
    Player -->|signs| dice
    Vault -->|signs| System
    dice -->|writes| House
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
    Player[(Player)]:::account
    House[(House)]:::account
    Vault[(Vault)]:::account
    Table[(Table)]:::account
    Wager[(Wager)]:::account
    System -->|owns| Player
    System -->|owns| House
    System -->|owns| Vault
    System -->|owns| Table
    System -->|owns| Wager
```
