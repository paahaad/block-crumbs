-- 1. Fee Landscape (per slot)
SELECT 
    slot,
    sum(fee) / 1e9 as total_fees_sol,
    sum(base_fee) / 1e9 as base_fees_sol,
    sum(priority_fee) / 1e9 as priority_fees_sol,
    sum(jito_tip_amount) / 1e9 as jito_tips_sol
FROM transactions
GROUP BY slot
ORDER BY slot DESC
LIMIT 10;

-- 2. Jito Status (per slot)
SELECT 
    slot,
    countIf(is_jito_bundle = 1) as jito_txs,
    countIf(is_jito_bundle = 0) as non_jito_txs,
    round(countIf(is_jito_bundle = 1) * 100.0 / count(), 2) as jito_pct
FROM transactions
GROUP BY slot
ORDER BY slot DESC
LIMIT 10;

-- 3. CU Utilization (per slot)
SELECT 
    slot,
    sum(cu_consumed) as total_cu_used,
    sum(cu_requested) as total_cu_requested,
    round(sum(cu_consumed) / sum(cu_requested) * 100, 1) as utilization_pct
FROM transactions
WHERE slot >= (SELECT max(slot) - 10 FROM transactions)
GROUP BY slot
ORDER BY slot;

-- 4. Block Summary
SELECT 
    slot,
    total_tx,
    successful_tx,
    jito_bundle_count,
    round(total_fees / 1e9, 4) as fees_sol
FROM block_stats
ORDER BY slot DESC
LIMIT 10;
