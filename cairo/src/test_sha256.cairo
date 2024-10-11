%builtins range_check bitwise
from starkware.cairo.common.cairo_builtins import BitwiseBuiltin
from starkware.cairo.common.uint256 import Uint256
from starkware.cairo.common.alloc import alloc
from cairo.src.sha256 import SHA256, sha256, HashUtils
from cairo.src.utils import pow2alloc128

func main{
    range_check_ptr,
    bitwise_ptr: BitwiseBuiltin*
}() {

    alloc_locals;

    let (sha256_ptr, sha256_ptr_start) = SHA256.init();
    let (pow2_array) = pow2alloc128();
    local length: felt;

    %{

        import random

        preimages = [
            0x0,
            0x1,
            0x222,
            0x33333,
            0x444444,
            0x5555555,
            0x66666666,
            0x777777777,
            0x8888888888,
            0x99999999999,
            0xaaaaaaaaaaaa,
            0xbbbbbbbbbbbbbb,
            0xcccccccccccccccc,
            0xdddddddddddddddddd,
            0xeeeeeeeeeeeeeeeeeeee,
            0xffffffffffffffffffff,
            0x123456789abcdef0123456,
            0xfedcba9876543210fedcba98,
            0x0123456789abcdef0123456789,
            0xfedcba9876543210fedcba9876543210,
            0x0123456789abcdef0123456789abcdef,
            0xfedcba9876543210fedcba9876543210fedcba,
            0x0123456789abcdef0123456789abcdef01234567,
            0xfedcba9876543210fedcba9876543210fedcba9876,
            0x0123456789abcdef0123456789abcdef0123456789ab,
            0xfedcba9876543210fedcba9876543210fedcba9876543210,
            0x0123456789abcdef0123456789abcdef0123456789abcdef01,
            0xfedcba9876543210fedcba9876543210fedcba9876543210fedcba,
            0x0123456789abcdef0123456789abcdef0123456789abcdef0123456789,
            0xfedcba9876543210fedcba9876543210fedcba9876543210fedcba9876543210,
            0x0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef01,
            0xfedcba9876543210fedcba9876543210fedcba9876543210fedcba9876543210fedcba,
            0x0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789,
            0xfedcba9876543210fedcba9876543210fedcba9876543210fedcba9876543210fedcba9876543210,
            0x0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef01,
            0xfedcba9876543210fedcba9876543210fedcba9876543210fedcba9876543210fedcba9876543210fedcba,
            0xfedcba9876543210fedcba9876543210fedcba9876543210fedcba9876543210fedcba9876543210fefefefedcba,
            0x0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef,
            0xfedcba9876543210fedcba9876543210fedcba9876543210fedcba9876543210fedcba9876543210fedcba9876543210324234,
            0x111111112222222233333333444444445555555566666666777777778888888899999999aaaaaaaabbbbbbbbccccccccddddddddeeee,
            0x111111112222222233333333444444445555555566666666777777778888888899999999aaaaaaaabbbbbbbbccccccccddddddddeeeeee,
            0x111111112222222233333333444444445555555566666666777777778888888899999999aaaaaaaabbbbbbbbccccccccddddddddeeeeeee,
            0x111111112222222233333333444444445555555566666666777777778888888899999999aaaaaaaabbbbbbbbccccccccddddddddeeeeeeeaaaaaaaaaaaaabb,
            0x111111112222222233333333444444445555555566666666777777778888888899999999aaaaaaaabbbbbbbbccccccccddddddddeeeeeeeeffffffffffffff,
            0x111111112222222233333333444444445555555566666666777777778888888899999999aaaaaaaabbbbbbbbccccccccddddddddeeeeeeeeffffffffffffffff,
            0x111111112222222233333333444444445555555566666666777777778888888899999999aaaaaaaabbbbbbbbccccccccddddddddeeeeeeeeffffffffffffffffffffffff,
            0x111111112222222233333333444444445555555566666666777777778888888899999999aaaaaaaabbbbbbbbccccccccddddddddeeeeeeeeffffffffffffffffffffffffaa,
            0x111111112222222233333333444444445555555566666666777777778888888899999999aaaaaaaabbbbbbbbccccccccddddddddeeeeeeeeffffffffffffffffffffffffaa1234,
            0x111111112222222233333333444444445555555566666666777777778888888899999999aaaaaaaabbbbbbbbccccccccddddddddeeeeeeeeffffffffffffffffffffffffaa12312356456,
            0x111111112222222233333333444444445555555566666666777777778888888899999999aaaaaaaabbbbbbbbccccccccddddddddeeeeeeeeffffffffffffffffffffffffaa23563524234124345234234,
        ]
        ids.length = len(preimages)
    %}

    with sha256_ptr, pow2_array {   
        run_test(index=length - 1);
    }

    SHA256.finalize(sha256_start_ptr=sha256_ptr_start, sha256_end_ptr=sha256_ptr);

    return ();
}

func run_test{
    range_check_ptr,
    bitwise_ptr: BitwiseBuiltin*,
    sha256_ptr: felt*,
    pow2_array: felt*
}(index: felt) {
    alloc_locals;

    if(index == 0) {
        return ();
    }

    let (input: felt*) = alloc();
    local n_bytes: felt;
    local expected: Uint256;
    %{
        import hashlib
        from cairo.py.utils import hex_to_chunks_32
        preimage = preimages[ids.index]
        print(hex(preimage))
        ids.n_bytes = (preimage.bit_length() + 7) // 8
        
        expected = hashlib.sha256(preimage.to_bytes(length=ids.n_bytes, byteorder='big')).hexdigest()
        ids.expected.high, ids.expected.low = divmod(int(expected, 16), 2**128)

        chunks = hex_to_chunks_32(hex(preimage))
        print([hex(chunk) for chunk in chunks])

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

// 00100010001000100010001000100010000000101000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000101000

// 22222222
// 02800000
// 00000000
// 00000000
// 00000000
// 00000000
// 00000000
// 00000000
// 00000000
// 00000000
// 00000000
// 00000000
// 00000000
// 00000000
// 00000000
// 00000028

// 1000101010110101101000110100110101100010001101101010100110100111010111100101111101011110111100111100111111011101000010000110010111100111111011011100110010001111100010111101011100010101100001111100111000101000001000101010010111000010000011010101111100000100
// 0110100010001010010010111000101000111001110111000011100010101110001100000110000111111110001110011001011010000010100110100011000001100100111100010111011011000101001101101001111110010001000100111100111001010000100110011011011000000101010111000111111100100000