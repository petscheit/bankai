// %builtins range_check bitwise
%builtins output range_check bitwise poseidon range_check96 add_mod mul_mod
from starkware.cairo.common.cairo_builtins import PoseidonBuiltin, ModBuiltin

from starkware.cairo.common.cairo_builtins import BitwiseBuiltin
from starkware.cairo.common.uint256 import Uint256, uint256_reverse_endian
from starkware.cairo.common.builtin_keccak.keccak import keccak_uint256s_bigend
from starkware.cairo.common.memcpy import memcpy
from starkware.cairo.common.memset import memset
from starkware.cairo.common.alloc import alloc
from sha import SHA256
from cairo.src.utils import pow2alloc128, felt_divmod

namespace SSZ {
    func hash_pair_container{
        range_check_ptr, bitwise_ptr: BitwiseBuiltin*, pow2_array: felt*, sha256_ptr: felt*
    }(left: Uint256, right: Uint256) -> Uint256 {
        alloc_locals;

        let input = MerkleUtils.chunk_pair(left, right);
        let (result_chunks) = SHA256.hash_64(input=input - 16);
        let result = MerkleUtils.chunks_to_uint256(output=result_chunks);
        return result;
    }

    func hash_header_root{
        range_check_ptr, bitwise_ptr: BitwiseBuiltin*, pow2_array: felt*, sha256_ptr: felt*
    }(
        slot: Uint256,
        proposer_index: Uint256,
        parent_root: Uint256,
        state_root: Uint256,
        body_root: Uint256,
    ) -> Uint256 {
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

        let result = MerkleTree.compute_root(leafs=leafs, leafs_len=8);

        return result;
    }

    func hash_execution_payload_header_root{
        range_check_ptr, bitwise_ptr: BitwiseBuiltin*, pow2_array: felt*, sha256_ptr: felt*
    }() {
        alloc_locals;

        let (logs_bloom_segments: felt*) = alloc();
        %{
            header = {
                "parent_hash": "0x09328eba375e1201fa80334bde2a671e91b719f5e85fcca326cf845d908be04e",
                "fee_recipient": "0xf29ff96aaea6c9a1fba851f74737f3c069d4f1a9",
                "state_root": "0x021720ca38972042e4754b663bf4cd56fb5268ddb4b4e09adffba89c6a770a34",
                "receipts_root": "0xfa1514f5ded1a2a5312a8e44456288d52794568740e3fdfababac90bb4c324f5",
                "logs_bloom": "0x71a04c06a910c29844d00042c06410e00011d5bb5016e1c40204402344032092000890200e02c47086122403312c23ba8c301970c3218a449090967a886c700bf282616cc11409124010a05a2b0b8cb58521deb8022740a3492027131108af03ceb7031b7a444114241a0801d9440c008108c9f224200c0268a2093150c82a045b810a252338630236e028680a01423a90ed8a021805006cc2077a405de04211031a829ae42cd9915b4aa14280242290056d4a88084a09644d8e0a0a805c12b198a984025e398006d49f000028133a01a020afe204b820300908b16800c17d29b99006b10075684fa80890836800d05f81a2723209250011021100062018e427",
                "prev_randao": "0x2e8c651bbdc0e98409e0ca18fd57bab7fe8f5c11fac9a11905c5503e8808153e",
                "block_number": "6407829",
                "gas_limit": "30000000",
                "gas_used": "16716478",
                "timestamp": "1722400260",
                "extra_data": "0xd883010e05846765746888676f312e32322e34856c696e757800000000000000",
                "base_fee_per_gas": "344457511",
                "block_hash": "0xcfb7526378aeaee2a760dc62775671ac1ea0ace8a2173a8540c29590267d533e",
                "transactions_root": "0x3ce17dc5c31c9fd6f7574eafa67fcb7c91ce2ece2ca3d82d0ff8544fec299ffe",
                "withdrawals_root": "0x32a6fc99b9e3b92ea2cca4e4a200529c1c2e74aaf3f64ac1247b3f4ed3268fc0",
                "blob_gas_used": "786432",
                "excess_blob_gas": "72613888"
            }

            logs_bloom = header["logs_bloom"][2:]  # Remove '0x' prefix
            logs_bloom_segments = [int(logs_bloom[i:i+32], 16) for i in range(0, 512, 32)]
            # Swap pairs of segments (low, high)
            for i in range(0, 16, 2):
                logs_bloom_segments[i], logs_bloom_segments[i + 1] = logs_bloom_segments[i + 1], logs_bloom_segments[i]

            segments.write_arg(ids.logs_bloom_segments, logs_bloom_segments)
        %}

        // since logs_bloom is larger the 32 bytes, we need to ssz it first
        let segs = cast(logs_bloom_segments, Uint256*);
        let logs_bloom_root = MerkleTree.compute_root(leafs=segs, leafs_len=8);

        let (leaf_segments: felt*) = alloc();
        %{

            import hashlib
            def sha256(x: bytes) -> bytes:
                """Return SHA-256 digest of x."""
                return hashlib.sha256(x).digest()

            def next_power_of_two(x: int) -> int:
                """
                Return the next power of 2 >= x.
                By definition next_power_of_two(0) = 1 in the spec.
                """
                if x <= 1:
                    return 1
                p = 1
                while p < x:
                    p <<= 1
                return p

            def merkleize(chunks: list[bytes], limit: int | None = None) -> bytes:
                """
                Merkleize a list of 32-byte chunks (or fewer).
                - If `limit` is None, pad up to next_power_of_two(len(chunks)).
                - If `limit` is not None, ensure len(chunks) <= limit,
                then pad up to next_power_of_two(limit).
                - Pairwise-hash repeatedly until we get a single 32-byte root.

                Reference: https://github.com/ethereum/consensus-specs/blob/dev/ssz/simple-serialize.md
                """
                n = len(chunks)
                if limit is not None:
                    if n > limit:
                        raise ValueError(f"Too many chunks ({n}) with limit={limit}")
                    size = next_power_of_two(limit)
                else:
                    # container, no explicit limit
                    size = next_power_of_two(n)

                # pad with zero-chunks up to "size"
                chunks_padded = chunks + [b"\x00" * 32] * (size - n)

                # do a bottom-up Merkle tree
                # layer 0 is the chunks themselves
                layer = chunks_padded
                while len(layer) > 1:
                    new_layer = []
                    for i in range(0, len(layer), 2):
                        left = layer[i]
                        right = layer[i+1]
                        new_layer.append(sha256(left + right))
                    layer = new_layer

                return layer[0]  # the single root

            def mix_in_length(root: bytes, length: int) -> bytes:
                """
                For SSZ list/bitlist, we mix in the length as a 256-bit little-endian number
                at the end. (uint256-little-endian)
                """
                length_bytes_256 = length.to_bytes(32, "little")
                return sha256(root + length_bytes_256)

            #
            # --- 2) SSZ "hash_tree_root" FOR BASIC & COMPOSITE TYPES ---
            #

            def hash_tree_root_of_uint64(x: int) -> bytes:
                """
                SSZ: a uint64 is 8 bytes, little-endian.
                Then 'merkleize(pack(...))' => 1 chunk of 32 bytes => pad => merkleize => single chunk.
                """
                b = x.to_bytes(8, "little")  # 8 bytes
                # pad to 32:
                b_padded = b + b"\x00" * (32 - len(b))
                return merkleize([b_padded])

            def hash_tree_root_of_uint256(x: int) -> bytes:
                """
                SSZ: a uint256 is 32 bytes, little-endian.
                Then merkleize that single chunk directly.
                """
                b = x.to_bytes(32, "little")  # 32 bytes
                return merkleize([b])

            def hash_tree_root_of_byte_vector(fixed_bytes: bytes, length: int) -> bytes:
                """
                SSZ ByteVector[length]: must be exactly `length` bytes.
                Then we pack it into 32-byte chunks, merkleize.
                """
                if len(fixed_bytes) != length:
                    raise ValueError(f"ByteVector: expected {length} bytes, got {len(fixed_bytes)}")
                # break into 32-byte chunks
                chunks = []
                for i in range(0, len(fixed_bytes), 32):
                    chunk = fixed_bytes[i : i+32]
                    # pad if < 32
                    if len(chunk) < 32:
                        chunk = chunk + b"\x00" * (32 - len(chunk))
                    chunks.append(chunk)

                # merkleize
                return merkleize(chunks)

            def hash_tree_root_of_byte_list(variable_bytes: bytes, max_length: int) -> bytes:
                """
                SSZ ByteList[max_length].
                1) chunk the bytes (up to 32).
                2) merkleize with limit = chunk_count(type) => (max_length + 31)//32
                3) mix_in_length(root, actual_length)
                """
                if len(variable_bytes) > max_length:
                    raise ValueError(f"ByteList: length {len(variable_bytes)} > max {max_length}")

                # split into 32-byte chunks, pad final
                chunks = []
                for i in range(0, len(variable_bytes), 32):
                    chunk = variable_bytes[i : i+32]
                    if len(chunk) < 32:
                        chunk = chunk + b"\x00" * (32 - len(chunk))
                    chunks.append(chunk)

                # how many 32-byte chunks *max* could this list have?
                chunk_limit = (max_length + 31) // 32  # ceiling

                root = merkleize(chunks, limit=chunk_limit)
                # Now "mix in length" = actual length in bytes
                root = mix_in_length(root, len(variable_bytes))
                return root


            def hash_tree_root_of_execution_payload_header(fields: dict) -> bytes:
                """
                We'll treat each field as either:
                - ByteVector(N)
                - uint64
                - uint256
                - ByteList(32)
                Then gather the 17 field roots in an array and merkleize that array.
                """

                # Compute sub-root for each field:
                roots = []

                # 0. parent_hash
                roots.append(hash_tree_root_of_byte_vector(fields["parent_hash"], 32))
                roots.append(hash_tree_root_of_byte_vector(fields["fee_recipient"], 20))
                roots.append(hash_tree_root_of_byte_vector(fields["state_root"], 32))
                roots.append(hash_tree_root_of_byte_vector(fields["receipts_root"], 32))
                roots.append(hash_tree_root_of_byte_vector(fields["logs_bloom"], 256))
                roots.append(hash_tree_root_of_byte_vector(fields["prev_randao"], 32))
                roots.append(hash_tree_root_of_uint64(fields["block_number"]))
                roots.append(hash_tree_root_of_uint64(fields["gas_limit"]))
                roots.append(hash_tree_root_of_uint64(fields["gas_used"]))
                roots.append(hash_tree_root_of_uint64(fields["timestamp"]))
                roots.append(hash_tree_root_of_byte_list(fields["extra_data"], 32))
                roots.append(hash_tree_root_of_uint256(fields["base_fee_per_gas"]))
                roots.append(hash_tree_root_of_byte_vector(fields["block_hash"], 32))
                roots.append(hash_tree_root_of_byte_vector(fields["transactions_root"], 32))
                roots.append(hash_tree_root_of_byte_vector(fields["withdrawals_root"], 32))
                roots.append(hash_tree_root_of_uint64(fields["blob_gas_used"]))
                roots.append(hash_tree_root_of_uint64(fields["excess_blob_gas"]))

                # Now the container root is just merkleize(roots) with no limit
                container_root = merkleize(roots, limit=None)
                return container_root, roots

            raw_data = {
                "parent_hash": "0x09328eba375e1201fa80334bde2a671e91b719f5e85fcca326cf845d908be04e",
                "fee_recipient": "0xf29ff96aaea6c9a1fba851f74737f3c069d4f1a9",
                "state_root": "0x021720ca38972042e4754b663bf4cd56fb5268ddb4b4e09adffba89c6a770a34",
                "receipts_root": "0xfa1514f5ded1a2a5312a8e44456288d52794568740e3fdfababac90bb4c324f5",
                "logs_bloom": "0x71a04c06a910c29844d00042c06410e00011d5bb5016e1c40204402344032092000890200e02c47086122403312c23ba8c301970c3218a449090967a886c700bf282616cc11409124010a05a2b0b8cb58521deb8022740a3492027131108af03ceb7031b7a444114241a0801d9440c008108c9f224200c0268a2093150c82a045b810a252338630236e028680a01423a90ed8a021805006cc2077a405de04211031a829ae42cd9915b4aa14280242290056d4a88084a09644d8e0a0a805c12b198a984025e398006d49f000028133a01a020afe204b820300908b16800c17d29b99006b10075684fa80890836800d05f81a2723209250011021100062018e427",
                "prev_randao": "0x2e8c651bbdc0e98409e0ca18fd57bab7fe8f5c11fac9a11905c5503e8808153e",
                "block_number": "6407829",
                "gas_limit": "30000000",
                "gas_used": "16716478",
                "timestamp": "1722400260",
                "extra_data": "0xd883010e05846765746888676f312e32322e34856c696e7578",
                "base_fee_per_gas": "344457511",
                "block_hash": "0xcfb7526378aeaee2a760dc62775671ac1ea0ace8a2173a8540c29590267d533e",
                "transactions_root": "0x3ce17dc5c31c9fd6f7574eafa67fcb7c91ce2ece2ca3d82d0ff8544fec299ffe",
                "withdrawals_root": "0x32a6fc99b9e3b92ea2cca4e4a200529c1c2e74aaf3f64ac1247b3f4ed3268fc0",
                "blob_gas_used": "786432",
                "excess_blob_gas": "72613888"
            }

            def hex_to_bytes(h: str) -> bytes:
                return bytes.fromhex(h.removeprefix("0x"))

            # Build a dict in the format that hash_tree_root_of_execution_payload_header expects:
            fields = {
                "parent_hash":       hex_to_bytes(raw_data["parent_hash"]),
                "fee_recipient":     hex_to_bytes(raw_data["fee_recipient"]),
                "state_root":        hex_to_bytes(raw_data["state_root"]),
                "receipts_root":     hex_to_bytes(raw_data["receipts_root"]),
                "logs_bloom":        hex_to_bytes(raw_data["logs_bloom"]),
                "prev_randao":       hex_to_bytes(raw_data["prev_randao"]),
                "block_number":      int(raw_data["block_number"]),
                "gas_limit":         int(raw_data["gas_limit"]),
                "gas_used":          int(raw_data["gas_used"]),
                "timestamp":         int(raw_data["timestamp"]),
                "extra_data":        hex_to_bytes(raw_data["extra_data"]),
                "base_fee_per_gas":  int(raw_data["base_fee_per_gas"]),
                "block_hash":        hex_to_bytes(raw_data["block_hash"]),
                "transactions_root": hex_to_bytes(raw_data["transactions_root"]),
                "withdrawals_root":  hex_to_bytes(raw_data["withdrawals_root"]),
                "blob_gas_used":     int(raw_data["blob_gas_used"]),
                "excess_blob_gas":   int(raw_data["excess_blob_gas"]),
            }

            # Compute the container Merkle root
            root, fields = hash_tree_root_of_execution_payload_header(fields)

            
            leaf_segments = []
            for field in fields:
                # Split each 32-byte (256-bit) field into two 16-byte (128-bit) segments
                # Convert to integers for Cairo memory representation
                high_segment = int.from_bytes(field[:16], 'big')
                low_segment = int.from_bytes(field[16:], 'big')
                leaf_segments.extend([low_segment, high_segment])

            # Write segments to memory
            segments.write_arg(ids.leaf_segments, leaf_segments)

        %}

        memset(dst=leaf_segments + 34, value=0, n=30);
        %{
            i = 0
            while i < 34:
                # print(hex(memory[ids.leaf_segments + i + 1] * 2*128 + memory[ids.leaf_segments + i]))
                print(hex(memory[ids.leaf_segments + i]), hex(memory[ids.leaf_segments + i + 1]))
                i += 2
        %}

        let leafs = cast(leaf_segments, Uint256*);
        let result = MerkleTree.compute_root(leafs=leafs, leafs_len=32);
        // 0x855cc12d1c187d0472e4cd116a59216ec9e901162337d8adbabc210f029cc7af
        %{ print("Result: ", hex(ids.result.low), hex(ids.result.high)) %}

        return ();
    }
}

namespace MerkleTree {
    func compute_root{
        range_check_ptr, bitwise_ptr: BitwiseBuiltin*, pow2_array: felt*, sha256_ptr: felt*
    }(leafs: Uint256*, leafs_len: felt) -> Uint256 {
        alloc_locals;

        // ensure we have a power of 2.
        // ToDo: we should automatically add padding leafs
        %{ assert ids.leafs_len & (ids.leafs_len - 1) == 0 %}

        // chunk the leafs and write to leafs array
        let (chunked_leafs: felt*) = alloc();
        MerkleUtils.chunk_leafs{
            range_check_ptr=range_check_ptr, pow2_array=pow2_array, output_ptr=chunked_leafs
        }(leafs=leafs, leafs_len=leafs_len, index=0);

        // move the pointer to the start of the chunked leafs
        let chunked_leafs = chunked_leafs - leafs_len * 8;

        let (tree: felt*) = alloc();
        let tree_len = 2 * leafs_len - 1;  // number nodes in the tree (not accounting for chunking)

        // copy the leafs to the end of the tree arra
        memcpy(dst=tree + (tree_len - leafs_len) * 8, src=chunked_leafs, len=leafs_len * 8);

        with sha256_ptr {
            let tree = tree + tree_len * 8;  // move the pointer to the end of the tree
            compute_merkle_root_inner{
                range_check_ptr=range_check_ptr,
                sha256_ptr=sha256_ptr,
                pow2_array=pow2_array,
                tree_ptr=tree,
            }(tree_range=tree_len - leafs_len - 1, index=0);
        }

        let result = MerkleUtils.chunks_to_uint256(output=tree - 8);

        return result;
    }

    // Implements the merkle tree building logic. This follows the unordered StandardMerkleTree implementation of OpenZeppelin
    func compute_merkle_root_inner{
        range_check_ptr, sha256_ptr: felt*, pow2_array: felt*, tree_ptr: felt*
    }(tree_range: felt, index: felt) {
        alloc_locals;

        if (tree_range + 1 == index) {
            return ();
        }

        // for each iteration, we must move the pointer 16 felts back to the next pair
        tempvar tree_ptr = tree_ptr - 16;
        let (node) = SHA256.hash_64(input=tree_ptr);

        // write the hash to the correct position in the tree
        memcpy(dst=tree_ptr - (1 + tree_range - index) * 8, src=node, len=8);
        return compute_merkle_root_inner(tree_range=tree_range, index=index + 1);
    }

    func hash_merkle_path{
        range_check_ptr, bitwise_ptr: BitwiseBuiltin*, pow2_array: felt*, sha256_ptr: felt*
    }(path: felt**, path_len: felt, leaf: felt*, index: felt) -> Uint256 {
        alloc_locals;

        // Base case - if no more siblings to process, return the current value
        if (path_len == 0) {
            let result = MerkleUtils.chunks_to_uint256(output=leaf);
            return result;
        }

        // Check if current node is left or right child
        let (new_index, r) = felt_divmod(index, 2);
        if (r == 0) {
            // for some reason this break if I append to leaf, instead of doing this
            let (input: felt*) = alloc();
            memcpy(dst=input, src=leaf, len=8);
            memcpy(dst=input + 8, src=[path], len=8);
            let (result_chunks) = SHA256.hash_64(input=input);
        } else {
            memcpy(dst=[path] + 8, src=leaf, len=8);
            let (result_chunks) = SHA256.hash_64(input=[path]);
        }

        // Recurse with remaining path
        return hash_merkle_path(
            path=path + 1, path_len=path_len - 1, leaf=result_chunks, index=new_index
        );
    }
}

namespace MerkleUtils {
    func chunk_pair{range_check_ptr, pow2_array: felt*}(left: Uint256, right: Uint256) -> felt* {
        let (leafs: Uint256*) = alloc();
        assert leafs[0] = left;
        assert leafs[1] = right;

        let (output_ptr: felt*) = alloc();
        with output_ptr {
            chunk_leafs(leafs=leafs, leafs_len=2, index=0);
        }
        return output_ptr;
    }

    func chunk_leafs{range_check_ptr, pow2_array: felt*, output_ptr: felt*}(
        leafs: Uint256*, leafs_len: felt, index: felt
    ) {
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
        let low = [output + 4] * pow2_array[96] + [output + 5] * pow2_array[64] + [output + 6] *
            pow2_array[32] + [output + 7];
        let high = [output] * pow2_array[96] + [output + 1] * pow2_array[64] + [output + 2] *
            pow2_array[32] + [output + 3];
        return (Uint256(low=low, high=high));
    }
}


func main{
    output_ptr: felt*,
    range_check_ptr,
    bitwise_ptr: BitwiseBuiltin*,
    poseidon_ptr: PoseidonBuiltin*,
    range_check96_ptr: felt*,
    add_mod_ptr: ModBuiltin*,
    mul_mod_ptr: ModBuiltin*,
}() {
    alloc_locals;

    let (pow2_array) = pow2alloc128();
    let (sha256_ptr, sha256_ptr_start) = SHA256.init();

    with pow2_array, sha256_ptr {
        SSZ.hash_execution_payload_header_root();
    }

    return ();
}