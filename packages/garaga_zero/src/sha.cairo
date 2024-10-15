namespace SHA256 {
    // Initializes the SHA256 context by allocating memory for the SHA256 pointer
    // Returns:
    //   sha256_ptr: A pointer to the allocated memory for SHA256 operations
    //   sha256_ptr_start: The starting address of the allocated memory
    func init() -> (sha256_ptr: felt*, sha256_ptr_start: felt*) {
        let (sha256_ptr: felt*) = alloc();
        let sha256_ptr_start = sha256_ptr;

        return (sha256_ptr=sha256_ptr, sha256_ptr_start=sha256_ptr_start);
    }

    // Finalizes the SHA256 computation
    // This function should be called after all hash operations are complete
    // Parameters:
    //   sha256_start_ptr: The starting address of the SHA256 context
    //   sha256_end_ptr: The ending address of the SHA256 context
    func finalize{
        range_check_ptr,
        bitwise_ptr: BitwiseBuiltin*,
    }(sha256_start_ptr: felt*, sha256_end_ptr: felt*) {
        finalize_sha256(sha256_start_ptr, sha256_end_ptr);
        return ();
    }

    // Computes the SHA256 hash of a pair of 32-byte inputs (64 bytes total)
    // Parameters:
    //   input: A pointer to the 64-byte input data
    // Returns:
    //   output: A pointer to the resulting 32-byte hash
    func hash_pair{
        range_check_ptr,
        sha256_ptr: felt*,
        pow2_array: felt*
    }(input: felt*) -> (output: felt*) {
        alloc_locals;
        let (output) = sha256(data=input, n_bytes=64);
        return (output=output);
    }

    // Computes the SHA256 hash of an arbitrary number of bytes
    // Parameters:
    //   input: A pointer to the input data
    //   n_bytes: The number of bytes to hash
    // Returns:
    //   output: A pointer to the resulting 32-byte hash
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
    // HashUtils namespace provides utility functions for working with SHA256 hashes
    // and Uint256 values in the context of Merkle tree operations.

    // Chunks a pair of Uint256 values into a sequence of felts
    // Parameters:
    //   left: The left Uint256 value
    //   right: The right Uint256 value
    // Returns:
    //   A pointer to an array of felts representing the chunked pair
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

    // Recursively chunks an array of Uint256 values into felts
    // Parameters:
    //   leafs: A pointer to an array of Uint256 values
    //   leafs_len: The length of the leafs array
    //   index: The current index being processed
    // Implicit:
    //   output_ptr: A pointer to the output array of felts
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

    // Chunks a single Uint256 value into an array of 8 felts
    // Parameters:
    //   leaf: The Uint256 value to chunk
    // Returns:
    //   output: A pointer to an array of 8 felts representing the chunked Uint256
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

    // Converts an array of 8 felts back into a Uint256 value
    // Parameters:
    //   output: A pointer to an array of 8 felts
    // Returns:
    //   A Uint256 value reconstructed from the input felts
    func chunks_to_uint256{pow2_array: felt*}(output: felt*) -> Uint256 {
        let low = [output + 4] * pow2_array[96] + [output + 5] * pow2_array[64] + [output + 6] * pow2_array[32] + [output + 7];
        let high = [output] * pow2_array[96] + [output + 1] * pow2_array[64] + [output + 2] * pow2_array[32] + [output + 3];
        return (Uint256(low=low, high=high));
    }
}

const SHA256_INPUT_CHUNK_SIZE_FELTS = 16;
const SHA256_STATE_SIZE_FELTS = 8;

// Computes the SHA256 hash of an arbitrary length of bytes
// Parameters:
//   data: Pointer to the input data (must be in big-endian 32-bit chunks)
//   n_bytes: Number of bytes to hash
// Returns:
//   output: Pointer to the resulting 32-byte (256-bit) hash
// Implicit parameters:
//   range_check_ptr: Range check builtin pointer
//   pow2_array: Array of powers of 2
//   sha256_ptr: Pointer to the SHA256 state
func sha256{range_check_ptr, pow2_array: felt*, sha256_ptr: felt*}(data: felt*, n_bytes: felt) -> (output: felt*) {
    alloc_locals;

    // Ensure n_bytes is within the valid range (0 to 2^32 - 1)
    assert [range_check_ptr] = pow2_array[32] - n_bytes;
    let range_check_ptr = range_check_ptr + 1;

    // Initialize the SHA256 state with the standard initial values (IV)
    assert sha256_ptr[16] = 0x6A09E667;
    assert sha256_ptr[17] = 0xBB67AE85;
    assert sha256_ptr[18] = 0x3C6EF372;
    assert sha256_ptr[19] = 0xA54FF53A;
    assert sha256_ptr[20] = 0x510E527F;
    assert sha256_ptr[21] = 0x9B05688C;
    assert sha256_ptr[22] = 0x1F83D9AB;
    assert sha256_ptr[23] = 0x5BE0CD19;

    // Process the input data
    sha256_inner(data=data, n_bytes=n_bytes, remaining_bytes=n_bytes);

    // Set the output pointer and update sha256_ptr
    let output = sha256_ptr;
    let sha256_ptr = sha256_ptr + SHA256_STATE_SIZE_FELTS;

    return (output=output);
}

// Internal function to process SHA256 input data in chunks
// Parameters:
//   data: Pointer to the current position in the input data
//   n_bytes: Total number of bytes in the original input
//   remaining_bytes: Number of bytes left to process
// Implicit parameters:
//   range_check_ptr: Range check builtin pointer
//   pow2_array: Array of powers of 2
//   sha256_ptr: Pointer to the SHA256 state
func sha256_inner{range_check_ptr, pow2_array: felt*, sha256_ptr: felt*}(data: felt*, n_bytes: felt, remaining_bytes: felt) {
    alloc_locals;

    // Calculate the number of additional full 64-byte blocks needed
    let (additional_message_blocks, _) = felt_divmod(remaining_bytes, 64);
    
    if (additional_message_blocks == 0) {
        // This is the last block (possibly the only block)
        let (n_full_words, local len_last_word) = felt_divmod(remaining_bytes, 4);

        // Copy full 32-bit words to sha256_ptr
        memcpy(dst=sha256_ptr, src=data, len=n_full_words);
        
        // Handle the last partial word (if any)
        if (len_last_word != 0) {
            // Left-shift the last partial word and add padding
            let left_shift = pow2_array[(4 - len_last_word) * 8];
            assert sha256_ptr[n_full_words] = data[n_full_words] * left_shift + left_shift / 2;
        } else {
            // If the last word is complete, append the '1' bit as a new word
            assert sha256_ptr[n_full_words] = 0x80000000;
        }

        // Check if we need one or two blocks for padding
        let (required_two_blocks, _) = felt_divmod(remaining_bytes, 56);
        if (required_two_blocks == 0) {
            // Single block is sufficient (message length <= 55 bytes)
            memset(dst=sha256_ptr + n_full_words + 1, value=0, n=14 - n_full_words);
            assert sha256_ptr[15] = n_bytes * 8;  // Append message length in bits
            _sha256_chunk();  // Process the block

            tempvar sha256_ptr = sha256_ptr + SHA256_INPUT_CHUNK_SIZE_FELTS + SHA256_STATE_SIZE_FELTS;
            return ();
        } else {
            // Two blocks are needed (56 <= message length < 64 bytes)
            
            // Fill the first block with zeros
            memset(dst=sha256_ptr + n_full_words + 1, value=0, n=15 - n_full_words); 
            _sha256_chunk();  // Process the first block
            tempvar sha256_ptr = sha256_ptr + SHA256_INPUT_CHUNK_SIZE_FELTS + SHA256_STATE_SIZE_FELTS;

            // Prepare the second block
            memcpy(dst=sha256_ptr + 24, src=sha256_ptr, len=8);  // Copy the intermediate hash
            tempvar sha256_ptr = sha256_ptr + SHA256_STATE_SIZE_FELTS;

            // Fill the second block with padding and message length
            memset(dst=sha256_ptr, value=0, n=15);
            assert sha256_ptr[15] = n_bytes * 8;  // Append message length in bits
            _sha256_chunk();  // Process the second block
            tempvar sha256_ptr = sha256_ptr + SHA256_INPUT_CHUNK_SIZE_FELTS + SHA256_STATE_SIZE_FELTS;

            return ();
        }
    } else {
        // Process a full 64-byte block
        memcpy(dst=sha256_ptr, src=data, len=16);
        _sha256_chunk();  // Process the block
        tempvar sha256_ptr = sha256_ptr + SHA256_INPUT_CHUNK_SIZE_FELTS + SHA256_STATE_SIZE_FELTS;
        
        // Prepare for the next block
        memcpy(dst=sha256_ptr + 24, src=sha256_ptr, len=8);  // Copy the intermediate hash
        tempvar sha256_ptr = sha256_ptr + SHA256_STATE_SIZE_FELTS;

        // Recursively process the remaining data
        return sha256_inner(data=data + 16, n_bytes=n_bytes, remaining_bytes=remaining_bytes - 64);
    }
}

// Computes the SHA256 hash of a single 64-byte block
// Implicit parameters:
//   range_check_ptr: Range check builtin pointer
//   sha256_ptr: Pointer to the SHA256 state
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