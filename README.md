## Task
Your task is to build a lightweight system that ingests Solana block data and makes it queryable for research purposes.

## Approach
I used the **Carbon indexer framework** to handle the data pipeline, letting me focus on the core logic. With `PassthroughDecoder`, I receive raw transactions and manually extract all fields in [`TransactionAnalyzer`](src/processor/transaction_analyzer.rs).


## Quick Start

**1. Start ClickHouse** (config in `fs/volumes/`)
```bash
docker compose up -d
```

**2. Run the indexer**
```bash
cp env.example .env  # add your Helius API key
cargo run
```


## Schema Design

**Why two tables?**
- **Transactions** — granular per-tx data for detailed analysis (fee breakdown, Jito detection, cu limit, tx omplexity )
- **Block Stats** — pre-aggregated block-level metrics to avoid expensive `GROUP BY slot` queries at read time

**1. Transactions Table**
```sql
CREATE TABLE IF NOT EXISTS transactions (
    slot UInt64,
    block_time DateTime64(3) DEFAULT 0,
    tx_index UInt64,
    signature String,
    fee_payer String,
    success UInt8,
    fee UInt64,
    base_fee UInt64,
    priority_fee UInt64,
    cu_price UInt64,
    cu_consumed UInt64,
    cu_requested UInt32,
    is_jito_bundle UInt8,
    jito_tip_amount UInt64,
    jito_tip_account String,
    landing_method LowCardinality(String),
    num_instructions UInt32,
    num_accounts UInt32,
    num_signers UInt32,
)
ENGINE = MergeTree()
PARTITION BY toYYYYMMDD(toDateTime(block_time))
ORDER BY (slot, tx_index)
```
- **Daily partitions** — enables efficient time-range queries via partition pruning
- **Order by (slot, tx_index)** — data is stored in the same order as transactions appear in blocks, so queries like "get all txs in slot X" read contiguous data instead of jumping around disk

**2. Block Stats Table**
```sql
CREATE TABLE IF NOT EXISTS block_stats (
    slot UInt64,
    block_time DateTime64(3) DEFAULT 0,
    total_tx UInt32,
    successful_tx UInt32,
    failed_tx UInt32,
    jito_bundle_count UInt32,
    priority_fee_count UInt32,
    base_fee_count UInt32,
    total_fees UInt64,
    total_priority_fees UInt64,
    total_jito_tips UInt64,
    total_cu UInt64,
    median_priority_fee UInt64,
    max_priority_fee UInt64,
    median_cu_price UInt64,
)
ENGINE = MergeTree()
PARTITION BY toYYYYMMDD(toDateTime(block_time))
ORDER BY slot
```
- **Order by slot** — blocks are naturally sequential, so queries like "stats for slots 1000-2000" read data in order


## Signals Extracted

**1. Jito Bundle Detection** — [`jito.rs`](src/decoder/jito.rs)

If a tx transfers SOL to any of the [8 Jito tip accounts](src/decoder/jito.rs#L7-L21), it's a Jito bundle. Also extract the tip amount.

**2. Priority Fee Calculation** — [`compute_budget.rs`](src/decoder/compute_budget.rs)

Extract `SetComputeUnitPrice` and `SetComputeUnitLimit` from ComputeBudget instructions. Priority fee = `(cu_price × cu_limit) / 1_000_000` — based on requested CU. This also enables analyzing CU efficiency: requested vs actually utilized.

**3. Fee Landscape per Block**

Aggregate fee distribution across each slot: what portion goes to priority fees, Jito tips, and base fees. This reveals how block space is being monetized and the competition dynamics between MEV (Jito) vs priority fee bidding.


## Interesting Finding

**~30-50% of requested CU goes unused** means users are overpaying priority fees.

Since priority fee = `cu_limit × cu_price`, requesting more CU than needed means paying for unused compute. From my data, the avg consumed vs requested ratio is roughly 1:2.

```sql
SELECT 
    slot,
    sum(cu_consumed) as total_cu_used,
    sum(cu_requested) as total_cu_requested,
    round(sum(cu_consumed) / sum(cu_requested) * 100, 1) as utilization_pct
FROM transactions
WHERE slot >= (SELECT max(slot) - 10 FROM transactions)
GROUP BY slot
ORDER BY slot;
```

## What I'd Build Next

Given more time, I'd explore **how validators schedule transactions**, analyzing tx ordering within blocks and how priority fees/Jito tips actually affect position. This requires understanding both the validator's scheduler and Jito's bundle processing.

## Example Queries
See [`queries.sql`](queries.sql)