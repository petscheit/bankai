# Used for writing a 384 bit integer to a dict, e.g. a Cairo struct
def write_uint384(ptr, value: int):
    mask = (1 << 96) - 1  # Creates a mask of 96 1's in binary
    for i in range(4):
        chunk = value & mask
        setattr(ptr, f'd{i}', chunk)
        value >>= 96  # Shift right by 96 bits

# Split int into 4 96-bit chunks
def int_to_uint384(value: int) -> list[int]:
    mask = (1 << 96) - 1  # Creates a mask of 96 1's in binary
    chunks = []
    for i in range(4):
        chunk = value & mask
        chunks.append(chunk)
        value >>= 96  # Shift right by 96 bits
    return chunks

# Creates a G1Point from a point dictionary
def write_g1(ptr, point: dict):
    write_uint384(ptr.x, int(point["x"], 16))
    write_uint384(ptr.y, int(point["y"], 16))

# Creates a G2Point from a point dictionary
def write_g2(ptr, point: dict):
    write_uint384(ptr.x0, int(point["x0"], 16))
    write_uint384(ptr.x1, int(point["x1"], 16))
    write_uint384(ptr.y0, int(point["y0"], 16))
    write_uint384(ptr.y1, int(point["y1"], 16))

# Creates a G1G2Pair
def write_g1g2(ptr, g1: dict, g2: dict):
    write_g1(ptr.P, g1)
    write_g2(ptr.Q, g2)

# Convert list of pubkeys to array of uint384
def generate_signers_array(pubs: list[dict]):
    values = []
    for pub in pubs:
        x_chunks = int_to_uint384(int(pub["x"], 16))
        y_chunks = int_to_uint384(int(pub["y"], 16))
        values.append([x_chunks, y_chunks])
    return values

def split_uint256(value: int):
    return [value & 0xFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF, value >> 128]


# print(split_uint256(0xCB81D775883FCD8E6F9F32DA1CE6D3E74A958AA6ADBAF4402DC90A8BDB718FE202D894D10B4D62653B51E0FFFF682B9))


# print(int_to_uint384(0xCB81D775883FCD8E6F9F32DA1CE6D3E74A958AA6ADBAF4402DC90A8BDB718FE202D894D10B4D62653B51E0FFFF682B9))