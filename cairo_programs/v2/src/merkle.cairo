use alexandria_math::sha256::sha256;
use debug::PrintTrait;

fn compute_root(values: Array<Array<u8>>) -> Array<u8> {
    let mut i = 0;
    let mut l1: Array<Array<u8>> = ArrayTrait::new();

    loop {
        if i >= values.len() {
            break;
        }

        let concat = concat_arrays(values.at(i), values.at(i + 1));
        l1.append(sha256(concat));
        i = i + 2;
    };

    l1.append(array![0xf5, 0xa5, 0xfd, 0x42, 0xd1, 0x6a, 0x20, 0x30, 0x27, 0x98, 0xef, 0x6e, 0xd3, 0x09, 0x97, 0x9b, 0x43, 0x00, 0x3d, 0x23, 0x20, 0xd9, 0xf0, 0xe8, 0xea, 0x98, 0x31, 0xa9, 0x27, 0x59, 0xfb, 0x4b]); // this is H(0, 0) which is always the same for header

    let mut l2: Array<Array<u8>> = ArrayTrait::new();
    i = 0;

    loop {
        if i >= l1.len() {
            break;
        }
        let a = l1.at(i);
        let b = l1.at(i + 1);
        let concat = concat_arrays(a, b);
        let hash: Array<u8> = sha256(concat);
        l2.append(hash);
        i = i + 2;
    };

    let concat = concat_arrays(l2.at(0), l2.at(1));

    return sha256(concat);
    
}

fn concat_arrays(a: @Array<u8>, b: @Array<u8>) -> Array<u8> {
    let mut result: Array<u8> = ArrayTrait::new();
    let mut i = 0;

    loop {
        if i >= a.len() {
            break;
        }

        result.append(*(a.at(i)));
        i = i + 1;
    };

    i = 0;

    loop {
        if i >= b.len() {
            break;
        }

        result.append(*(b.at(i)));
        i = i + 1;
    
    };
    return result;
}

impl RectanglePrintImpl of PrintTrait<Array<u8>> {
    fn print(self: Array<u8>) {
        let mut i = 0;
        let length = self.len();
        'Array<u8>['.print();
        loop {
            if i >= length {
                break;
            }
            let byte: u8 = *(self.at(i));
            byte.print();
            i = i + 1;
        };
        ']____'.print();
    }
}