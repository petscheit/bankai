// %builtins bitwise
from starkware.cairo.common.cairo_builtins import BitwiseBuiltin
from starkware.cairo.common.uint256 import Uint256, uint256_reverse_endian
from starkware.cairo.common.builtin_keccak.keccak import keccak_uint256s_bigend
from starkware.cairo.common.alloc import alloc
from cairo.src.unsafe_hints import unsafe_sha256, unsafe_compute_merkle_root

namespace SSZ {
    func hash_pair_container{
        range_check_ptr,
    }(left: Uint256, right: Uint256) -> Uint256 {
       let result = unsafe_sha256(left, right);
       return result;
    }

    func hash_header_root{
        bitwise_ptr: BitwiseBuiltin*
    }(slot: Uint256, proposer_index: Uint256, parent_root: Uint256, state_root: Uint256, body_root: Uint256) -> Uint256 {
        alloc_locals;
        // For numbers, we need to reverse the endianness
        let (slot) = uint256_reverse_endian(num=slot);
        let (proposer_index) = uint256_reverse_endian(num=proposer_index);

        let (leafs: Uint256*) = alloc();
        assert leafs[0] = slot;
        assert leafs[1] = proposer_index;
        assert leafs[2] = parent_root;
        assert leafs[3] = state_root;
        assert leafs[4] = body_root;

        // we need to pad, to make sure the length is a power of 2
        // ToDo: we can add some precomputation here
        assert leafs[5] = Uint256(low=0, high=0);
        assert leafs[6] = Uint256(low=0, high=0);
        assert leafs[7] = Uint256(low=0, high=0);

        let result = unsafe_compute_merkle_root(leafs=leafs, leafs_len=8);

        return result;
    }
}


// func main{
//     bitwise_ptr: BitwiseBuiltin*
// }() {
//     let slot = Uint256(low=5, high=0);
//     let proposer_index = Uint256(low=6, high=0);
//     let parent_root = Uint256(low=0x8e72b9e78db7f56ff3dc52db9faad317, high=0x21dee62104b733c508e90115c7a17514);
//     let state_root = Uint256(low=0xee227e18d9c4fac12ac428c7fa3f98be, high=0x46c4ba46473cd15a0df9bfa8cf175600);
//     let body_root = Uint256(low=0x0f66353403427a84cc4e9e68ab613242, high=0xf1e2b2f1fe80acb4bce95299f594ec43);
//     let root = SSZ.hash_header_root(slot, proposer_index, parent_root, state_root, body_root);

//     %{ print("root: ", hex(ids.root.low), hex(ids.root.high)) %}
    
//     return ();
// }