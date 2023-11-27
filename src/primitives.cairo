trait Unpack<T> {
    fn unpack(self: T) -> Array<u8>;
}

#[derive(Copy, Drop)]
struct U64 {
    value: u64,
}

impl U64Impl of Unpack<U64> {
    // Unpacks the u64 into an array of bytes. SSZ numbers are also LE (like in cairo), so we keep the order. 
    fn unpack(self: U64) -> Array<u8> {
        return array![
            (self.value & 0xFF).try_into().unwrap(), // Byte 1
            ((self.value / 256) & 0xFF).try_into().unwrap(), // Byte 2
            ((self.value / 65536) & 0xFF).try_into().unwrap(), // Byte 3
            ((self.value / 16777216) & 0xFF).try_into().unwrap(), // Byte 4
            ((self.value / 4294967296) & 0xFF).try_into().unwrap(), // Byte 5
            ((self.value / 1099511627776) & 0xFF).try_into().unwrap(), // Byte 6
            ((self.value / 281474976710656) & 0xFF).try_into().unwrap(), // Byte 7
            ((self.value / 72057594037927936) & 0xFF).try_into().unwrap(), // Byte 8
            0_u8,0_u8,0_u8,0_u8,0_u8,0_u8,0_u8,0_u8,0_u8,0_u8,0_u8,0_u8,0_u8,0_u8,0_u8,0_u8,0_u8,0_u8,0_u8,0_u8,0_u8,0_u8,0_u8,0_u8 // RHS padding
        ];
    }
} 

#[derive(Copy, Drop)]
struct Hash {
    value: u256,
}

impl HashImpl of Unpack<Hash> {
    // Unpacks the hash into an array of bytes. Since this is not a number, we need to reverse the endianess again (LE -> BE)
    fn unpack(self: Hash) -> Array<u8> {
        let low_bytes: Array<u8> = extract_bytes_from_part(self.value.low);
        let high_bytes: Array<u8> = extract_bytes_from_part(self.value.high);

        // Combine high and low bytes into a single array sequentially
        let res: Array<u8> = array![
            *high_bytes.at(0), *high_bytes.at(1), *high_bytes.at(2), *high_bytes.at(3),
            *high_bytes.at(4), *high_bytes.at(5), *high_bytes.at(6), *high_bytes.at(7),
            *high_bytes.at(8), *high_bytes.at(9), *high_bytes.at(10), *high_bytes.at(11),
            *high_bytes.at(12), *high_bytes.at(13), *high_bytes.at(14), *high_bytes.at(15),
            *low_bytes.at(0), *low_bytes.at(1), *low_bytes.at(2), *low_bytes.at(3),
            *low_bytes.at(4), *low_bytes.at(5), *low_bytes.at(6), *low_bytes.at(7),
            *low_bytes.at(8), *low_bytes.at(9), *low_bytes.at(10), *low_bytes.at(11),
            *low_bytes.at(12), *low_bytes.at(13), *low_bytes.at(14), *low_bytes.at(15)
        ];

        return res;
    }
}

fn extract_bytes_from_part(part: u128) -> Array<u8> {
    return array![
        ((part / 1329227995784915872903807060280344576) & 0xFF).try_into().unwrap(), // Byte 16
        ((part / 5192296858534827628530496329220096) & 0xFF).try_into().unwrap(),    // Byte 15
        ((part / 20282409603651670423947251286016) & 0xFF).try_into().unwrap(),      // Byte 14
        ((part / 79228162514264337593543950336) & 0xFF).try_into().unwrap(),         // Byte 13
        ((part / 309485009821345068724781056) & 0xFF).try_into().unwrap(),           // Byte 12
        ((part / 1208925819614629174706176) & 0xFF).try_into().unwrap(),             // Byte 11
        ((part / 4722366482869645213696) & 0xFF).try_into().unwrap(),                // Byte 10
        ((part / 18446744073709551616) & 0xFF).try_into().unwrap(),                  // Byte 9
        ((part / 72057594037927936) & 0xFF).try_into().unwrap(),                     // Byte 8
        ((part / 281474976710656) & 0xFF).try_into().unwrap(),                       // Byte 7
        ((part / 1099511627776) & 0xFF).try_into().unwrap(),                         // Byte 6
        ((part / 4294967296) & 0xFF).try_into().unwrap(),                            // Byte 5
        ((part / 16777216) & 0xFF).try_into().unwrap(),                              // Byte 4
        ((part / 65536) & 0xFF).try_into().unwrap(),                                 // Byte 3
        ((part / 256) & 0xFF).try_into().unwrap(),                                   // Byte 2
        (part & 0xFF).try_into().unwrap()                                            // Byte 1
    ];
}

#[cfg(test)]
mod tests {
    use super::{U64, U64Impl, Hash, HashImpl};

    #[test]
    #[available_gas(10000000)]
    fn test_small_number_u64() {
        let num = U64 { value: 1 };
        let result = num.unpack();
        assert(result == array![
            1,0,0,0,0,0,0,0,
            0,0,0,0,0,0,0,0,
            0,0,0,0,0,0,0,0,
            0,0,0,0,0,0,0,0
        ], 'Failed');
    }

    #[test]
    #[available_gas(10000000)]
    fn test_large_number_u64() {
        let num = U64 { value: 0xFFFFFFFFFFFFFFFF };
        let result = num.unpack();
        assert(result == array![
            0xFF,0xFF,0xFF,0xFF,0xFF,0xFF,0xFF,0xFF,
            0,0,0,0,0,0,0,0,
            0,0,0,0,0,0,0,0,
            0,0,0,0,0,0,0,0
        ], 'Failed');
    }

    #[test]
    #[available_gas(10000000)]
    fn test_zero_u64() {
        let num = U64 { value: 0 };
        let result = num.unpack();
        assert(result == array![
            0,0,0,0,0,0,0,0,
            0,0,0,0,0,0,0,0,
            0,0,0,0,0,0,0,0,
            0,0,0,0,0,0,0,0
        ], 'Failed');
    }

    #[test]
    #[available_gas(10000000)]
    fn test_different_bytes_u64() {
        let num = U64 { value: 0x0102030405060708 };
        let result = num.unpack();
        assert(result == array![
            0x08,0x07,0x06,0x05,0x04,0x03,0x02,0x01,
            0,0,0,0,0,0,0,0,
            0,0,0,0,0,0,0,0,
            0,0,0,0,0,0,0,0
        ], 'Failed');
    }

    fn create_hash(value: u256) -> Hash {
        return Hash { value: value};
    }

    #[test]
    #[available_gas(10000000)]
    fn test_small_number_hash() {
        let value = create_hash(1);
        let result = value.unpack();
        assert(result == array![
            0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 1
        ], 'Failed');
    }

    #[test]
    #[available_gas(10000000)]
    fn test_large_number_hash() {
        let value = create_hash(115792089237316195423570985008687907853269984665640564039457584007913129639935);
        let result = value.unpack();
        assert(result == array![
            0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
            0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
            0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
            0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF
        ], 'Failed');
    }

    #[test]
    #[available_gas(10000000)]
    fn test_zero_hash() {
        let value = create_hash(0);
        let result = value.unpack();
        assert(result == array![
            0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0
        ], 'Failed');
    }

    #[test]
    #[available_gas(10000000)]
    fn test_different_parts_u256() {
        let value = create_hash(0x0102030405060708090A0B0C0D0E0F101112131415161718191A1B1C1D1E1F20);
        let result = value.unpack();
        assert(result == array![
            0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 
            0x09, 0x0A, 0x0B, 0x0C, 0x0D, 0x0E, 0x0F, 0x10, 
            0x11, 0x12, 0x13, 0x14, 0x15, 0x16, 0x17, 0x18, 
            0x19, 0x1A, 0x1B, 0x1C, 0x1D, 0x1E, 0x1F, 0x20
        ], 'Failed');
    }
}