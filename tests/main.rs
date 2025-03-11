use bankai_core::cairo_runner::rust::{generate_epoch_batch_pie, generate_pie};
use bankai_core::types::proofs::epoch_batch::EpochUpdateBatch;
use bankai_core::types::proofs::sync_committee::SyncCommitteeUpdate;
use std::env;
use std::fs;
use std::path::Path;
use tokio::process::Command;
use walkdir;
use futures::stream::{self, TryStreamExt};
use futures::StreamExt;

#[tokio::main]
async fn main() {
    // Check command line arguments.
    let args: Vec<String> = env::args().collect();
    if args.len() >= 2 {
        match args[1].as_str() {
            "--process-fixture" => {
                // This spawns a new process for the fixture.
                process_fixture(&args[2]).await.unwrap();
            }
            "--debug-fixture" => {
                // Debug: run the fixture in the current process and print detailed logs.
                run_single_fixture_debug(&args[2]).await.unwrap();
            }
            "--committee" => {
                // Process committee fixtures sequentially
                run_committee_fixtures().await.unwrap();
            }
            _ => {
                run_tracegen_for_fixture().await.unwrap();
            }
        }
    } else {
        run_tracegen_for_fixture().await.unwrap();
    }
}

/// Spawns a new process for each fixture file, limiting the concurrency to 10.
/// A single errored fixture will terminate the entire execution.
async fn run_tracegen_for_fixture() -> Result<(), Box<dyn std::error::Error>> {
    let fixtures = get_fixture_files();
    let current_exe = env::current_exe()?;
    
    // Use try_for_each_concurrent so that a single failure returns an error.
    stream::iter(fixtures)
        .map(Ok)
        .try_for_each_concurrent(10, |fixture| {
            let current_exe = current_exe.clone();
            async move {
                let file_name = Path::new(&fixture)
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or(&fixture);
                let start = std::time::Instant::now();

                let mut child = Command::new(&current_exe)
                    .arg("--process-fixture")
                    .arg(&fixture)
                    .spawn()
                    .map_err(|e| {
                        Box::<dyn std::error::Error>::from(format!(
                            "ERROR: Failed to spawn process for fixture {}: {:?}",
                            file_name, e
                        ))
                    })?;
                
                let status = child.wait().await.map_err(|e| {
                    Box::<dyn std::error::Error>::from(format!(
                        "ERROR: Failed to wait on fixture {}: {:?}",
                        file_name, e
                    ))
                })?;
                
                if !status.success() {
                    return Err(Box::<dyn std::error::Error>::from(format!(
                        "ERROR: Fixture {}: Process exited with status: {:?}",
                        file_name, status
                    )));
                }
                
                println!(
                    "[SUCCESS] Finished fixture: {} in {:?}",
                    file_name,
                    start.elapsed()
                );
                Ok(())
            }
        })
        .await?;
    Ok(())
}

/// Processes a single fixture file.
async fn process_fixture(fixture: &str) -> Result<(), Box<dyn std::error::Error>> {
    let file_name = Path::new(fixture)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or(fixture);
    let start_time = std::time::Instant::now();

    let content = fs::read_to_string(fixture)?;
    let batch: EpochUpdateBatch = serde_json::from_str(&content).map_err(|e| {
        Box::<dyn std::error::Error>::from(format!(
            "ERROR: Failed to deserialize fixture {}: {:?}",
            file_name, e
        ))
    })?;
    let config = bankai_core::utils::config::BankaiConfig::test_config();

    // Run the cryptographic operation.
    generate_epoch_batch_pie(batch, &config, None, None)
        .await
        .map_err(|e| {
            Box::<dyn std::error::Error>::from(format!(
                "ERROR: Processing fixture {} failed: {:?}",
                file_name, e
            ))
        })?;

    Ok(())
}

/// Returns all fixture file paths as a vector.
fn get_fixture_files() -> Vec<String> {
    let fixtures_dir = Path::new("fixtures/epoch_batch");
    walkdir::WalkDir::new(fixtures_dir)
        .into_iter()
        .filter_map(|entry| entry.ok())
        .filter(|entry| {
            entry
                .path()
                .extension()
                .and_then(|ext| ext.to_str())
                == Some("json")
        })
        .map(|entry| entry.path().to_str().unwrap().to_owned())
        .collect()
}

/// New function for debugging: processes a single fixture file and prints logs immediately.
async fn run_single_fixture_debug(fixture: &str) -> Result<(), Box<dyn std::error::Error>> {
    println!("Running fixture (debug): {}", fixture);
    let start_time = std::time::Instant::now();
    let result = process_fixture(fixture).await;
    match &result {
        Ok(_) => {
            println!(
                "[SUCCESS] Processed fixture {} in {:?}",
                fixture,
                start_time.elapsed()
            );
        }
        Err(e) => {
            eprintln!("[ERROR] Processing fixture {} failed: {:?}", fixture, e);
        }
    }
    result
}

/// Returns all committee fixture file paths as a vector.
fn get_committee_fixture_files() -> Vec<String> {
    let fixtures_dir = Path::new("fixtures/committee");
    walkdir::WalkDir::new(fixtures_dir)
        .into_iter()
        .filter_map(|entry| entry.ok())
        .filter(|entry| {
            entry
                .path()
                .extension()
                .and_then(|ext| ext.to_str())
                == Some("json")
        })
        .map(|entry| entry.path().to_str().unwrap().to_owned())
        .collect()
}

/// Processes committee fixtures sequentially
async fn run_committee_fixtures() -> Result<(), Box<dyn std::error::Error>> {
    let fixtures = get_committee_fixture_files();
    for fixture in fixtures {
        let file_name = Path::new(&fixture)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap();
        let start = std::time::Instant::now();
        
        let content = fs::read_to_string(fixture.clone())?;
        let batch: SyncCommitteeUpdate = serde_json::from_str(&content).map_err(|e| {
            Box::<dyn std::error::Error>::from(format!(
                "ERROR: Failed to deserialize fixture {}: {:?}",
                file_name, e
            ))
        })?;
        let config = bankai_core::utils::config::BankaiConfig::test_config();

        // Run the cryptographic operation.
        generate_pie(batch, &config, None, None)
            .await
            .map_err(|e| {
                Box::<dyn std::error::Error>::from(format!(
                    "ERROR: Processing fixture {} failed: {:?}",
                    file_name, e
                ))
            })?;

            
        println!(
                "[SUCCESS] Finished committee fixture: {} in {:?}",
                file_name,
                start.elapsed()
            );
    }
    Ok(())
}
