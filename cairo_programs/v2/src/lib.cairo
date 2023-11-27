mod unpack;

use alexandria_math::sha256::sha256;
use unpack::unpack_u64;
use unpack::unpack_u256;
use debug::PrintTrait;

#[derive(Copy, Drop)]
struct BeaconHeader {
    slot: u64,
    proposer_index: u64,
    parent_root: u256,
    state_root: u256,
    body_root: u256,
}

trait HeaderTrait {
    fn hash_tree_root(self: @BeaconHeader);
}

impl HeaderImpl of HeaderTrait {
    fn hash_tree_root(self: @BeaconHeader)  {
        let hashed_slot: Array<u8> = sha256(unpack_u64(*self.slot));
        let hashed_proposer_index: Array<u8> = sha256(unpack_u64(*self.proposer_index));
        let hashed_parent_root: Array<u8> = sha256(unpack_u256(*self.parent_root));
        let hashed_state_root: Array<u8> = sha256(unpack_u256(*self.state_root));
        let hashed_body_root: Array<u8> = sha256(unpack_u256(*self.body_root));
        return array![hashed_slot, hashed_proposer_index, hashed_parent_root, hashed_state_root, hashed_body_root];
    }
}

fn main() {
    let header = BeaconHeader {
        slot: 3434343, 
        proposer_index: 1393, 
        parent_root: 0x8bfa968d1064d7c6b1fef896f56ad511bb5854d2dfd6e6a9952736d07c9aa0a9, 
        state_root: 0x9712b4a722614bd9359d3e1e5aae3a1785ff113df738f2780f8a590794f50b86, 
        body_root: 0x00fc081845403d1b2180d48bcb4af7204a4c8a3c85c6c811445c876a50a1fdf2
    };
    header.hash_tree_root();
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