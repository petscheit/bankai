%builtins range_check bitwise

from starkware.cairo.common.cairo_builtins import BitwiseBuiltin
from starkware.cairo.common.uint256 import Uint256
from starkware.cairo.common.alloc import alloc
from starkware.cairo.common.memcpy import memcpy
from starkware.cairo.common.memset import memset
from starkware.cairo.common.cairo_sha256.sha256_utils import finalize_sha256
from cairo.src.utils import felt_divmod, pow2alloc128

namespace SHA256 {
    func init() -> (sha256_ptr: felt*, sha256_ptr_start: felt*) {
        let (sha256_ptr: felt*) = alloc();
        let sha256_ptr_start = sha256_ptr;

        return (sha256_ptr=sha256_ptr, sha256_ptr_start=sha256_ptr_start);
    }

    func finalize{
        range_check_ptr,
        bitwise_ptr: BitwiseBuiltin*,
    }(sha256_start_ptr: felt*, sha256_end_ptr: felt*) {
        finalize_sha256(sha256_start_ptr, sha256_end_ptr);
        return ();
    }

    func hash_pair{
        range_check_ptr,
        sha256_ptr: felt*,
        pow2_array: felt*
    }(input: felt*) -> (output: felt*) {
        alloc_locals;
        let (output) = sha256(data=input, n_bytes=64);
        return (output=output);
    }

    func hash_bytes{
        range_check_ptr,
        sha256_ptr: felt*,
        pow2_array: felt*
    }(input: felt*, n_bytes: felt) -> (output: felt*) {
        alloc_locals;
        let (output) = sha256(data=input, n_bytes=n_bytes);
        return (output=output);
    }
}

namespace HashUtils {
    func chunk_pair{
        range_check_ptr,
        pow2_array: felt*,
    }(left: Uint256, right: Uint256) -> felt* {
        let (leafs: Uint256*) = alloc();
        assert leafs[0] = left;
        assert leafs[1] = right;

        let (output_ptr: felt*) = alloc();
        with output_ptr {
            chunk_leafs(leafs=leafs, leafs_len=2, index=0);
        }
        return output_ptr;
    }

    func chunk_leafs{
        range_check_ptr,
        pow2_array: felt*,
        output_ptr: felt*
    }(leafs: Uint256*, leafs_len: felt, index: felt) {
        if (index == leafs_len) {
            return ();
        }

        let leaf = [leafs];

        // Process left-high
        let (q0, r0) = felt_divmod(leaf.high, pow2_array[32]);
        let (q1, r1) = felt_divmod(q0, pow2_array[32]);
        let (q2, r2) = felt_divmod(q1, pow2_array[32]);
        let (q3, r3) = felt_divmod(q2, pow2_array[32]);
        assert [output_ptr] = r3;
        assert [output_ptr + 1] = r2;
        assert [output_ptr + 2] = r1;
        assert [output_ptr + 3] = r0;

        // Proccess left-low
        let (q4, r4) = felt_divmod(leaf.low, pow2_array[32]);
        let (q5, r5) = felt_divmod(q4, pow2_array[32]);
        let (q6, r6) = felt_divmod(q5, pow2_array[32]);
        let (q7, r7) = felt_divmod(q6, pow2_array[32]);
        assert [output_ptr + 4] = r7;
        assert [output_ptr + 5] = r6;
        assert [output_ptr + 6] = r5;
        assert [output_ptr + 7] = r4;

        tempvar output_ptr = output_ptr + 8;
        return chunk_leafs(leafs=leafs + Uint256.SIZE, leafs_len=leafs_len, index=index + 1);
    }

    func chunk_uint256{
        range_check_ptr,
        pow2_array: felt*
    }(leaf: Uint256) -> (output: felt*) {
        let (output: felt*) = alloc();

        // Process left-high
        let (q0, r0) = felt_divmod(leaf.high, pow2_array[32]);
        let (q1, r1) = felt_divmod(q0, pow2_array[32]);
        let (q2, r2) = felt_divmod(q1, pow2_array[32]);
        let (q3, r3) = felt_divmod(q2, pow2_array[32]);
        assert [output] = r3;
        assert [output + 1] = r2;
        assert [output + 2] = r1;
        assert [output + 3] = r0;

        // Proccess left-low
        let (q4, r4) = felt_divmod(leaf.low, pow2_array[32]);
        let (q5, r5) = felt_divmod(q4, pow2_array[32]);
        let (q6, r6) = felt_divmod(q5, pow2_array[32]);
        let (q7, r7) = felt_divmod(q6, pow2_array[32]);
        assert [output + 4] = r7;
        assert [output + 5] = r6;
        assert [output + 6] = r5;
        assert [output + 7] = r4;

        return (output=output);
    }

    func chunks_to_uint256{pow2_array: felt*}(output: felt*) -> Uint256 {
        let low = [output + 4] * pow2_array[96] + [output + 5] * pow2_array[64] + [output + 6] * pow2_array[32] + [output + 7];
        let high = [output] * pow2_array[96] + [output + 1] * pow2_array[64] + [output + 2] * pow2_array[32] + [output + 3];
        return (Uint256(low=low, high=high));
    }
}

const SHA256_INPUT_CHUNK_SIZE_FELTS = 16;
const SHA256_STATE_SIZE_FELTS = 8;

// Hash an arbitrary length of bytes. Input must be BE 32bit chunks
func sha256{range_check_ptr, pow2_array: felt*, sha256_ptr: felt*}(data: felt*, n_bytes: felt) -> (output: felt*) {
    alloc_locals;

    // Maximum bytes_len is 2^32 - 1. This simplifies the padding calculation.
    assert [range_check_ptr] = pow2_array[32] - n_bytes;
    let range_check_ptr = range_check_ptr + 1;

    // Set the initial input state to IV.
    assert sha256_ptr[16] = 0x6A09E667;
    assert sha256_ptr[17] = 0xBB67AE85;
    assert sha256_ptr[18] = 0x3C6EF372;
    assert sha256_ptr[19] = 0xA54FF53A;
    assert sha256_ptr[20] = 0x510E527F;
    assert sha256_ptr[21] = 0x9B05688C;
    assert sha256_ptr[22] = 0x1F83D9AB;
    assert sha256_ptr[23] = 0x5BE0CD19;

    sha256_inner(data=data, n_bytes=n_bytes, remaining_bytes=n_bytes);

    let output = sha256_ptr;
    let sha256_ptr = sha256_ptr + SHA256_STATE_SIZE_FELTS;

    return (output=output);
}

func sha256_inner{range_check_ptr, pow2_array: felt*, sha256_ptr: felt*}(data: felt*, n_bytes: felt, remaining_bytes: felt) {
    alloc_locals;

    // If we have > 64 bytes input, we need at least two blocks for the message alone (without padding)
    let (additional_message_blocks, _) = felt_divmod(remaining_bytes, 64);
    if (additional_message_blocks == 0) {    
        let (n_full_words, local len_last_word) = felt_divmod(remaining_bytes, 4);

        // write the full input words to the sha256_ptr
        memcpy(dst=sha256_ptr, src=data, len=n_full_words);
        // compute the last word and write it to the sha256_ptr
        if (len_last_word != 0) {
            // if the last word is not a full word, we need to left-shift it
            let left_shift = pow2_array[(4 - len_last_word) * 8];
            assert sha256_ptr[n_full_words] = data[n_full_words] * left_shift + left_shift / 2;
        } else {
            // if the last word is a full word, we just append binary 1
            assert sha256_ptr[n_full_words] = 0x80000000;
        }

        // If the msg >= 56 bytes, we need two blocks
        let (required_two_blocks, _) = felt_divmod(remaining_bytes, 56);
        if (required_two_blocks == 0) {
            // msg.len <= 55 - Finalize hashing
            memset(dst=sha256_ptr + n_full_words + 1, value=0, n=14 - n_full_words);
            // append binary length
            assert sha256_ptr[15] = n_bytes * 8;
            _sha256_chunk(); // fill outputs

            tempvar sha256_ptr = sha256_ptr + SHA256_INPUT_CHUNK_SIZE_FELTS + SHA256_STATE_SIZE_FELTS;
            return ();
        } else {
            // 55 < msg.len < 64 -> We need two more blocks
            
            // Fill current block with required padding
            memset(dst=sha256_ptr + n_full_words + 1, value=0, n=15 - n_full_words); 
            _sha256_chunk(); // fill outputs
            tempvar sha256_ptr = sha256_ptr + SHA256_INPUT_CHUNK_SIZE_FELTS + SHA256_STATE_SIZE_FELTS;

            // write the output to the state of the next block
            memcpy(dst=sha256_ptr + 24, src=sha256_ptr, len=8);
            tempvar sha256_ptr = sha256_ptr + SHA256_STATE_SIZE_FELTS;

            // Fill last block with padding and binary length
            memset(dst=sha256_ptr, value=0, n=15);
            assert sha256_ptr[15] = n_bytes * 8;

            _sha256_chunk(); // fill outputs
            tempvar sha256_ptr = sha256_ptr + SHA256_INPUT_CHUNK_SIZE_FELTS + SHA256_STATE_SIZE_FELTS;

            return ();

        }
    } else {
        // otherwise we fill the entire block with our input
        memcpy(dst=sha256_ptr, src=data, len=16);
        _sha256_chunk(); // fill outputs
        tempvar sha256_ptr = sha256_ptr + SHA256_INPUT_CHUNK_SIZE_FELTS + SHA256_STATE_SIZE_FELTS;
        
        // copy output to the state of the next block
        memcpy(dst=sha256_ptr + 24, src=sha256_ptr, len=8);
        tempvar sha256_ptr = sha256_ptr + SHA256_STATE_SIZE_FELTS;

        return sha256_inner(data=data + 16, n_bytes=n_bytes, remaining_bytes=remaining_bytes - 64);
    }
}

// Computes the sha256 hash of the input chunk from `message` to `message + SHA256_INPUT_CHUNK_SIZE_FELTS`
func _sha256_chunk{range_check_ptr, sha256_ptr: felt*}() {
    let message = sha256_ptr;
    let state = sha256_ptr + SHA256_INPUT_CHUNK_SIZE_FELTS;
    let output = state + SHA256_STATE_SIZE_FELTS;

    %{
        from starkware.cairo.common.cairo_sha256.sha256_utils import (
            compute_message_schedule, sha2_compress_function)

        _sha256_input_chunk_size_felts = int(ids.SHA256_INPUT_CHUNK_SIZE_FELTS)
        assert 0 <= _sha256_input_chunk_size_felts < 100
        _sha256_state_size_felts = int(ids.SHA256_STATE_SIZE_FELTS)
        assert 0 <= _sha256_state_size_felts < 100
        w = compute_message_schedule(memory.get_range(
            ids.message, _sha256_input_chunk_size_felts))
        new_state = sha2_compress_function(memory.get_range(ids.state, _sha256_state_size_felts), w)
        segments.write_arg(ids.output, new_state)
    %}
    return ();
}


func main{range_check_ptr, bitwise_ptr: BitwiseBuiltin*}() {
    alloc_locals;

    let (sha256_ptr, sha256_ptr_start) = SHA256.init();
    let (pow2_array) = pow2alloc128();
    local length: felt;

    %{
        import random
        import os

        def generate_hex_array(N):
            hex_array = [os.urandom(i).hex() for i in range(1, N + 1)]
            return hex_array

        preimages = generate_hex_array(150)
        print(preimages)

        ids.length = len(preimages)
    %}

    with sha256_ptr, pow2_array {
        run_test(index=length - 1);
    }

    SHA256.finalize(sha256_start_ptr=sha256_ptr_start, sha256_end_ptr=sha256_ptr);

    return ();
}

func run_test{range_check_ptr, bitwise_ptr: BitwiseBuiltin*, sha256_ptr: felt*, pow2_array: felt*}(
    index: felt
) {
    alloc_locals;

    if (index == 0) {
        return ();
    }

    let (input: felt*) = alloc();
    local n_bytes: felt;
    local expected: Uint256;
    %{
        from garaga.hints.io import bigint_split
        import math
        import hashlib

        def hex_to_chunks_32(hex_string: str):
            # Remove '0x' prefix if present
            if hex_string.startswith(('0x', '0X')):
                hex_string = hex_string[2:]

            # if we have an odd number of characters, prepend a 0
            if len(hex_string) % 2 == 1:
                hex_string = '0' + hex_string

            # Now split into 8-character (32-bit) chunks
            chunks = [int(hex_string[i:i+8], 16) for i in range(0, len(hex_string), 8)]
            return chunks

        preimage = int(preimages[ids.index], 16)
        n_bytes = (preimage.bit_length() + 7) // 8
        ids.n_bytes = n_bytes
        print(n_bytes)
        print(math.ceil(n_bytes / 4))
        expected = hashlib.sha256(preimage.to_bytes(length=ids.n_bytes, byteorder='big')).hexdigest()
        ids.expected.high, ids.expected.low = divmod(int(expected, 16), 2**128)

        print([hex(x) for x in hex_to_chunks_32(hex(preimage))])

        chunks = bigint_split(preimage, math.ceil(n_bytes / 4), 2**32)
        print([hex(x) for x in chunks])
        segments.write_arg(ids.input, chunks)
    %}

    with sha256_ptr, pow2_array {
        let (output) = sha256(data=input, n_bytes=n_bytes);
        let hash = HashUtils.chunks_to_uint256(output=output);
        assert hash.high = expected.high;
        assert hash.low = expected.low;
    }

    return run_test(index=index - 1);
}