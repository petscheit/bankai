
# Used for writing a 384 bit integer to the Cairo struct
def write_uint384(ptr, value: int):
    mask = (1 << 96) - 1  # Creates a mask of 96 1's in binary
    for i in range(4):
        chunk = value & mask
        setattr(ptr, f'd{i}', chunk)
        value >>= 96  # Shift right by 96 bits

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