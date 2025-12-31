use clickhouse::{Client, Row};
use serde::{Serialize, Deserialize};
use crate::db::schema::{BlockStats, TransactionRecord};


pub struct ClickHouseClient {
    client: Client,
    database: String,
    url: String,
}


#[derive(Debug, Clone, Serialize, Deserialize, Row)]
pub struct TransactionRow {
    pub slot: u64,
    pub block_time: i64,
    pub tx_index: u64,
    pub signature: String,
    pub fee_payer: String,
    pub success: u8,
    pub fee: u64,
    pub base_fee: u64,
    pub priority_fee: u64,
    pub cu_price: u64,
    pub cu_consumed: u64,
    pub cu_requested: u32,
    pub is_jito_bundle: u8,
    pub jito_tip_account: String,
    pub jito_tip_amount: u64,
    pub landing_method: String,
    pub num_instructions: u32,
    pub num_accounts: u32,
    pub num_signers: u32, 
}

impl From<TransactionRecord> for TransactionRow {
    fn from(value: TransactionRecord) -> Self {
        Self { 
            slot: value.slot, 
            // Convert seconds to milliseconds for DateTime64(3)
            block_time: value.block_time.unwrap_or(0) * 1000, 
            tx_index: value.tx_index.unwrap_or(0), 
            signature: value.signature, 
            fee_payer: value.fee_payer, 
            success: if value.success { 1 } else { 0 }, 
            fee: value.fee, 
            base_fee: value.base_fee, 
            priority_fee: value.priority_fee, 
            cu_price: value.cu_price.unwrap_or(0), 
            cu_consumed: value.cu_consumed, 
            cu_requested: value.cu_requested.unwrap_or(0), 
            is_jito_bundle: if value.is_jito_bundle { 1 } else { 0 }, 
            jito_tip_account: value.jito_tip_account.unwrap_or_default(), 
            jito_tip_amount: value.jito_tip_amount.unwrap_or(0), 
            landing_method: value.landing_method, 
            num_instructions: value.num_instructions, 
            num_accounts: value.num_accounts, 
            num_signers: value.num_signers
        }
    }
}


#[derive(Debug, Clone, Serialize, Deserialize, Row)]
pub struct BlockStatsRow {
    pub slot: u64,
    pub block_time: i64,    
    pub total_tx: u32,
    pub successful_tx: u32,
    pub failed_tx: u32,
    pub jito_bundle_count: u32,
    pub priority_fee_count: u32,
    pub base_fee_count: u32,
    pub total_fees: u64,
    pub total_priority_fees: u64,
    pub total_jito_tips: u64,
    pub total_cu: u64,
    pub median_priority_fee: u64,
    pub max_priority_fee: u64,
    pub median_cu_price: u64,
}

impl From<BlockStats> for BlockStatsRow {
    fn from(value: BlockStats) -> Self {
        Self { 
            slot: value.slot, 
            // Convert seconds to milliseconds for DateTime64(3)
            block_time: value.block_time.unwrap_or(0) * 1000, 
            total_tx: value.total_tx, 
            successful_tx: value.successful_tx, 
            failed_tx: value.failed_tx, 
            jito_bundle_count: value.jito_bundle_count, 
            priority_fee_count: value.priority_fee_count, 
            base_fee_count: value.base_fee_count, 
            total_fees: value.total_fees, 
            total_priority_fees: value.total_priority_fees, 
            total_jito_tips: value.total_jito_tips, 
            total_cu: value.total_cu, 
            median_priority_fee: value.median_priority_fee, 
            max_priority_fee: value.max_priority_fee, 
            median_cu_price: value.median_cu_price 
        }
    }
}


impl ClickHouseClient {

    pub fn new(url: &str, database: &str) -> Self {
        let client = Client::default()
            .with_url(url)
            .with_database(database)
            .with_option("wait_end_of_query", "1")
            .with_option("async_insert", "1")
            .with_option("async_insert_busy_timeout_ms", "1000");
        Self { 
            client,
            database: database.to_string(),
            url: url.to_string(),
        }
    }

    pub async fn init_schema(&self) -> Result<(), clickhouse::error::Error> {

        // Create database first (using a client without database set)
        let admin_client = Client::default().with_url(&self.url);
        let create_db_query = format!("CREATE DATABASE IF NOT EXISTS {}", self.database);
        admin_client.query(&create_db_query).execute().await?;
        log::info!("Database '{}' created or already exists", self.database);


        self.client.query(TRANSACTIONS_TABLE_DDL).execute().await?;
        self.client.query(BLOCK_STATS_TABLE_DDL).execute().await?;

        log::info!("Clickhouse schema initialized successfully");
        Ok(())
    }

    pub async fn insert_transactions(&self, records: &[TransactionRecord]) -> Result<(), clickhouse::error::Error>{
        if records.is_empty(){
            return Ok(());
        }

        let mut insert = self.client.insert("transactions")?;

        for record in records {
            insert.write(&TransactionRow::from(record.clone())).await?;
        }
        insert.end().await?;

        log::debug!("Inserted {} transaction records", records.len());
        Ok(())
    }

    pub async fn insert_block(&self, stats: &BlockStats) -> Result<(), clickhouse::error::Error> {
        
        let mut insert = self.client.insert("block_stats")?;
        insert.write(&BlockStatsRow::from(stats.clone())).await?;
        insert.end().await?;

        log::debug!("Inserted block stats for slot {}", stats.slot);
        Ok(())
    }
}

const TRANSACTIONS_TABLE_DDL: &str = r#"
CREATE TABLE IF NOT EXISTS transactions (
    -- Block Context
    slot UInt64,
    block_time DateTime64(3) DEFAULT 0,
    tx_index UInt64,
    
    -- Transaction Identity
    signature String,
    fee_payer String,
    
    -- Execution Status
    success UInt8,
    
    -- Fee Information
    fee UInt64,
    base_fee UInt64,
    priority_fee UInt64,
    cu_price UInt64,
    cu_consumed UInt64,
    cu_requested UInt32,
    
    -- Jito Bundle Detection
    is_jito_bundle UInt8,
    jito_tip_amount UInt64,
    jito_tip_account String,
    
    -- Landing Method
    landing_method LowCardinality(String),
    
    -- Transaction Complexity
    num_instructions UInt32,
    num_accounts UInt32,
    num_signers UInt32,
    
)
ENGINE = MergeTree()
PARTITION BY toYYYYMMDD(toDateTime(block_time))
ORDER BY (slot, tx_index)
SETTINGS index_granularity = 8192
"#;


const BLOCK_STATS_TABLE_DDL: &str = r#"
CREATE TABLE IF NOT EXISTS block_stats (
    slot UInt64,
    block_time DateTime64(3) DEFAULT 0,
    
    -- Transaction Counts
    total_tx UInt32,
    successful_tx UInt32,
    failed_tx UInt32,
    
    -- Landing Method Breakdown
    jito_bundle_count UInt32,
    priority_fee_count UInt32,
    base_fee_count UInt32,
    
    -- Fee Aggregates
    total_fees UInt64,
    total_priority_fees UInt64,
    total_jito_tips UInt64,
    
    -- Compute Units
    total_cu UInt64,
    
    -- Fee Statistics
    median_priority_fee UInt64,
    max_priority_fee UInt64,
    median_cu_price UInt64,
    
)
ENGINE = MergeTree()
PARTITION BY toYYYYMMDD(toDateTime(block_time))
ORDER BY slot
SETTINGS index_granularity = 8192
"#;