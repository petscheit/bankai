use crate::error::DaemonError;
use alloy_rpc_types_beacon::events::HeadEvent;
use bankai_core::types::job::Job;
use bankai_core::utils::helpers;
use reqwest;
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio::time::{timeout, Duration};
use tokio_stream::StreamExt;
use tracing::{error, info, warn};
pub struct BeaconListener {
    events_endpoint: String,
    tx: mpsc::Sender<HeadEvent>,
}

impl BeaconListener {
    pub fn new(events_endpoint: String, tx: mpsc::Sender<HeadEvent>) -> Self {
        Self {
            events_endpoint,
            tx,
        }
    }

    pub async fn start(&self) -> Result<(), DaemonError> {
        let events_endpoint = self.events_endpoint.clone();
        let tx = self.tx.clone();

        tokio::spawn(async move {
            let http_stream_client = reqwest::Client::new();

            loop {
                // Send the request to the Beacon node
                let response = match http_stream_client.get(&events_endpoint).send().await {
                    Ok(r) => r,
                    Err(e) => {
                        error!("Failed to connect: {}", e);
                        tokio::time::sleep(std::time::Duration::from_secs(5)).await;
                        continue; // retry
                    }
                };

                if !response.status().is_success() {
                    error!("Got non-200: {}", response.status());
                    tokio::time::sleep(std::time::Duration::from_secs(5)).await;
                    continue; // retry
                }

                info!("Listening for new slots, epochs and sync committee updates...");

                let mut stream = response.bytes_stream();

                loop {
                    match timeout(Duration::from_secs(30), stream.next()).await {
                        // Timed out
                        Err(_elapsed) => {
                            warn!("Timed out waiting for new slot beacon chain event chunk. Maybe some slots was skipped. Will reconnect...");
                            break;
                        }
                        Ok(Some(Ok(bytes))) => {
                            if let Ok(event_text) = String::from_utf8(bytes.to_vec()) {
                                if let Some(json_data) =
                                    helpers::extract_json_from_event(&event_text)
                                {
                                    match serde_json::from_str::<HeadEvent>(&json_data) {
                                        Ok(event) => {
                                            // Clone the sender before using it in the async context
                                            let tx = tx.clone();
                                            if let Err(e) = tx.send(event).await {
                                                error!("Failed to send event to channel: {}", e);
                                                // Continue processing other events
                                            }
                                        }
                                        Err(e) => {
                                            error!("Failed to parse event: {}", e);
                                        }
                                    }
                                }
                            }
                        }
                        Ok(Some(Err(e))) => {
                            warn!("Beacon chain client stream error: {}", e);
                            break;
                        }
                        Ok(None) => {
                            warn!("Beacon chain client stream ended");
                            break;
                        }
                    }
                }

                info!("Timeout waiting for next event, reconnecting to beacon node...");
            }
        });

        Ok(())
    }
}
