use std::sync::Arc;
use async_trait::async_trait;

use carbon_core::{
    error::CarbonResult, 
    processor::Processor, 
    transaction::TransactionProcessorInputType,
    metrics::MetricsCollection

};
use solana_client::rpc_response::transaction::VersionedMessage;
use solana_instruction::{Instruction, AccountMeta};
use tokio::sync::mpsc;

use crate::decoder::{ComputeBudgetInfo, JitoTipInfo, PassthroughDecoder, classify_landing_method};
use crate::db::schema::{TransactionRecord, BlockStats};

const BASE_FEE_PER_SIGNATURE: u64 = 5000;

pub struct TransactonAnaylzer {
    tx_sender: mpsc::Sender<TransactionRecord>,
    block_stats_sender: mpsc::Sender<BlockStats>,
    current_slot: u64,
    current_block_stats: Option<BlockStats>,
}

impl TransactonAnaylzer {
    pub fn new(
        tx_sender: mpsc::Sender<TransactionRecord>, block_stats_sender: mpsc::Sender<BlockStats>
    ) -> Self {
        Self { 
            tx_sender,
            block_stats_sender,
            current_slot: 0,
            current_block_stats: None,
         }
    }

    fn extract_instruction(
        message: &VersionedMessage, 
        loaded_addresses: &solana_transaction_status::TransactionStatusMeta
    ) -> Vec<Instruction> {

        let mut instructions: Vec<Instruction> = Vec::new();

        let mut all_accounts = message.static_account_keys().to_vec();
        all_accounts.extend_from_slice(&loaded_addresses.loaded_addresses.writable);
        all_accounts.extend_from_slice(&loaded_addresses.loaded_addresses.readonly);

        for compiled_ix in message.instructions() {
            let program_id = compiled_ix.program_id(&all_accounts);

            let accounts = compiled_ix.accounts.iter().map(|&idx|{
                let idx = idx as usize;
                AccountMeta{
                    pubkey: all_accounts[idx],
                    is_signer: message.is_signer(idx),
                    is_writable: message.is_maybe_writable(idx, None)
                }
            })
            .collect();

            instructions.push( Instruction { 
                program_id: *program_id, 
                accounts, 
                data: compiled_ix.data.clone()
            });
        }

        instructions

    }
}

#[async_trait]
impl Processor for TransactonAnaylzer {
    type InputType = TransactionProcessorInputType<PassthroughDecoder>;

    // INFO: This is core Processor fn which will run for each tx in a block
    async fn process(
        &mut self,
        data:Self::InputType,
        _metrics:Arc<MetricsCollection>
    ) ->  CarbonResult<()> {
        let (metadata, _instruction, _schema_match) = data;

        // Send the block if slot has passed
        if metadata.slot != self.current_slot {
            if let Some(mut block_stats) = self.current_block_stats.take() {
                block_stats.finalize();
                if let Err(e) = self.block_stats_sender.try_send(block_stats) {
                    log::warn!("Failed to send block stats: {}", e)
                }
            }

            // start new slot
            self.current_slot = metadata.slot;
            self.current_block_stats = Some(BlockStats::new(metadata.slot, metadata.block_time));

            log::info!(
                "Proccessing slot: {} | Block time: {:?}",
                metadata.slot,
                metadata.block_time.and_then(|t| chrono::DateTime::from_timestamp(t, 0)),
            );

            // log::info!("{:?}", self.current_block_stats);
        }

        let instruction = Self::extract_instruction(&metadata.message, &metadata.meta );
        // log::info!("Instruction {:?}", instruction);

        let compute_budget = ComputeBudgetInfo::from_ixs(&instruction);
        let jito_info = JitoTipInfo::from_ixs(&instruction);

        let num_signers = metadata.message.header().num_required_signatures as u32;
        let base_fee = BASE_FEE_PER_SIGNATURE * num_signers as u64;
        let cu_consumed = metadata.meta.compute_units_consumed.unwrap_or(0);
        let priority_fee = compute_budget.calculate_priority_fee();


        let landing_method = classify_landing_method(
            jito_info.is_jito_bundle,
            compute_budget.cu_price,
        );


        let num_accounts = metadata.message.static_account_keys().len() as u32
            + metadata.meta.loaded_addresses.writable.len() as u32
            + metadata.meta.loaded_addresses.readonly.len() as u32;


        let record = TransactionRecord {
            slot: metadata.slot,
            block_time: metadata.block_time,
            tx_index: metadata.index, // Index from enumerating block.transactions array (0 = first tx in block)
        
            // Transaction metadata
            signature: metadata.signature.to_string(),
            fee_payer: metadata.fee_payer.to_string(),
        
            // Execution metadata
            success: metadata.meta.status.is_ok(), // If tx succeeded
        
            // Fee metadata
            fee: metadata.meta.fee,
            base_fee,
            priority_fee,
            cu_price: compute_budget.cu_price,
            cu_consumed,
            cu_requested: compute_budget.cu_limit,
        
            // Jito Detection
            is_jito_bundle: jito_info.is_jito_bundle,
            jito_tip_account: jito_info.tip_account.map(|a| a.to_string()),
            jito_tip_amount: jito_info.tip_amount,
        
        
            // Tx Landing Method: "jito_bundle", "priority_fee", "base_fee"
            landing_method: landing_method.as_str().to_string(),
        
            // Tx Complexity
            num_instructions: instruction.len() as u32,
            num_accounts,
            num_signers, 

        };

        // log::info!("{:?}", record);
        if let Some(stats) = self.current_block_stats.as_mut() {
             stats.add_transaction(&record);
        }

        // Send the record
        if let Err(e) = self.tx_sender.try_send(record){
            log::warn!("Failed to send transaction record: {}", e)
        };
        Ok(())
    }
}


