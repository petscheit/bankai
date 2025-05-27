%builtins output pedersen range_check ecdsa bitwise ec_op keccak poseidon range_check96 add_mod mul_mod

from starkware.cairo.common.cairo_builtins import PoseidonBuiltin, ModBuiltin, BitwiseBuiltin, HashBuiltin
from starkware.cairo.common.uint256 import Uint256
from starkware.cairo.common.alloc import alloc
from starkware.cairo.common.builtin_poseidon.poseidon import (
    poseidon_hash,
    poseidon_hash_many,
)
from starkware.cairo.common.memcpy import memcpy
from starkware.cairo.common.memset import memset
from starkware.cairo.common.math import assert_lt, assert_le
from cairo.src.merkle import PoseidonMerkleTree
from cairo.src.verify_epoch import run_epoch_update

from cairo.src.utils import pow2alloc128
from cairo.src.types import EpochUpdateBatch
from sha import SHA256
from debug import print_string, print_felt

func main{
    output_ptr: felt*,
    pedersen_ptr: HashBuiltin*,
    range_check_ptr,
    ecdsa_ptr: felt*,
    bitwise_ptr: BitwiseBuiltin*,
    ec_op_ptr: felt*,
    keccak_ptr: felt*,
    poseidon_ptr: PoseidonBuiltin*,
    range_check96_ptr: felt*,
    add_mod_ptr: ModBuiltin*,
    mul_mod_ptr: ModBuiltin*,
} () {
    alloc_locals;

    let (pow2_array) = pow2alloc128();
    let (sha256_ptr, sha256_ptr_start) = SHA256.init();

    local epoch_batch: EpochUpdateBatch;
    local batch_len: felt;
    %{ write_epoch_update_batch_inputs() %}
    
    with pow2_array, sha256_ptr {
        let (epoch_outputs: felt*) = alloc();
        let (latest_batch_output: felt*) = run_epoch_batches{
            output_ptr=epoch_outputs,
        }(epoch_batch, 0, batch_len, 0);

        // Retrieve the next power of 2 index
        local next_power_of_2_index: felt;
        %{ set_next_power_of_2() %}

        // Retrieve actual next power of 2
        let next_power_of_2 = pow2_array[next_power_of_2_index];

        // Ensure the batch length is less than the next power of 2
        assert_le(batch_len, next_power_of_2);

        let previous_power_of_2 = pow2_array[next_power_of_2_index - 1];
        // Ensure the previous power of 2 is less than the batch length
        assert_le(previous_power_of_2, batch_len);

        // Pad the epoch outputs with zeros to the next power of 2
        memset(dst=epoch_outputs + batch_len, value=0, n=next_power_of_2 - batch_len);

        // now we compute a merkle root of the epoch outputs
        let batch_root = PoseidonMerkleTree.compute_root(epoch_outputs, next_power_of_2);
    }

    assert [output_ptr] = batch_root;

    // Copy the latest batch output to the output_ptr
    memcpy(dst=output_ptr + 1, src=latest_batch_output, len=11);
    tempvar output_ptr = output_ptr + 12;

    %{ assert_epoch_batch_outputs() %}

    SHA256.finalize(sha256_start_ptr=sha256_ptr_start, sha256_end_ptr=sha256_ptr);
    return ();
}

func run_epoch_batches{
    range_check_ptr,
    output_ptr: felt*,
    bitwise_ptr: BitwiseBuiltin*,
    poseidon_ptr: PoseidonBuiltin*,
    range_check96_ptr: felt*,
    add_mod_ptr: ModBuiltin*,
    mul_mod_ptr: ModBuiltin*,
    pow2_array: felt*,
    sha256_ptr: felt*,
}(epoch_batch: EpochUpdateBatch, index: felt, batch_len: felt, previous_epoch: felt) -> (latest_batch_output: felt*) {
    alloc_locals;

    // Create a new output_ptr per batch
    let (epoch_output: felt*) = alloc();
    let epoch = epoch_batch.epochs[index];
    run_epoch_update{
        output_ptr=epoch_output,
    }(epoch);

    // set output_ptr to first output
    let epoch_output = epoch_output - 11;

    %{ assert_batched_epoch_outputs() %}

    // Verify the slot number matches what we expect
    let current_slot = epoch_output[4];
    local current_epoch: felt;
    %{ compute_epoch_from_slot() %}

    // Ensure the slot matches the computed epoch
    // We dont guarantee that we use the checkpoint slot, but we do guarantee that the slot is in the correct epoch
    assert_le(current_epoch * 32, current_slot);
    assert_lt(current_slot, (current_epoch + 1) * 32);


    // Ensure we dont skip any epochs
    if (index != 0) {
        assert current_epoch = previous_epoch + 1;
    }

    // Ensure we only process batches using the predetermined committee hash
    // This is important to ensure we dont batch epochs that use an unknown committee
    assert epoch_batch.committee_hash.low = epoch_output[5];
    assert epoch_batch.committee_hash.high = epoch_output[6];

    let epoch_output_hash = compute_batch_hash(epoch_output);
    assert [output_ptr + index] = epoch_output_hash;

    // If we have reached the last batch, return the ouput
    if(index + 1 == batch_len) {
        return (latest_batch_output=epoch_output);
    }

    // Otherwise, run the next batch
    return run_epoch_batches(epoch_batch=epoch_batch, index=index + 1, batch_len=batch_len, previous_epoch=current_epoch);
}

// The when batching, we want to compute one hash per epoch update.
// Since the epoch update circuit has 11 outputs, we can compute the hash of the 10 outputs.
func compute_batch_hash{
    poseidon_ptr: PoseidonBuiltin*,
}(batch_output: felt*) -> felt {
    let (batch_hash) = poseidon_hash_many(11, batch_output);
    return batch_hash;
}