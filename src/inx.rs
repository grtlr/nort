use bee_inx::{client, Marker, LedgerSpent, LedgerOutput};
use futures::{StreamExt, TryStreamExt};
use tracing::info;

use crate::shutdown::ShutdownSignal;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("invalid milestone state")]
    InvalidMilestoneState,
    #[error(transparent)]
    BeeInxError(#[from] bee_inx::Error),
}

pub async fn start(shutdown: ShutdownSignal) -> Result<(), Error> {
    tracing::info!("Connecting to INX");
        let mut inx = client::Inx::connect("http://localhost:9013".into()).await?;
        tracing::info!("Connected to INX");
        
        let mut stream = inx.listen_to_ledger_updates((..).into()).await?.take_until(shutdown.listen());

        while let Some(ledger_update) = stream.try_next().await? {
            
            info!("received ledger update record: {:?}", ledger_update);
    
            let Marker {
                milestone_index,
                consumed_count,
                created_count,
            } = ledger_update.begin().ok_or(Error::InvalidMilestoneState)?;
    
            info!("Received begin of milestone {milestone_index} with {consumed_count} consumed and {created_count} created outputs.");
    
            let mut consumed_stream = stream.by_ref().take(consumed_count);
            while let Some(ledger_update) = consumed_stream.try_next().await? {
                let LedgerSpent { output: LedgerOutput { output_id, ..}, ..} = ledger_update.consumed().ok_or(Error::InvalidMilestoneState)?;
                info!("Spent output: {output_id}");
                // Handle spent outputs here.
            }
            info!("switching to spent");
    
            let mut created_stream = stream.by_ref().take(created_count);
            while let Some(ledger_update) = created_stream.try_next().await? {
                let LedgerOutput { output_id, ..} = ledger_update.created().ok_or(Error::InvalidMilestoneState)?;
                info!("Created output: {output_id}");
                // Handle created outputs here.
            }
    
            if let Some(ledger_update) = stream.try_next().await? {
                let Marker {
                    milestone_index,
                    consumed_count,
                    created_count,
                } = ledger_update.end().ok_or(Error::InvalidMilestoneState)?;
                info!("Received end of milestone {milestone_index} with {consumed_count} consumed and {created_count} created outputs.");
            }
        }

        match stream.take_result() {
            Some(_) => tracing::info!("INX stream closed due to shutdown signal"),
            None => {
                tracing::info!("INX stream closed unexpectedly, sending shutdown signal to app");
                shutdown.signal().await;
            }
        }

        Ok(())
}
