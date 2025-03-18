use bankai_core::types::job::Job;

pub fn broadcast_job(job: Job) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    Ok(())
}