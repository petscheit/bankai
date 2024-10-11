// %builtins range_check bitwise

from starkware.cairo.common.cairo_builtins import BitwiseBuiltin
from starkware.cairo.common.uint256 import Uint256
from starkware.cairo.common.alloc import alloc
from starkware.cairo.common.cairo_sha256.sha256_utils import finalize_sha256
from starkware.cairo.common.sha256_state import (
    Sha256ProcessBlock,
    Sha256State,
    Sha256Input,
)

from cairo.src.utils import felt_divmod, pow2alloc128


namespace SHA256 {
    func init() -> (sha256_ptr: felt*, sha256_ptr_start: felt*) {
        let (sha256_ptr: felt*) = alloc();
        let sha256_ptr_start = sha256_ptr;

        return (sha256_ptr=sha256_ptr, sha256_ptr_start=sha256_ptr_start);
    }

    func finalize{
        range_check_ptr,
        bitwise_ptr: BitwiseBuiltin*
    }(sha256_start_ptr: felt*, sha256_end_ptr: felt*) {
        finalize_sha256(sha256_start_ptr, sha256_end_ptr);
        return ();
    }

    func hash_pair{
        range_check_ptr,
        sha256_ptr: felt*
    }(input: felt*) -> (output: felt*) {
        alloc_locals;
        let output = compute_sha256(data=input, n_bytes=64);
        return (output=output);
    }

    func hash_bytes{
        range_check_ptr,
        sha256_ptr: felt*
    }(input: felt*, n_bytes: felt) -> (output: felt*) {
        alloc_locals;
        let output = compute_sha256(data=input, n_bytes=n_bytes);
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

    func chunks_to_uint256{pow2_array: felt*}(output: felt*) -> Uint256 {
        let low = [output + 4] * pow2_array[96] + [output + 5] * pow2_array[64] + [output + 6] * pow2_array[32] + [output + 7];
        let high = [output] * pow2_array[96] + [output + 1] * pow2_array[64] + [output + 2] * pow2_array[32] + [output + 3];
        return (Uint256(low=low, high=high));
    }
}

// --- FUNCTIONS BELOW TAKEN FROM https://github.com/ZeroSync/ZeroSync ---
// Thanks for the implementation!


from starkware.cairo.common.math_cmp import is_le
from starkware.cairo.common.math import assert_nn_le, unsigned_div_rem
from starkware.cairo.common.memcpy import memcpy
from starkware.cairo.common.memset import memset
from cairo.src.utils import pow2


const SHA256_INPUT_CHUNK_SIZE_FELTS = 16;
const SHA256_INPUT_CHUNK_SIZE_BYTES = 64;
// A 256-bit hash is represented as an array of 8 x Uint32
const SHA256_STATE_SIZE_FELTS = 8;
// Each instance consists of 16 words of message, 8 words for the input state and 8 words
// for the output state.
const SHA256_INSTANCE_SIZE = SHA256_INPUT_CHUNK_SIZE_FELTS + 2 * SHA256_STATE_SIZE_FELTS;


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

    sha256_inner_new(data=data, n_bytes=n_bytes, remaining_bytes=n_bytes);

    let output = sha256_ptr;
    let sha256_ptr = sha256_ptr + SHA256_STATE_SIZE_FELTS;

    return (output=output);
}

func sha256_inner_new{range_check_ptr, pow2_array: felt*, sha256_ptr: felt*}(data: felt*, n_bytes: felt, remaining_bytes: felt) {
    alloc_locals;

    %{
        print("n_bytes", ids.n_bytes)
        print("remaining_bytes", ids.remaining_bytes)
    %}

    // If we have >= 64 bytes input, we need at least one full block for the messagfe
    let (additional_message_blocks, _) = felt_divmod(remaining_bytes, 64);
    %{ print("aaditional_msg_blocks:", ids.additional_message_blocks) %}
    if (additional_message_blocks == 0) {    
        let (n_full_words, local len_last_word) = felt_divmod(remaining_bytes, 4);

        %{
            print("n_full_words:", ids.n_full_words)
            print("len_last_word:", ids.len_last_word)
        %}


        // write the full input words to the sha256_ptr
        memcpy(dst=sha256_ptr, src=data, len=n_full_words);
        // compute the last word and write it to the sha256_ptr
        if (len_last_word != 0) {
            // if the last word is not a full word, we need to left-shift it
            let left_shift = pow2_array[(4 - len_last_word) * 8];
            // shift and append binary 1 right after the word ends
            assert sha256_ptr[n_full_words] = data[n_full_words] * left_shift + left_shift / 2;
        } else {
            // if the last word is a full word, we just append binary 1
            assert sha256_ptr[n_full_words] = 0x80000000;
        }

        %{ print("padded_last_word", hex(memory[ids.sha256_ptr + ids.n_full_words])) %}

        // If the msg >= 56 bytes, we need two blocks
        let (required_two_blocks, _) = felt_divmod(remaining_bytes, 56);
        %{ print("required_two_blocks", ids.required_two_blocks) %}
        if (required_two_blocks == 0) {
            // msg.len <= 55 - Finalize hashing
            memset(dst=sha256_ptr + n_full_words + 1, value=0, n=14 - n_full_words);
            // append binary length
            assert sha256_ptr[15] = n_bytes * 8;

            tempvar sha256_ptr = sha256_ptr;
            let message = sha256_ptr;
            let state = sha256_ptr + SHA256_INPUT_CHUNK_SIZE_FELTS;
            let output = state + SHA256_STATE_SIZE_FELTS;

            // write outputs
            _sha256_chunk{message=message, state=state, output=output}();

            let sha256_ptr = sha256_ptr + SHA256_INPUT_CHUNK_SIZE_FELTS + SHA256_STATE_SIZE_FELTS;

            return ();
        } else {
            // 55 < msg.len < 64
            memset(dst=sha256_ptr + n_full_words + 1, value=0, n=15 - n_full_words); // fill all the way until the end (index 15) ith 0 padding

            let message = sha256_ptr;
            let state = sha256_ptr + SHA256_INPUT_CHUNK_SIZE_FELTS;
            let output = state + SHA256_STATE_SIZE_FELTS;
            _sha256_chunk{message=message, state=state, output=output}();

            // move sha256_ptr to output start
            let sha256_ptr = sha256_ptr + SHA256_INPUT_CHUNK_SIZE_FELTS + SHA256_STATE_SIZE_FELTS;
            // write the output to the state of the next block
            memcpy(dst=sha256_ptr + 24, src=sha256_ptr, len=8);
            let sha256_ptr = sha256_ptr + SHA256_STATE_SIZE_FELTS;

            // write the last round with padding
            memset(dst=sha256_ptr, value=0, n=15);
            assert sha256_ptr[15] = n_bytes * 8;

            tempvar sha256_ptr = sha256_ptr;
            let message = sha256_ptr;
            let state = sha256_ptr + SHA256_INPUT_CHUNK_SIZE_FELTS;
            let output = state + SHA256_STATE_SIZE_FELTS;

            // write outputs
            _sha256_chunk{message=message, state=state, output=output}();

            let sha256_ptr = sha256_ptr + SHA256_INPUT_CHUNK_SIZE_FELTS + SHA256_STATE_SIZE_FELTS;
        
            return ();

        
        }
        

        
        
        // _add_zero_padding(index=n_full_words + 1, number_of_zero_chunks=14 - n_full_words);
        // If the message is <= 55 bytes, we fit everything one sha256 block
        // add zero padding, until we reach the last element of the input.
        // we subtract from 14, as we added one word above, and another is reserved for the length
   
        // append the binary length of the message
    } else {
        // otherwise we fill the entire block with our input
        memcpy(dst=sha256_ptr, src=data, len=16);

        let message = sha256_ptr;
        let state = sha256_ptr + SHA256_INPUT_CHUNK_SIZE_FELTS;
        let output = state + SHA256_STATE_SIZE_FELTS;
        _sha256_chunk{message=message, state=state, output=output}();

        // move sha256_ptr to output start
        let sha256_ptr = sha256_ptr + SHA256_INPUT_CHUNK_SIZE_FELTS + SHA256_STATE_SIZE_FELTS;
        // write the output to the state of the next block
        memcpy(dst=sha256_ptr + 24, src=sha256_ptr, len=8);

        let sha256_ptr = sha256_ptr + SHA256_STATE_SIZE_FELTS;
        return sha256_inner_new(data=data + 16, n_bytes=n_bytes, remaining_bytes=remaining_bytes - 64);
    }
}

func _add_zero_padding{sha256_ptr: felt*}(index: felt, number_of_zero_chunks: felt) {
    %{ print("Padding zero chunks: ", ids.number_of_zero_chunks) %}
    tempvar i = 0;

    loop:
    let i = [ap - 1];

    %{ memory[ap] = 1 if ids.i == ids.number_of_zero_chunks else 0 %}
    jmp end_loop if [ap] != 0, ap++;
    assert sha256_ptr[index + i] = 0;

    [ap] = i + 1, ap++;
    jmp loop;

    end_loop:
    %{ print("Done padding zero chunks") %}
    return ();
}

// 111011101110111011101110
// 11101110111011101110111010000000

// 0001000000000000

// Computes SHA256 of 'input'. Inputs of arbitrary length are supported.
// To use this function, split the input into (up to) 14 words of 32 bits (big endian).
// For example, to compute sha256('Hello world'), use:
//   input = [1214606444, 1864398703, 1919706112]
// where:
//   1214606444 == int.from_bytes(b'Hell', 'big')
//   1864398703 == int.from_bytes(b'o wo', 'big')
//   1919706112 == int.from_bytes(b'rld\x00', 'big')  # Note the '\x00' padding.
//
// block layout:
// 0 - 15: Message
// 16 - 23: Input State
// 24 - 32: Output
//
// output is an array of 8 32-bit words (big endian).
//
// Note: You must call finalize_sha2() at the end of the program. Otherwise, this function
// is not sound and a malicious prover may return a wrong result.
// Note: the interface of this function may change in the future.
func compute_sha256{range_check_ptr, sha256_ptr: felt*}(data: felt*, n_bytes: felt) -> felt* {
    alloc_locals;

    // Set the initial input state to IV.
    assert sha256_ptr[16] = 0x6A09E667;
    assert sha256_ptr[17] = 0xBB67AE85;
    assert sha256_ptr[18] = 0x3C6EF372;
    assert sha256_ptr[19] = 0xA54FF53A;
    assert sha256_ptr[20] = 0x510E527F;
    assert sha256_ptr[21] = 0x9B05688C;
    assert sha256_ptr[22] = 0x1F83D9AB;
    assert sha256_ptr[23] = 0x5BE0CD19;

    sha256_inner(data=data, n_bytes=n_bytes, total_bytes=n_bytes);

    // Set `output` to the start of the final state.
    let output = sha256_ptr;
    // Set `sha256_ptr` to the end of the output state.
    let sha256_ptr = sha256_ptr + SHA256_STATE_SIZE_FELTS;
    return output;
}

// Computes the sha256 hash of the input chunk from `message` to `message + SHA256_INPUT_CHUNK_SIZE_FELTS`
func _sha256_chunk{range_check_ptr, message: felt*, state: felt*, output: felt*}() {
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

// Inner loop for sha256. `sha256_ptr` points to the start of the block.
func sha256_inner{range_check_ptr, sha256_ptr: felt*}(
    data: felt*, n_bytes: felt, total_bytes: felt
) {
    alloc_locals;

    let message = sha256_ptr;
    let state = sha256_ptr + SHA256_INPUT_CHUNK_SIZE_FELTS;
    let output = state + SHA256_STATE_SIZE_FELTS;

    let zero_bytes = is_le(n_bytes, 0);
    let zero_total_bytes = is_le(total_bytes, 0);

    // If the previous message block was full we are still missing "1" at the end of the message
    let (_, r_div_by_64) = unsigned_div_rem(total_bytes, 64);
    let missing_bit_one = is_le(r_div_by_64, 0);

    // This works for 0 total bytes too, because zero_chunk will be -1 and, therefore, not 0.
    let zero_chunk = zero_bytes - zero_total_bytes - missing_bit_one;

    let is_last_block = is_le(n_bytes, 55);
    if (is_last_block == 1) {
        _sha256_input(data, n_bytes, SHA256_INPUT_CHUNK_SIZE_FELTS - 2, zero_chunk);
        // Append the original message length at the end of the message block as a 64-bit big-endian integer.
        assert sha256_ptr[0] = 0;
        assert sha256_ptr[1] = total_bytes * 8;
        let sha256_ptr = sha256_ptr + 2;
        _sha256_chunk{message=message, state=state, output=output}();
        let sha256_ptr = sha256_ptr + SHA256_STATE_SIZE_FELTS;

        return ();
    }

    let (q, r) = unsigned_div_rem(n_bytes, SHA256_INPUT_CHUNK_SIZE_BYTES);
    let is_remainder_block = is_le(q, 0);
    if (is_remainder_block == 1) {
        _sha256_input(data, r, SHA256_INPUT_CHUNK_SIZE_FELTS, 0);
        _sha256_chunk{message=message, state=state, output=output}();

        let sha256_ptr = sha256_ptr + SHA256_STATE_SIZE_FELTS;
        memcpy(
            output + SHA256_STATE_SIZE_FELTS + SHA256_INPUT_CHUNK_SIZE_FELTS,
            output,
            SHA256_STATE_SIZE_FELTS,
        );
        let sha256_ptr = sha256_ptr + SHA256_STATE_SIZE_FELTS;

        return sha256_inner(data=data, n_bytes=n_bytes - r, total_bytes=total_bytes);
    } else {
        _sha256_input(data, SHA256_INPUT_CHUNK_SIZE_BYTES, SHA256_INPUT_CHUNK_SIZE_FELTS, 0);
        _sha256_chunk{message=message, state=state, output=output}();

        let sha256_ptr = sha256_ptr + SHA256_STATE_SIZE_FELTS;
        memcpy(
            output + SHA256_STATE_SIZE_FELTS + SHA256_INPUT_CHUNK_SIZE_FELTS,
            output,
            SHA256_STATE_SIZE_FELTS,
        );
        let sha256_ptr = sha256_ptr + SHA256_STATE_SIZE_FELTS;

        return sha256_inner(
            data=data + SHA256_INPUT_CHUNK_SIZE_FELTS,
            n_bytes=n_bytes - SHA256_INPUT_CHUNK_SIZE_BYTES,
            total_bytes=total_bytes,
        );
    }
}

// 1. Encode the input to binary using UTF-8 and append a single '1' to it.
// 2. Prepend that binary to the message block.
func _sha256_input{range_check_ptr, sha256_ptr: felt*}(
    input: felt*, n_bytes: felt, n_words: felt, pad_chunk: felt
) {
    alloc_locals;

    local full_word;
    %{ ids.full_word = int(ids.n_bytes >= 4) %}

    if (full_word != 0) {
        assert sha256_ptr[0] = input[0];
        let sha256_ptr = sha256_ptr + 1;
        return _sha256_input(
            input=input + 1, n_bytes=n_bytes - 4, n_words=n_words - 1, pad_chunk=pad_chunk
        );
    }

    if (n_words == 0) {
        return ();
    }

    if (n_bytes == 0 and pad_chunk == 1) {
        // Add zeros between the encoded message and the length integer so that the message block is a multiple of 512.
        memset(dst=sha256_ptr, value=0, n=n_words);
        let sha256_ptr = sha256_ptr + n_words;
        return ();
    }

    if (n_bytes == 0) {
        // This is the last input word, so we should add a byte '0x80' at the end and fill the rest with zeros.
        assert sha256_ptr[0] = 0x80000000;
        // Add zeros between the encoded message and the length integer so that the message block is a multiple of 512.
        memset(dst=sha256_ptr + 1, value=0, n=n_words - 1);
        let sha256_ptr = sha256_ptr + n_words;
        return ();
    }

    with_attr error_message("n_bytes is negative or greater than 3.") {
        assert_nn_le(n_bytes, 3);
    }
    let padding = pow2(8 * (3 - n_bytes));
    local range_check_ptr = range_check_ptr;

    assert sha256_ptr[0] = input[0] + padding * 0x80;

    memset(dst=sha256_ptr + 1, value=0, n=n_words - 1);
    let sha256_ptr = sha256_ptr + n_words;
    return ();
}

func main{
    range_check_ptr,
    bitwise_ptr: BitwiseBuiltin*
}() {

    alloc_locals;

    let (sha256_ptr, sha256_ptr_start) = SHA256.init();
    let (pow2_array) = pow2alloc128();

    let (input: felt*) = alloc();
    %{
        segments.write_arg(ids.input, [0x11111111,0x22222222,0x33333333,0x44444444,0x55555555,0x66666666,0x77777777,0x88888888,0x99999999,0xaaaaaaaa,0xbbbbbbbb,0xcccccccc,0xdddddddd,0xeeeeeeee, 0xffffffff, 0xffffffff, 0xffffffff])
        #segments.write_arg(ids.input, [0x22222222, 0x02])
    %}

    with sha256_ptr, pow2_array {
        let (output) = sha256(data=input, n_bytes=68);
        let hash = HashUtils.chunks_to_uint256(output=output);
    }
    %{ print("Hash: ", hex(ids.hash.high * 2**128 + ids.hash.low)) %}

    SHA256.finalize(sha256_start_ptr=sha256_ptr_start, sha256_end_ptr=sha256_ptr);

    return ();
}