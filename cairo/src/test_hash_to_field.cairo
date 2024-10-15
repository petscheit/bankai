%builtins range_check bitwise range_check96 add_mod mul_mod

from starkware.cairo.common.cairo_builtins import BitwiseBuiltin, ModBuiltin
from starkware.cairo.common.alloc import alloc
from starkware.cairo.common.uint256 import Uint256

from cairo.src.hash_to_field import HashToField32
from cairo.src.sha256 import SHA256, HashUtils
from cairo.src.utils import pow2alloc128

func main{
    range_check_ptr,
    bitwise_ptr: BitwiseBuiltin*,
    range_check96_ptr: felt*,
    add_mod_ptr: ModBuiltin*,
    mul_mod_ptr: ModBuiltin*,
}() {
    alloc_locals;

    let (sha256_ptr, sha256_ptr_start) = SHA256.init();
    let (pow2_array) = pow2alloc128();

    with sha256_ptr, pow2_array {
        test_32_bytes_msg();
    }


    SHA256.finalize(sha256_ptr_start, sha256_ptr);
    return ();
}

func test_32_bytes_msg{
    range_check_ptr,
    bitwise_ptr: BitwiseBuiltin*,
    range_check96_ptr: felt*,
    add_mod_ptr: ModBuiltin*,
    mul_mod_ptr: ModBuiltin*,
    sha256_ptr: felt*,
    pow2_array: felt*
}() {
    alloc_locals;

    // This is the signing root of sepl block 5800064
    let value = Uint256(low=0x135aa063454c6023e1fbafd896f89df9, high=0x18b90e7987b9393d878786da78fa13fd);
    let (chunks) = HashUtils.chunk_uint256(value);
    let (res) = HashToField32.hash_to_field(chunks, 32, 2);
    let f0 = res[0][0];
    let f1 = res[0][1];
    let f2 = res[1][0];
    let f3 = res[1][1];

    %{

        f0 = ids.f0.d3 * 2**288 + ids.f0.d2 * 2**192 + ids.f0.d1 * 2**96 + ids.f0.d0
        f1 = ids.f1.d3 * 2**288 + ids.f1.d2 * 2**192 + ids.f1.d1 * 2**96 + ids.f1.d0
        f2 = ids.f2.d3 * 2**288 + ids.f2.d2 * 2**192 + ids.f2.d1 * 2**96 + ids.f2.d0
        f3 = ids.f3.d3 * 2**288 + ids.f3.d2 * 2**192 + ids.f3.d1 * 2**96 + ids.f3.d0

        assert f0 == 0xa1d5f7f60126ba84c5a337f4e9ae2d02ef018a7fd34c6eff78b3bcc327d8ca8f28ed9a9344b3cef5b6946d0078d34f3
        assert f1 == 0xe7f6f005fa2404be24f13fc6bf9facabc9f7b01d4a0fa1df8695251163c27437afccf0d3eb4e611fe8dd14bcd1bb881
        assert f2 == 0x184fd90d987275be14e967e4a4ee0e963544e6694c88d36358a05f0fb45ae538a0e3b687126e0962a40a3cdfa899d0aa
        assert f3 == 0xd7d91409c8b0ed8914cc0c877eb255e05135240e28835ade65557b69b6b34b18fe99801702311eb0e1328e4de7ea38b
    
    %}
    return ();
}