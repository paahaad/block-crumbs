use tokio::{ sync::mpsc::Receiver };

use crate::db::{
    clickhouse::ClickHouseClient,
    schema::{BlockStats, TransactionRecord}
};

pub async fn clickhouse_writer_task(
    mut tx_receiver: Receiver<TransactionRecord>,
    mut block_reciver: Receiver<BlockStats>,
    ch_url: String,
    ch_database: String
){

    let client = ClickHouseClient::new(&ch_url, &ch_database);

    if let Err(e) = client.init_schema().await {
        log::error!("Failed to initialized the schema {}", e);
        return;
    }

    let mut tx_batch: Vec<TransactionRecord> = Vec::with_capacity(1000);
    let mut flush_interval = tokio::time::interval(tokio::time::Duration::from_secs(5));

    loop {
        tokio::select! {
            Some(record) = tx_receiver.recv() => {
                tx_batch.push(record);

                // flush in batch is large enough
                if tx_batch.len() >= 500 {
                    if let Err(e) = client.insert_transactions(&tx_batch).await {
                        log::error!("Failed to insert transaction {}", e);
                    }else {
                        log::info!("Flushed {} transaction to clickhouse", tx_batch.len())
                    }
                    tx_batch.clear();
                }
            }

            // Receive block stats
            Some(stats) = block_reciver.recv() => {
                if let Err(e) = client.insert_block(&stats).await {
                    log::error!("Failed to insert block stats: {}", e);
                }
            }

           // Periodic flush
           _ = flush_interval.tick() => {
            if !tx_batch.is_empty() {
                if let Err(e) = client.insert_transactions(&tx_batch).await {
                    log::error!("Failed to insert transactions: {}", e);
                } else {
                    log::info!("Periodic flush: {} transaction records", tx_batch.len());
                }
                tx_batch.clear();
            }
        }
        }
    }
}