mod db;
mod decoder;
mod processor;

use solana_commitment_config::CommitmentConfig;
use solana_transaction_status::UiTransactionEncoding;
use std::{env, sync::{Arc}, time::Duration};
use tokio::sync::mpsc;
use carbon_core:: {
    pipeline::{ 
        ShutdownStrategy, Pipeline 
    }, 
    error::CarbonResult 
};
use carbon_rpc_block_crawler_datasource::{ RpcBlockConfig, RpcBlockCrawler };
use carbon_log_metrics::LogMetrics;

use crate::decoder::PassthroughDecoder;
use crate::processor::transaction_analyzer::TransactonAnaylzer;
use crate::db::writer::clickhouse_writer_task;

#[tokio::main]
async fn main() -> CarbonResult<()> {

    dotenv::dotenv().ok();
    env_logger::init(); 

    let rpc_url = env::var("RPC_URL").expect("RPC_URL should be provided");
    let ch_url = env::var("CLICKHOUSE_URL").expect("Clickhouse URL must pe provided");
    let ch_database = env::var("CLICKHOUSE_DATABASE").expect("Database name is missing");
    
    let solana_client = solana_client::rpc_client::RpcClient::new(&rpc_url);
    let slot = solana_client.get_slot().expect("Failed to get slot, Check RPC_URL");

    // Channels for carbon processor to DB writer
    let (tx_sender, tx_receiver) = mpsc::channel(10_000);
    let (block_sender, block_reciver) = mpsc::channel(1000);
    log::info!("Starting Pipeline");
    log::info!("    RPC URL {}", rpc_url);
    log::info!("    Start slot: {}", slot);
    log::info!("    Clickhouse URL: {}. Clickhouse Database: {}", ch_url, ch_database);
    
    tokio::spawn(async move {
        clickhouse_writer_task(tx_receiver, block_reciver, ch_url.clone(), ch_database.clone()).await
    });

    // Set data source
    let block_crawler = RpcBlockCrawler::new(
        rpc_url, 
        slot, 
        None, // No end slot i.e stream indefintly
        Some(Duration::from_millis(200)), // Poll interval 
        RpcBlockConfig{
            encoding: Some(UiTransactionEncoding::Base58),
            commitment: Some(CommitmentConfig::confirmed()),
            max_supported_transaction_version: Some(0),
            ..Default::default()
        }, 
        Some(3), // Max concurrecy
        Some(500), // Channel buffer, Carbon uses channel to send block to task processor
    );


    let processor = TransactonAnaylzer::new(tx_sender, block_sender);
    let schema = None::<carbon_core::schema::TransactionSchema<PassthroughDecoder>>;
    
    // Build and run pipeline
    Pipeline::builder()
        .datasource(block_crawler)
        .metrics(Arc::new(LogMetrics::new()))
        .metrics_flush_interval(10)
        .transaction(processor, schema)
        .shutdown_strategy(ShutdownStrategy::Immediate)
        .build()?
        .run()
        .await?;


    Ok(())
}