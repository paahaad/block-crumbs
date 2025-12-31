use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionRecord {
    // Block metadata
    pub slot: u64,
    pub block_time: Option<i64>,
    pub tx_index: Option<u64>, // Position of transaction in block

    // Transaction metadata
    pub signature: String,
    pub fee_payer: String,

    // Execution metadata
    pub success: bool, // If tx succeeded

    // Fee metadata
    pub fee: u64,
    pub base_fee: u64,
    pub priority_fee: u64,
    pub cu_price: Option<u64>,
    pub cu_consumed: u64,
    pub cu_requested: Option<u32>,

    // Jito Detection
    pub is_jito_bundle: bool,
    pub jito_tip_account: Option<String>,
    pub jito_tip_amount: Option<u64>,


    // Tx Landing Method: "jito_bundle", "priority_fee", "base_fee"
    pub landing_method: String,

    // Tx Complexity
    pub num_instructions: u32,
    pub num_accounts: u32,
    pub num_signers: u32, 

}



#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockStats{
    pub slot: u64,
    pub block_time: Option<i64>,
    
    // Transation Count
    pub total_tx: u32,
    pub successful_tx: u32,
    pub failed_tx: u32,

    // Landing Method breakdown
    pub jito_bundle_count: u32,
    pub priority_fee_count: u32,
    pub base_fee_count: u32,

    // Fee Aggregates
    pub total_fees: u64,
    pub total_priority_fees: u64,
    pub total_jito_tips: u64,

    // CU Aggregates
    pub total_cu: u64,

    // Fee Statistic
    pub median_priority_fee: u64,
    pub max_priority_fee: u64,
    pub median_cu_price: u64,

    // Internal: for computing medians (not serialized to DB)
    #[serde(skip)]
    priority_fees: Vec<u64>,
    #[serde(skip)]
    cu_prices: Vec<u64>,
}


impl BlockStats {
    pub fn new(slot: u64, block_time: Option<i64> ) -> Self {
        Self { 
            slot,
            block_time, 
            total_tx: 0, 
            successful_tx: 0, 
            failed_tx: 0, 
            jito_bundle_count: 0, 
            priority_fee_count: 0, 
            base_fee_count: 0, 
            total_fees: 0, 
            total_priority_fees: 0, 
            total_jito_tips: 0, 
            total_cu: 0, 
            median_priority_fee: 0, 
            max_priority_fee: 0, 
            median_cu_price: 0,
            priority_fees: Vec::new(),
            cu_prices: Vec::new(),
        }
    }

    pub fn add_transaction(&mut self, tx: &TransactionRecord){
        self.total_tx += 1;

        if tx.success {
            self.successful_tx += 1;
        }else {
            self.failed_tx += 1;
        }

        match tx.landing_method.as_str() {
            "jito_bundle" => self.jito_bundle_count += 1,
            "priority_fee" => self.priority_fee_count += 1,
            _ => self.base_fee_count += 1
        }

        self.total_fees += tx.fee;
        self.total_priority_fees += tx.priority_fee;
        self.total_jito_tips += tx.jito_tip_amount.unwrap_or(0);
        self.total_cu += tx.cu_consumed;

        if tx.priority_fee > self.max_priority_fee {
            self.max_priority_fee = tx.priority_fee;
        }

        // Collect values for median calculation
        if tx.priority_fee > 0 {
            self.priority_fees.push(tx.priority_fee);
        }
        if let Some(cu_price) = tx.cu_price {
            if cu_price > 0 {
                self.cu_prices.push(cu_price);
            }
        }
    }

    pub fn finalize(&mut self) {
        self.median_priority_fee = Self::compute_median(&mut self.priority_fees);
        self.median_cu_price = Self::compute_median(&mut self.cu_prices);
        
        self.priority_fees.clear();
        self.cu_prices.clear();
    }

    fn compute_median(values: &mut [u64]) -> u64 {
        if values.is_empty() {
            return 0;
        }
        
        values.sort_unstable();
        let len = values.len();
        
        if len % 2 == 0 {
            (values[len / 2 - 1] + values[len / 2]) / 2
        } else {
            values[len / 2]
        }
    }
}