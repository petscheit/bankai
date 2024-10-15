from starkware.cairo.common.cairo_builtins import BitwiseBuiltin, UInt384, ModBuiltin
from starkware.cairo.common.alloc import alloc
from starkware.cairo.common.memcpy import memcpy
from starkware.cairo.common.uint256 import Uint256
from starkware.cairo.common.bitwise import bitwise_xor

from cairo.packages.garaga_zero.src.basic_field_ops import u512_mod_p
from cairo.packages.garaga_zero.src.definitions import get_P
from cairo.src.utils import felt_divmod
from cairo.src.sha256 import SHA256, HashUtils

// HashToField functionality, using SHA256 and 32-byte messages
// DST is "BLS_SIG_BLS12381G2_XMD:SHA-256_SSWU_RO_NUL_"
namespace HashToField32 {
    const B_IN_BYTES = 32; // hash function output size
    const B_IN_FELTS = 8; // 32 bytes, require 8 chunks
    const Z_PAD_LEN = B_IN_FELTS * 2;
    const BYTES_PER_CHUNK = 4;
    const CURVE_M = 2; // extension degree of F
    const CURVE_K = 128; // security level
    const CURVE_L = 64; // ceil((CURVE.P.bitlength + CURVE_K) / 8)
    const CURVE_L_IN_FELTS = 16; // 64 bits, require 16 chunks

    // Returns a zero-padded array of length CHUNKS_PER_BLOCK * 2
    func Z_PAD() -> felt* {
        let (z_pad: felt*) = alloc();
        assert [z_pad] = 0;
        assert [z_pad + 1] = 0;
        assert [z_pad + 2] = 0;
        assert [z_pad + 3] = 0;
        assert [z_pad + 4] = 0;
        assert [z_pad + 5] = 0;
        assert [z_pad + 6] = 0;
        assert [z_pad + 7] = 0;
        assert [z_pad + 8] = 0;
        assert [z_pad + 9] = 0;
        assert [z_pad + 10] = 0;
        assert [z_pad + 11] = 0;
        assert [z_pad + 12] = 0;
        assert [z_pad + 13] = 0;
        assert [z_pad + 14] = 0;
        assert [z_pad + 15] = 0;
        
        return z_pad;
    }

    // Returns the concatenation of 0x01, DST, and DST length
    func ONE_DST_PRIME() -> felt* {
        let (one_dst: felt*) = alloc();
        assert [one_dst] = 0x01424C53;
        assert [one_dst + 1] = 0x5F534947;
        assert [one_dst + 2] = 0x5F424C53;
        assert [one_dst + 3] = 0x31323338;
        assert [one_dst + 4] = 0x3147325F;
        assert [one_dst + 5] = 0x584D443A;
        assert [one_dst + 6] = 0x5348412D;
        assert [one_dst + 7] = 0x3235365F;
        assert [one_dst + 8] = 0x53535755;
        assert [one_dst + 9] = 0x5F524F5F;
        assert [one_dst + 10] = 0x4E554C5F;
        assert [one_dst + 11] = 0x2B;

        return one_dst;
    }

    // Expands a message according to the algorithm specified in:
    // https://datatracker.ietf.org/doc/html/draft-irtf-cfrg-hash-to-curve-11#section-5.4.1
    // Current implementation only supports 32-byte messages.
    // Parameters:
    // - msg: the message to expand, in big-endian 4-byte chunks
    // - msg_bytes_len: the length of the message in bytes
    // - n_bytes: the number of bytes to output
    //
    // Returns:
    // - result: the expanded message
    func expand_msg_xmd{
        range_check_ptr,
        bitwise_ptr: BitwiseBuiltin*,
        sha256_ptr: felt*,
        pow2_array: felt*
    }(msg: felt*, msg_bytes_len: felt, n_bytes: felt) -> (result: felt*){
        alloc_locals;

        // for now we only support 32 bytes messages. Some smaller changes are needed to support other msg lengths
        assert msg_bytes_len = 32;

        let (q, r) = felt_divmod(n_bytes, 32);
        local ell: felt;
        if (r == 0) {
            ell = q;
        } else {
            ell = q + 1;
        }

        with_attr error_message("INVALID_XMD_LENGTH") {
            assert [range_check_ptr] = 255 - ell;
            tempvar range_check_ptr = range_check_ptr + 1;
        }

        // Prepare the initial hash input:
        // Z_pad || msg || N_BYTES ||  0x0 || DST || len(DST)
        let msg_hash_train = Z_PAD();
        memcpy(dst=msg_hash_train + Z_PAD_LEN, src=msg, len=8);

        // Append other required values (DST and lengths)
        assert [msg_hash_train + 24] = 0x01000042; // to_bytes(n_bytes, 2) + 0x0 + DST (starts at 42)
        assert [msg_hash_train + 25] = 0x4C535F53;
        assert [msg_hash_train + 26] = 0x49475F42;
        assert [msg_hash_train + 27] = 0x4C533132;
        assert [msg_hash_train + 28] = 0x33383147; 
        assert [msg_hash_train + 29] = 0x325F584D;
        assert [msg_hash_train + 30] = 0x443A5348;
        assert [msg_hash_train + 31] = 0x412D3235;
        assert [msg_hash_train + 32] = 0x365F5353;
        assert [msg_hash_train + 33] = 0x57555F52;
        assert [msg_hash_train + 34] = 0x4F5F4E55;
        assert [msg_hash_train + 35] = 0x4C5F2B; // DST + DST.len

        // Compute the initial hash (b_0)
        let (msg_hash) = SHA256.hash_bytes(msg_hash_train, 111 + msg_bytes_len); // 64b z_pad + msg_bytes_len + 2b block_size, 0x0 ++ 43b dst + 1b dst_len

        // Prepare input for the first block hash (b_1)
        let (hash_args: felt*) = alloc();
        memcpy(dst=hash_args, src=msg_hash, len=B_IN_FELTS);
        let one_dst_prime = ONE_DST_PRIME();
        memcpy(dst=hash_args + 8, src=one_dst_prime, len=12);

        // Compute the first block hash (b_1)
        let (hash_1) = SHA256.hash_bytes(hash_args, 77); // 32b msg + 1b 0x1 + 43b dst + 1b dst_len

        // Create hash_train and copy first hash. The hash_train contains all 
        let (hash_train: felt*) = alloc();
        memcpy(dst=hash_train, src=hash_1, len=B_IN_FELTS);

        with one_dst_prime {
            expand_msg_xmd_inner(msg_hash=msg_hash, hash_train=hash_train, ell=ell, index=0);
        }

        // Copy the result. Potentially remove this
        let (result: felt*) = alloc();
        memcpy(dst=result, src=hash_train, len=n_bytes / 4);

        return (result=result);

    }
    
    // Inner recursive function for expand_msg_xmd
    func expand_msg_xmd_inner{
        range_check_ptr,
        bitwise_ptr: BitwiseBuiltin*,
        sha256_ptr: felt*,
        pow2_array: felt*,
        one_dst_prime: felt*
    }(msg_hash: felt*, hash_train: felt*, ell: felt, index: felt) {
        alloc_locals;

        if(index == ell){
            return ();
        }

        // XOR the initial hash (b_0) with the previous block hash
        let xored = _xor_hash_segments(msg_hash, hash_train + index * B_IN_FELTS);

        // Prepare the input for the next block hash:
        // (b_0 XOR b_i) || i+1 || DST || len(DST)
        assert [xored + 8] = [one_dst_prime] + 0x01000000 * (index + 1);
        memcpy(dst=xored + 9, src=one_dst_prime + 1, len=11);

        // Compute the next block hash
        let (hash) = SHA256.hash_bytes(xored, 77);
        // Store the new block hash in the output array
        memcpy(dst=hash_train + (index + 1) * B_IN_FELTS, src=hash, len=B_IN_FELTS);

        return expand_msg_xmd_inner(msg_hash=msg_hash, hash_train=hash_train, ell=ell, index=index+1);
    }

    // Hashes a message to a field element
    // Implementation of algorithm from:
    // https://datatracker.ietf.org/doc/html/draft-irtf-cfrg-hash-to-curve-11#section-5.3
    //
    // Parameters:
    // - msg: a byte string containing the message to hash, chunked in big-endian 4-bytes
    // - msg_bytes_len: length of the message in bytes
    // - count: the number of field elements to output
    //
    // Returns:
    // - fields: an array of field elements [u_0, ..., u_(count - 1)]
    func hash_to_field{
        range_check_ptr,
        bitwise_ptr: BitwiseBuiltin*,
        range_check96_ptr: felt*,
        add_mod_ptr: ModBuiltin*,
        mul_mod_ptr: ModBuiltin*,
        sha256_ptr: felt*,
        pow2_array: felt*
    }(msg: felt*, msg_bytes_len: felt, count: felt) -> (fields: UInt384**){
        alloc_locals;

        let n_bytes = count * CURVE_M * CURVE_L;
        let (result) = expand_msg_xmd(msg=msg, msg_bytes_len=msg_bytes_len, n_bytes=n_bytes);
        let (result_fields: UInt384**) = alloc();
        with result_fields {
            hash_to_field_inner(expanded_msg=result, count=count, index=0);
        }

        return (fields=result_fields);
    }

    // Inner recursive function for hash_to_field
    // Processes each count required in the hash_to_field function
    func hash_to_field_inner{
        range_check96_ptr: felt*,
        add_mod_ptr: ModBuiltin*,
        mul_mod_ptr: ModBuiltin*,
        pow2_array: felt*,
        result_fields: UInt384**,
    }(expanded_msg: felt*, count: felt, index: felt) {
        alloc_locals;

        if(count == index){
            return ();
        }
        
        let offset = index * CURVE_L_IN_FELTS * CURVE_M;
        let (fields: UInt384*) = alloc();
        with fields {
            hash_to_field_inner_inner(expanded_msg=expanded_msg, count_index=index, degree_index=0, offset=offset);
        }
        assert result_fields[index] = fields;

        return hash_to_field_inner(expanded_msg=expanded_msg, count=count, index=index+1);
    }

    // Innermost recursive function for hash_to_field
    // Runs CURVE_M times (degree of the extension field)
    func hash_to_field_inner_inner{
        range_check96_ptr: felt*,
        add_mod_ptr: ModBuiltin*,
        mul_mod_ptr: ModBuiltin*,
        pow2_array: felt*,
        fields: UInt384*
    }(expanded_msg: felt*, count_index: felt, degree_index: felt, offset: felt) {
        if (degree_index == CURVE_M){
            return ();
        }

        let (result) = _u512_mod_p(expanded_msg + offset);
        // %{ print(hex(ids.result.d3 * 2**288 + ids.result.d2 * 2**192 + ids.result.d1 * 2**96 + ids.result.d0)) %}
        assert fields[degree_index] = result;

        return hash_to_field_inner_inner(expanded_msg=expanded_msg, count_index=count_index, degree_index=degree_index+1, offset=offset + CURVE_L_IN_FELTS);
    }

    // Converts a 512-bit byte array to UInt384 and calls u512_mod_p
    func _u512_mod_p{
        range_check96_ptr: felt*,
        add_mod_ptr: ModBuiltin*,
        mul_mod_ptr: ModBuiltin*,
        pow2_array: felt*
    }(value: felt*) -> (result: UInt384){
        let (p: UInt384) = get_P(1);

        let (result) = u512_mod_p(
            low=(v0=value[8], v1=value[9], v2=value[10], v3=value[11], v4=value[12], v5=value[13], v6=value[14], v7=value[15]), 
            high=(v0=value[0], v1=value[1], v2=value[2], v3=value[3], v4=value[4], v5=value[5], v6=value[6], v7=value[7]),
            p=p
        );

        return (result=result);
    }

    // XORs two 256-bit hashes
    func _xor_hash_segments{
        range_check_ptr,
        bitwise_ptr: BitwiseBuiltin*,
        pow2_array: felt*
    }(hash_a: felt*, hash_b: felt*) -> felt* {
        alloc_locals;

        let hash_a_uint = HashUtils.chunks_to_uint256(hash_a);
        let hash_b_uint = HashUtils.chunks_to_uint256(hash_b);

        let (result_low) = bitwise_xor(hash_a_uint.low, hash_b_uint.low);
        let (result_high) = bitwise_xor(hash_a_uint.high, hash_b_uint.high);

        let (chunks) = HashUtils.chunk_uint256(Uint256(low=result_low, high=result_high));

        return chunks;
    }
}
