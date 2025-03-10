pub mod hint_processor;
pub mod error;
pub mod committee_update;
pub mod types;
pub mod epoch_update;
pub mod epoch_batch;
use cairo_vm::{
    cairo_run::{self, cairo_run_program},
    types::{layout_name::LayoutName, program::Program}, vm::runners::cairo_pie::CairoPie,
};
use epoch_batch::EpochUpdateBatchCircuit;
use error::Error;
use hint_processor::CustomHintProcessor;
use committee_update::CommitteeUpdateCircuit;
use epoch_update::EpochUpdateCircuit;

fn load_program(path: &str) -> Result<Program, Error> {
    let program_file = std::fs::read(path).map_err(Error::IO)?;
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
    println!("{:?}", cairo_runner.get_execution_resources());

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
    println!("Running epoch update...");
    let mut hint_processor = CustomHintProcessor::new(None, Some(update), None);
    let cairo_runner = cairo_run_program(&program, &cairo_run_config, &mut hint_processor)?;
    println!("{:?}", cairo_runner.get_execution_resources());

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
    println!("Running epoch batch...");
    let mut hint_processor = CustomHintProcessor::new(None, None, Some(update));
    let cairo_runner = cairo_run_program(&program, &cairo_run_config, &mut hint_processor)?;
    println!("{:?}", cairo_runner.get_execution_resources());

    let pie = cairo_runner.get_cairo_pie()?;
    Ok(pie)
}

// fn main() -> Result<(), Error> {
//     // Init CairoRunConfig
//     let cairo_run_config = cairo_run::CairoRunConfig {
//         allow_missing_builtins: Some(true),
//         layout: LayoutName::all_cairo,
//         ..Default::default()
//     };
//     println!("Loading hash_to_curve program...");
//     let program_file = std::fs::read("../build/epoch_update.json").map_err(Error::IO)?;

//     let update = CommitteeUpdate::from_file("committee_update_input.json").unwrap();
    
//     // Load the Program
//     let program = Program::from_bytes(&program_file, Some(cairo_run_config.entrypoint))?;

//     let mut hint_processor = CustomHintProcessor::new(Some(update));
//     let cairo_runner = cairo_run_program(&program, &cairo_run_config, &mut hint_processor).unwrap();

//     println!("{:?}", cairo_runner.get_execution_resources());

//     Ok(())
// }
