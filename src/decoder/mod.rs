mod jito;
mod compute_budget;

use carbon_core:: { 
    instruction::DecodedInstruction,
    collection::InstructionDecoderCollection 
};
use serde::Serialize;

pub use compute_budget::ComputeBudgetInfo;
pub use jito::{JitoTipInfo, classify_landing_method};

#[derive(Debug, Clone, Serialize, Hash, PartialEq, Eq)]
pub enum PassthroughDecoder {
    Unknown,    
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PassthroughType {
    Unknown,
}

impl InstructionDecoderCollection for PassthroughDecoder {
    type InstructionType = PassthroughType;

    fn parse_instruction(
        instruction: &solana_instruction::Instruction,
    ) -> Option<DecodedInstruction<Self>> {
        Some(DecodedInstruction { program_id:instruction.program_id, data: PassthroughDecoder::Unknown, accounts: instruction.accounts.clone() })
    }

    fn get_type(&self) -> Self::InstructionType {
        PassthroughType::Unknown
    }
}