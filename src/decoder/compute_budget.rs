use borsh::BorshDeserialize;
use solana_pubkey::Pubkey;


pub const COMPUTE_BUDGET_PROGRAM_ID: Pubkey = Pubkey::from_str_const("ComputeBudget111111111111111111111111111111");


/// https://github.com/solana-labs/solana/blob/master/sdk/src/compute_budget.rs
/// Decoded Compute Budget instruction data
#[derive(Debug, Clone, PartialEq, Eq, BorshDeserialize)]
pub enum ComputeBudgetInstruction {
    /// Deprecated variant, reserved value (discriminator 0x00)
    Unused,
    /// Request a specific transaction-wide program heap region size in bytes.
    /// The value requested must be a multiple of 1024.
    RequestHeapFrame(u32),
    /// Set a specific compute unit limit that the transaction is allowed to consume.
    SetComputeUnitLimit(u32),
    /// Set a compute unit price in "micro-lamports" to pay a higher transaction
    /// fee for higher transaction prioritization.
    SetComputeUnitPrice(u64),
    /// Set a specific transaction-wide account data size limit, in bytes, is allowed to load.
    SetLoadedAccountsDataSizeLimit(u32),
}


impl ComputeBudgetInstruction {
    pub fn decode(data: &[u8]) -> Option<Self> {
        Self::try_from_slice(data).ok()
    }
}

#[derive(Debug, Clone, Default)]
pub struct ComputeBudgetInfo {
    pub cu_limit: Option<u32>, // CU set by tx, default is 200k
    pub cu_price: Option<u64>, // CU price in micro lamports
    pub head_frame: Option<u32>,
}

impl ComputeBudgetInfo {
    pub fn from_ixs(ixs: &[solana_instruction::Instruction]) -> Self {
        let mut info = ComputeBudgetInfo::default();

        for ix in ixs {
            if ix.program_id != COMPUTE_BUDGET_PROGRAM_ID {
                continue;
            }

            if let Some(decoded) = ComputeBudgetInstruction::decode(&ix.data) {
                match decoded {
                    ComputeBudgetInstruction::SetComputeUnitLimit(units) => {
                        info.cu_limit  = Some(units);
                    }
                    ComputeBudgetInstruction::SetComputeUnitPrice(micro_lamports) => {
                        info.cu_price = Some(micro_lamports);
                    }
                    ComputeBudgetInstruction::RequestHeapFrame(bytes) => {
                        info.head_frame = Some(bytes);
                    }

                    _ => {}
                }
            }
        }

        info
    }

    // Link: https://solana.com/docs/core/fees#prioritization-fees
    // Prioritization fee = CU limit × CU price
    pub fn calculate_priority_fee(&self) -> u64 {
        match (self.cu_price, self.cu_limit) {
            (Some(price), Some(limit)) => (price * limit as u64) / 1_000_000,
            _ => 0
        }
    }

    #[allow(dead_code)]
    pub fn priority_fee_from_meta(total_fee: u64, num_signatures: u64) -> u64 {
        let base_fee = 5000 * num_signatures;
        total_fee.saturating_sub(base_fee)
    }

}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decode_set_compute_unit_limit() {
        // Discriminator 0x02 + u32 little-endian 400000 = 0x00061a80
        let data = vec![0x02, 0x80, 0x1a, 0x06, 0x00];
        let decoded = ComputeBudgetInstruction::decode(&data);
        assert_eq!(
            decoded,
            Some(ComputeBudgetInstruction::SetComputeUnitLimit(400000))
        );
    }

    #[test]
    fn test_decode_set_compute_unit_price() {
        // Discriminator 0x03 + u64 little-endian 1000000 = 0x00000000000f4240
        let data = vec![0x03, 0x40, 0x42, 0x0f, 0x00, 0x00, 0x00, 0x00, 0x00];
        let decoded = ComputeBudgetInstruction::decode(&data);
        assert_eq!(
            decoded,
            Some(ComputeBudgetInstruction::SetComputeUnitPrice(1_000_000))
        );
    }

    #[test]
    fn test_decode_request_heap_frame() {
        // Discriminator 0x01 + u32 little-endian 262144 (256KB)
        let data = vec![0x01, 0x00, 0x00, 0x04, 0x00];
        let decoded = ComputeBudgetInstruction::decode(&data);
        assert_eq!(
            decoded,
            Some(ComputeBudgetInstruction::RequestHeapFrame(262144))
        );
    }

    #[test]
    fn test_priority_fee_from_meta() {
        // 1 signature, total fee 10000 lamports
        // base fee = 5000, priority fee = 5000
        assert_eq!(ComputeBudgetInfo::priority_fee_from_meta(10000, 1), 5000);

        // 2 signatures, total fee 15000 lamports
        // base fee = 10000, priority fee = 5000
        assert_eq!(ComputeBudgetInfo::priority_fee_from_meta(15000, 2), 5000);
    }
}