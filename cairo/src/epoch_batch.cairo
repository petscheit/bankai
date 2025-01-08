%builtins output range_check bitwise poseidon range_check96 add_mod mul_mod

from starkware.cairo.common.cairo_builtins import PoseidonBuiltin, ModBuiltin, BitwiseBuiltin
from starkware.cairo.common.uint256 import Uint256
from starkware.cairo.common.alloc import alloc
from starkware.cairo.common.builtin_poseidon.poseidon import (
    poseidon_hash,
    poseidon_hash_many,
)
from cairo.src.epoch_update import run_epoch_update
from cairo.src.merkle import PoseidonMerkleTree
from cairo.src.utils import pow2alloc128
from sha import SHA256

func main{
    output_ptr: felt*,
    range_check_ptr,
    bitwise_ptr: BitwiseBuiltin*,
    poseidon_ptr: PoseidonBuiltin*,
    range_check96_ptr: felt*,
    add_mod_ptr: ModBuiltin*,
    mul_mod_ptr: ModBuiltin*,
} () {
    alloc_locals;

    local batch_len: felt;
    local committee_hash: Uint256;
    %{ 
        from cairo.py.utils import int_to_uint256

        ids.batch_len = len(program_input["epochs"]) 
        low, high = int_to_uint256(int(program_input["committee_hash"], 16))
        ids.committee_hash.low = low
        ids.committee_hash.high = high
    %}

    let (pow2_array) = pow2alloc128();
    let (sha256_ptr, sha256_ptr_start) = SHA256.init();
    
    with pow2_array, sha256_ptr {
        let (epoch_outputs: felt*) = alloc();
        run_epoch_batches{
            output_ptr=epoch_outputs,
        }(0, batch_len, committee_hash);
        
        // now we compute a merkle root of the epoch outputs
        let epoch_root = PoseidonMerkleTree.compute_root(epoch_outputs, batch_len);
        %{ print("computed batch root", hex(ids.epoch_root)) %}
    }

    assert [output_ptr] = epoch_root;
    assert [output_ptr + 1] = committee_hash.low;
    assert [output_ptr + 2] = committee_hash.high;

    tempvar output_ptr = output_ptr + 3;

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
}(index: felt, batch_len: felt, committee_hash: Uint256) {
    alloc_locals;

    if(index == batch_len) {
        return ();
    }

    %{ vm_enter_scope({'program_input': program_input["epochs"][ids.index]}) %}
    
    // Create a new output_ptr per batch
    let (epoch_output: felt*) = alloc();
    run_epoch_update{
        output_ptr=epoch_output,
    }();

    // set output_ptr to first output
    let epoch_output = epoch_output - 11;
    %{ vm_exit_scope() %}

    // Ensure we only process batches using the predetermined committee hash
    // This is important to ensure we dont batch epochs that use an unknown committee
    assert committee_hash.low = epoch_output[5];
    assert committee_hash.high = epoch_output[6];

    let epoch_output_hash = compute_batch_hash(epoch_output);
    assert [output_ptr + index] = epoch_output_hash;

    return run_epoch_batches(index=index + 1, batch_len=batch_len, committee_hash=committee_hash);
}

// The when batching, we want to compute one hash per epoch update.
// Since the epoch update circuit has 11 outputs, we can compute the hash of the 10 outputs.
func compute_batch_hash{
    poseidon_ptr: PoseidonBuiltin*,
}(batch_output: felt*) -> felt {
    let (batch_hash) = poseidon_hash_many(11, batch_output);
    return batch_hash;
}