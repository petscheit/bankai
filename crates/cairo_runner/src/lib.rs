pub mod committee_update;
pub mod epoch_batch;
pub mod epoch_update;
pub mod error;
pub mod hint_processor;
pub mod types;
use cairo_vm::{
    cairo_run::{self, cairo_run_program},
    types::{layout_name::LayoutName, program::Program},
    vm::runners::cairo_pie::CairoPie,
};
use committee_update::CommitteeUpdateCircuit;
use epoch_batch::EpochUpdateBatchCircuit;
use epoch_update::EpochUpdateCircuit;
use error::Error;
use hint_processor::CustomHintProcessor;
use std::path::Path;
use serde_json;

fn load_program(path: &str) -> Result<Program, Error> {    
    // Check if it's an absolute path that doesn't exist, try relative
    let final_path = if path.starts_with('/') && !std::path::Path::new(path).exists() {
        // Try converting absolute path to relative
        let relative_path = path.strip_prefix('/').unwrap_or(path);
        println!("Absolute path not found, trying relative: {}", relative_path);
        relative_path
    } else {
        path
    };
    
    let program_file = std::fs::read(final_path).map_err(Error::IO)?;
    let cairo_run_config = cairo_run::CairoRunConfig {
        allow_missing_builtins: Some(true),
        layout: LayoutName::all_cairo,
        ..Default::default()
    };

    let program = Program::from_bytes(&program_file, Some(cairo_run_config.entrypoint))?;
    Ok(program)
}

pub fn run_committee_update(path: &str, update: CommitteeUpdateCircuit) -> Result<CairoPie, Error> {
    let program = load_program(path)?;
    let cairo_run_config = cairo_run::CairoRunConfig {
        allow_missing_builtins: Some(true),
        layout: LayoutName::all_cairo,
        ..Default::default()
    };
    let mut hint_processor = CustomHintProcessor::new(Some(update), None, None);
    let cairo_runner = cairo_run_program(&program, &cairo_run_config, &mut hint_processor)?;
    tracing::info!("{:?}", cairo_runner.get_execution_resources());

    let pie = cairo_runner.get_cairo_pie()?;
    Ok(pie)
}

pub fn run_epoch_update(path: &str, update: EpochUpdateCircuit) -> Result<CairoPie, Error> {
    let program = load_program(path)?;
    let cairo_run_config = cairo_run::CairoRunConfig {
        allow_missing_builtins: Some(true),
        layout: LayoutName::all_cairo,
        ..Default::default()
    };
    let mut hint_processor = CustomHintProcessor::new(None, Some(update), None);
    let cairo_runner = cairo_run_program(&program, &cairo_run_config, &mut hint_processor)?;
    tracing::info!("{:?}", cairo_runner.get_execution_resources());

    let pie = cairo_runner.get_cairo_pie()?;
    Ok(pie)
}

pub fn run_epoch_batch(path: &str, update: EpochUpdateBatchCircuit) -> Result<CairoPie, Error> {
    let program = load_program(path)?;
    let cairo_run_config = cairo_run::CairoRunConfig {
        allow_missing_builtins: Some(true),
        layout: LayoutName::all_cairo,
        ..Default::default()
    };
    let mut hint_processor = CustomHintProcessor::new(None, None, Some(update));
    let cairo_runner = cairo_run_program(&program, &cairo_run_config, &mut hint_processor)?;
    tracing::info!("{:?}", cairo_runner.get_execution_resources());

    let pie = cairo_runner.get_cairo_pie()?;
    Ok(pie)
}

pub fn run_epoch_update_stwo(path: &str, update: EpochUpdateCircuit, output_dir: &str) -> Result<(), Error> {
    
    let program = load_program(path)?;
    let cairo_run_config = cairo_run::CairoRunConfig {
        allow_missing_builtins: Some(true),
        layout: LayoutName::all_cairo,
        relocate_mem: true,
        trace_enabled: true,
        disable_trace_padding: false,
        ..Default::default()
    };

    let mut hint_processor = CustomHintProcessor::new(None, Some(update), None);
    let cairo_runner = cairo_run_program(&program, &cairo_run_config, &mut hint_processor)?;
    
    tracing::info!("{:?}", cairo_runner.get_execution_resources());

    generate_stwo_files(&cairo_runner, output_dir)?;
    Ok(())
}


pub fn run_epoch_batch_stwo(
    path: &str, 
    update: EpochUpdateBatchCircuit,
    output_dir: &str
) -> Result<(), Error> {
    let program = load_program(path)?;
    let cairo_run_config = cairo_run::CairoRunConfig {
        allow_missing_builtins: Some(true),
        layout: LayoutName::all_cairo,
        relocate_mem: true,
        ..Default::default()
    };
    let mut hint_processor = CustomHintProcessor::new(None, None, Some(update));
    let mut cairo_runner = cairo_run_program(&program, &cairo_run_config, &mut hint_processor)?;
    let rel_table = cairo_runner
        .vm
        .segments
        .relocate_segments()
        .expect("Couldn't relocate after compute effective sizes");
    cairo_runner.relocate_trace(&rel_table).unwrap();
    tracing::info!("{:?}", cairo_runner.get_execution_resources());

    generate_stwo_files(&cairo_runner, output_dir)?;
    Ok(())
}

fn generate_stwo_files(cairo_runner: &cairo_vm::vm::runners::cairo_runner::CairoRunner, output_dir: &str) -> Result<(), Error> {
    println!("exporting to {}", output_dir);
    std::fs::create_dir_all(output_dir)?;
    
    // 1. Generate air_public_inputs.json
    let public_input = cairo_runner.get_air_public_input();
    println!("Public input: {:?}", public_input);
    let public_input_json = serde_json::to_string_pretty(&public_input.unwrap()).unwrap();
    std::fs::write(
        Path::new(output_dir).join("air_public_inputs.json"),
        public_input_json
    )?;
    
    // 3. Generate trace.bin first
    if let Some(trace) = &cairo_runner.relocated_trace {
        let trace_binary = serialize_trace_to_binary(trace)?;
        std::fs::write(
            Path::new(output_dir).join("trace.bin"),
            trace_binary
        )?;
    }
    
    // 4. Generate memory.bin
    let memory_binary = serialize_memory_to_binary(&cairo_runner.relocated_memory)?;
    std::fs::write(
        Path::new(output_dir).join("memory.bin"),
        memory_binary
    )?;
    
    // 2. Generate air_private_inputs.json (after binary files are created)
    let private_input = cairo_runner.get_air_private_input();
    let trace_path = Path::new(output_dir).join("trace.bin").to_string_lossy().to_string();
    let memory_path = Path::new(output_dir).join("memory.bin").to_string_lossy().to_string();
    let private_input_serializable = private_input.to_serializable(trace_path, memory_path);
    let private_input_json = serde_json::to_string_pretty(&private_input_serializable).unwrap();
    std::fs::write(
        Path::new(output_dir).join("air_private_inputs.json"),
        private_input_json
    )?;
    
    Ok(())
}

fn serialize_trace_to_binary(trace: &[cairo_vm::vm::trace::trace_entry::RelocatedTraceEntry]) -> Result<Vec<u8>, Error> {
    let mut binary_data = Vec::new();
    
    for entry in trace {
        // Each trace entry contains: pc, ap, fp
        // Serialize as 3 * 8 bytes = 24 bytes per entry
        binary_data.extend_from_slice(&entry.pc.to_be_bytes());
        binary_data.extend_from_slice(&entry.ap.to_be_bytes());  
        binary_data.extend_from_slice(&entry.fp.to_be_bytes());
    }
    
    Ok(binary_data)
}

fn serialize_memory_to_binary(memory: &[Option<cairo_vm::Felt252>]) -> Result<Vec<u8>, Error> {
    let mut binary_data = Vec::new();
    
    for memory_cell in memory {
        match memory_cell {
            Some(felt) => {
                // Serialize each felt as 32 bytes
                binary_data.extend_from_slice(&felt.to_bytes_be());
            }
            None => {
                // Use zero bytes for empty memory cells
                binary_data.extend_from_slice(&[0u8; 32]);
            }
        }
    }
    
    Ok(binary_data)
}
