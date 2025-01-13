impl DatabaseManager {
    pub async fn new(db_url: &str) -> Result<Self, Box<dyn Error + Send + Sync>> {
        let (client, connection) = tokio_postgres::connect(db_url, NoTls).await?;

        // Spawn a task to handle the connection so it is always polled
        tokio::spawn(async move {
            if let Err(e) = connection.await {
                eprintln!("Database connection error: {}", e);
            }
        });

        Ok(Self { client })
    }

    /// Inserts a verified epoch into the `verified_epoch` table.
    pub async fn insert_verified_epoch(
        &self,
        epoch_id: u64,
        epoch_proof: EpochProof,
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        self.client
            .execute(
                "INSERT INTO verified_epoch (epoch_id, header_root, state_root, n_signers)
             VALUES ($1, $2, $3, $4)",
                &[
                    &epoch_id.to_string(),
                    &epoch_proof.header_root.to_string(),
                    &epoch_proof.state_root.to_string(),
                    &epoch_proof.n_signers.to_string(),
                    &epoch_proof.execution_hash.to_string(),
                    &epoch_proof.execution_height.to_string(),
                ],
            )
            .await?;

        Ok(())
    }

    pub async fn insert_verified_sync_committee(
        &self,
        sync_committee_id: u64,
        sync_committee_hash: FixedBytes<32>,
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        self.client
            .execute(
                "INSERT INTO verified_sync_committee (sync_committee_id, sync_committee_hash)
             VALUES ($1, $2)",
                &[
                    &sync_committee_id.to_string(),
                    &sync_committee_hash.to_string(),
                ],
            )
            .await?;

        Ok(())
    }
}
