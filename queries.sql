-- 1. Fee Landscape
-- Breakdown of total fees, base fees, priority fees, and Jito tips per slot (in SOL)
SELECT 
    slot,
    total_fees / 1e9 as total_fees_sol,
    (total_fees - total_priority_fees - total_jito_tips) / 1e9 as base_fees_sol,
    total_priority_fees / 1e9 as priority_fees_sol,
    total_jito_tips / 1e9 as jito_tips_sol
FROM block_stats
ORDER BY slot DESC
LIMIT 10;

-- 2. Jito Status
-- Count of Jito vs non-Jito transactions and Jito bundle percentage per slot
SELECT 
    slot,
    jito_bundle_count as jito_txs,
    total_tx - jito_bundle_count as non_jito_txs,
    round(jito_bundle_count * 100.0 / total_tx, 2) as jito_pct
FROM block_stats
ORDER BY slot DESC
LIMIT 10;

-- 3. CU Utilization
-- Compute units consumed vs requested per slot to measure CU efficiency
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
-- High-level block stats including tx counts, Jito bundles, and total fees
SELECT 
    slot,
    total_tx,
    successful_tx,
    jito_bundle_count,
    round(total_fees / 1e9, 4) as fees_sol
FROM block_stats
ORDER BY slot DESC
LIMIT 10;
