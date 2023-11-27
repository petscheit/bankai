mod primitives;
mod merkle;
use merkle::{compute_root};
use primitives::{U64, U64Impl, Hash, HashImpl};
use debug::PrintTrait;

#[derive(Copy, Drop)]
struct BeaconHeader {
    slot: U64,
    proposer_index: U64,
    parent_root: Hash,
    state_root: Hash,
    body_root: Hash,
}

trait HeaderTrait {
    fn hash_tree_root(self: BeaconHeader) -> Array<u8>;
    fn serialize(self: BeaconHeader) -> Array<Array<u8>>;
}

impl HeaderImpl of HeaderTrait {
    fn hash_tree_root(self: BeaconHeader) -> Array<u8>  {
        return compute_root(self.serialize());
    }

    fn serialize(self: BeaconHeader) -> Array<Array<u8>> {

        return array![
            self.slot.unpack(),
            self.proposer_index.unpack(),
            self.parent_root.unpack(),
            self.state_root.unpack(),
            self.body_root.unpack(),
            array![0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0] // since odd number of leafs
        ];
    
    }
}

fn main() {
    let header = BeaconHeader {
        slot: U64 {value: 3434343}, 
        proposer_index: U64 {value: 1393}, 
        parent_root: Hash {value: 0x8bfa968d1064d7c6b1fef896f56ad511bb5854d2dfd6e6a9952736d07c9aa0a9}, 
        state_root: Hash {value: 0x9712b4a722614bd9359d3e1e5aae3a1785ff113df738f2780f8a590794f50b86}, 
        body_root: Hash {value: 0x00fc081845403d1b2180d48bcb4af7204a4c8a3c85c6c811445c876a50a1fdf2}
    };
    let root = header.hash_tree_root();

    root.print();
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

#[cfg(test)]
mod tests {
    use super::{U64, U64Impl, Hash, HashImpl, BeaconHeader, HeaderImpl};

    #[test]
    #[available_gas(10000000000)]
    fn header_hash_tree_root_correctly() {
        let header = BeaconHeader {
            slot: U64 {value: 3434343}, 
            proposer_index: U64 {value: 1393}, 
            parent_root: Hash {value: 0x8bfa968d1064d7c6b1fef896f56ad511bb5854d2dfd6e6a9952736d07c9aa0a9}, 
            state_root: Hash {value: 0x9712b4a722614bd9359d3e1e5aae3a1785ff113df738f2780f8a590794f50b86}, 
            body_root: Hash {value: 0x00fc081845403d1b2180d48bcb4af7204a4c8a3c85c6c811445c876a50a1fdf2}
        };
        let root = header.hash_tree_root();

        assert(root == array![
            0xA9, 0xDE, 0x3F, 0x6E, 0x28, 0x17, 0x30, 0x37,
            0xCA, 0x34, 0x25, 0x7B, 0x42, 0xAA, 0xB2, 0x0F,
            0xA8, 0xCC, 0x4C, 0x3C, 0x61, 0x83, 0xE7, 0x35,
            0x55, 0xA3, 0xFF, 0x3E, 0xEF, 0x5E, 0x40, 0xD5
        ], 'Failed');
    }
}