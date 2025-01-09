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


def hash_tree_root_of_execution_payload_header(fields: dict) -> tuple[bytes, list[bytes]]:
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
