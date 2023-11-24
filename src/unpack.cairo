use debug::PrintTrait;

fn unpack_u16(value: u16) -> Array<u8> {
    let one: u8 = (value & 0xFF).try_into().unwrap();
    let two: u8 = (value / 256).try_into().unwrap();
    return array![one, two];
}

fn unpack_u32(value: u32) -> Array<u8> {
    let one: u8 = (value & 0xFF).try_into().unwrap();
    let two: u8 = ((value / 256) & 0xFF).try_into().unwrap(); // 2^8
    let three: u8 = ((value / 65536) & 0xFF).try_into().unwrap(); // 2^16
    let four: u8 = ((value / 16777216) & 0xFF).try_into().unwrap(); // 2^24
    return array![one, two, three, four];
}

fn unpack_u64(value: u64) -> Array<u8> {
    let one: u8 = (value & 0xFF).try_into().unwrap();
    let two: u8 = ((value / 256) & 0xFF).try_into().unwrap(); // 2^8
    let three: u8 = ((value / 65536) & 0xFF).try_into().unwrap(); // 2^16
    let four: u8 = ((value / 16777216) & 0xFF).try_into().unwrap(); // 2^24
    let five: u8 = ((value / 4294967296) & 0xFF).try_into().unwrap(); // 2^32
    let six: u8 = ((value / 1099511627776) & 0xFF).try_into().unwrap(); // 2^40
    let seven: u8 = ((value / 281474976710656) & 0xFF).try_into().unwrap(); // 2^48
    let eight: u8 = ((value / 72057594037927936) & 0xFF).try_into().unwrap(); // 2^56
    return array![one, two, three, four, five, six, seven, eight];
}

fn unpack_u128(value: u128) -> Array<u8> {
    let one: u8 = (value & 0xFF).try_into().unwrap();
    let two: u8 = ((value / 256) & 0xFF).try_into().unwrap(); // 2^8
    let three: u8 = ((value / 65536) & 0xFF).try_into().unwrap(); // 2^16
    let four: u8 = ((value / 16777216) & 0xFF).try_into().unwrap(); // 2^24
    let five: u8 = ((value / 4294967296) & 0xFF).try_into().unwrap(); // 2^32
    let six: u8 = ((value / 1099511627776) & 0xFF).try_into().unwrap(); // 2^40
    let seven: u8 = ((value / 281474976710656) & 0xFF).try_into().unwrap(); // 2^48
    let eight: u8 = ((value / 72057594037927936) & 0xFF).try_into().unwrap(); // 2^56
    let nine: u8 = ((value / 18446744073709551616) & 0xFF).try_into().unwrap(); // 2^64
    let ten: u8 = ((value / 4722366482869645213696) & 0xFF).try_into().unwrap(); // 2^72
    let eleven: u8 = ((value / 1208925819614629174706176) & 0xFF).try_into().unwrap(); // 2^80
    let twelve: u8 = ((value / 309485009821345068724781056) & 0xFF).try_into().unwrap(); // 2^88
    let thirteen: u8 = ((value / 79228162514264337593543950336) & 0xFF).try_into().unwrap(); // 2^96
    let fourteen: u8 = ((value / 20282409603651670423947251286016) & 0xFF).try_into().unwrap(); // 2^104
    let fifteen: u8 = ((value / 5192296858534827628530496329220096) & 0xFF).try_into().unwrap(); // 2^112
    let sixteen: u8 = ((value / 1329227995784915872903807060280344576) & 0xFF).try_into().unwrap(); // 2^120

    return array![one, two, three, four, five, six, seven, eight, nine, ten, eleven, twelve, thirteen, fourteen, fifteen, sixteen];
}

fn unpack_u256(value: u256) -> (Array<u8>, Array<u8>) {
    let low = unpack_u128(value.low);
    let high = unpack_u128(value.high);
    return (low, high);
}

#[cfg(test)]
mod tests {
    use super::unpack_u16;
    use super::unpack_u32;
    use super::unpack_u64;
    use super::unpack_u128;
    use super::unpack_u256;

    #[test]
    #[available_gas(10000000)]
    fn test_small_number_u16() {
        let result = unpack_u16(1);
        assert(result == array![1, 0], 'Failed');
    }

    #[test]
    #[available_gas(10000000)]
    fn test_large_number_u16() {
        let result = unpack_u16(0xFFFF);
        assert(result == array![0xFF, 0xFF], 'Failed');
    }

    #[test]
    #[available_gas(10000000)]
    fn test_zero_u16() {
        let result = unpack_u16(0);
        assert(result == array![0, 0], 'Failed');
    }

    #[test]
    #[available_gas(10000000)]
    fn test_different_msb_lsb_u16() {
        let result = unpack_u16(0x01FF);
        assert(result == array![0xFF, 0x01], 'Failed');
    }

    #[test]
    #[available_gas(10000000)]
    fn test_small_number_u32() {
        let result = unpack_u32(1);
        assert(result == array![1, 0, 0, 0], 'Failed');
    }

    #[test]
    #[available_gas(10000000)]
    fn test_large_number_u32() {
        let result = unpack_u32(0xFFFFFFFF);
        assert(result == array![0xFF, 0xFF, 0xFF, 0xFF], 'Failed');
    }

    #[test]
    #[available_gas(10000000)]
    fn test_zero_u32() {
        let result = unpack_u32(0);
        assert(result == array![0, 0, 0, 0], 'Failed');
    }

    #[test]
    #[available_gas(10000000)]
    fn test_different_bytes_u32() {
        let result = unpack_u32(0x01020304);
        assert(result == array![4, 3, 2, 1], 'Failed');
    }

    #[test]
    #[available_gas(10000000)]
    fn test_small_number_u64() {
        let result = unpack_u64(1);
        assert(result == array![1, 0, 0, 0, 0, 0, 0, 0], 'Failed');
    }

    #[test]
    #[available_gas(10000000)]
    fn test_large_number_u64() {
        let result = unpack_u64(0xFFFFFFFFFFFFFFFF);
        assert(result == array![0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF], 'Failed');
    }

    #[test]
    #[available_gas(10000000)]
    fn test_zero_u64() {
        let result = unpack_u64(0);
        assert(result == array![0, 0, 0, 0, 0, 0, 0, 0], 'Failed');
    }

    #[test]
    #[available_gas(10000000)]
    fn test_different_bytes_u64() {
        let result = unpack_u64(0x0102030405060708);
        assert(result == array![0x08, 0x07, 0x06, 0x05, 0x04, 0x03, 0x02, 0x01], 'Failed');
    }

    #[test]
    #[available_gas(10000000)]
    fn test_small_number_u128() {
        let result = unpack_u128(1);
        assert(result == array![1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0], 'Failed');
    }

    #[test]
    #[available_gas(10000000)]
    fn test_large_number_u128() {
        let result = unpack_u128(0xFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF);
        assert(result == array![0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF], 'Failed');
    }

    #[test]
    #[available_gas(10000000)]
    fn test_zero_u128() {
        let result = unpack_u128(0);
        assert(result == array![0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0], 'Failed');
    }

    #[test]
    #[available_gas(10000000)]
    fn test_different_bytes_u128() {
        let result = unpack_u128(0x0102030405060708090A0B0C0D0E0F10);
        assert(result == array![0x10, 0x0F, 0x0E, 0x0D, 0x0C, 0x0B, 0x0A, 0x09, 0x08, 0x07, 0x06, 0x05, 0x04, 0x03, 0x02, 0x01], 'Failed');
    }

    // Assuming a constructor or a way to create a u256 value
    fn create_u256(low: u128, high: u128) -> u256 {
        u256 { low, high }
    }

    #[test]
    #[available_gas(10000000)]
    fn test_small_number_u256() {
        let value = create_u256(1, 0);
        let (low, high) = unpack_u256(value);
        assert(low == unpack_u128(1), 'Failed');
        assert(high == unpack_u128(0), 'Failed');
    }

    #[test]
    #[available_gas(10000000)]
    fn test_large_number_u256() {
        let value = create_u256(0xFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF, 0xFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF);
        let (low, high) = unpack_u256(value);
        assert(low == unpack_u128(0xFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF), 'Failed');
        assert(high == unpack_u128(0xFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF), 'Failed');
    }

    #[test]
    #[available_gas(10000000)]
    fn test_zero_u256() {
        let value = create_u256(0, 0);
        let (low, high) = unpack_u256(value);
        assert(low == unpack_u128(0), 'Failed');
        assert(high == unpack_u128(0), 'Failed');
    }

    #[test]
    #[available_gas(10000000)]
    fn test_different_parts_u256() {
        let value = create_u256(0x0102030405060708090A0B0C0D0E0F10, 0x1112131415161718191A1B1C1D1E1F20);
        let (low, high) = unpack_u256(value);
        assert(low == unpack_u128(0x0102030405060708090A0B0C0D0E0F10), 'Failed');
        assert(high == unpack_u128(0x1112131415161718191A1B1C1D1E1F20), 'Failed');
    }
}