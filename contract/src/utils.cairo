use core::poseidon::PoseidonImpl;
use core::hash::HashStateTrait;

pub const WRAPPER_PROGRAM_HASH: felt252 =
    0x193641eb151b0f41674641089952e60bc3aded26e3cf42793655c562b8c3aa0;

pub fn calculate_wrapped_bootloaded_fact_hash(
    wrapper_program_hash: felt252,
    bootloader_program_hash: felt252,
    child_program_hash: felt252,
    child_output: Span<felt252>,
) -> felt252 {
    let mut bootloader_output = PoseidonImpl::new()
        .update(0x1)
        .update(child_output.len().into() + 2)
        .update(child_program_hash);
    for x in child_output {
        bootloader_output = bootloader_output.update(*x);
    };

    let mut wrapper_output = PoseidonImpl::new()
        .update(0x1)
        .update(0x4)
        .update(wrapper_program_hash)
        .update(bootloader_program_hash)
        .update(bootloader_output.finalize());

    PoseidonImpl::new().update(bootloader_program_hash).update(wrapper_output.finalize()).finalize()
}
