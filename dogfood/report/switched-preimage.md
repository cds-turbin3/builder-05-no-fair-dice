# A switched preimage is caught

**Intent.** The house reveals a preimage that does not open the commitment. `resolve_bet` recomputes sha256 and rejects the settle; the wager survives.

**Outcome.** The transaction failed: `custom program error: 0x4`.

**Source.** [`tests/gambling.rs::a_switched_preimage_is_caught`](../tests/gambling.rs#L411)

## Structured execution log

```
CPI Tree (1,145 BPF CU / 1,400,000 budget):
├── reveal (35 / 1,400,000 CU) dice (no CPIs)
└── resolve_bet FAILED: CommitRevealMismatch (0x4) (1,110 / 1,399,965 CU) dice (no CPIs)
```

## Sequence diagram

```mermaid
sequenceDiagram
    autonumber
    participant House
    participant dice
    House ->> dice: reveal (35cu)
    House ->> dice: resolve_bet (1110cu)
    rect rgb(255, 220, 220)
    note over dice: ✗ CommitRevealMismatch (0x4)
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
    Player[(Player)]:::writable
    Vault[(Vault)]:::writable
    Wager[(Wager)]:::writable
    House -->|signs| dice
    dice -->|writes| Player
    dice -->|writes| Vault
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
    dice[dice]:::owner
    Wager[(Wager)]:::account
    System -->|owns| House
    System -->|owns| Player
    System -->|owns| Vault
    dice -->|owns| Wager
```
