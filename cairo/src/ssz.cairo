from starkware.cairo.common.uint256 import Uint256
from starkware.cairo.common.builtin_keccak.keccak import keccak_uint256s_bigend
from starkware.cairo.common.alloc import alloc

namespace SSZ {
    func hash_pair_container{
        range_check_ptr,
    }(left: Uint256, right: Uint256) -> Uint256 {
        alloc_locals;
        // let (nodes: Uint256*) = alloc();
        // assert nodes[0] = left;
        // assert nodes[1] = right;
        // let (hash) = keccak_uint256s_bigend(n_elements=2, elements=nodes);
        local result: Uint256;
        %{
            import hashlib

            def uint256_to_hex(uint256):
                return f"{uint256.high:032x}{uint256.low:032x}"

            m = hashlib.sha256()
            left_hex = uint256_to_hex(ids.left)
            right_hex = uint256_to_hex(ids.right)
            m.update(bytes.fromhex(left_hex))
            m.update(bytes.fromhex(right_hex))
            res = int( m.hexdigest(), 16)
            ids.result.high, ids.result.low = divmod(res, 2**128)
        %}
        
        return result;
    }
}