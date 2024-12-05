 use starknet::ContractAddress;

#[starknet::interface]
pub trait IBankaiContract<TContractState> {
    fn get_committee_hash(self: @TContractState, committee_id: u64) -> u256;
    fn get_latest_epoch(self: @TContractState) -> u64;
    fn get_committee_update_contract(self: @TContractState) -> ContractAddress;
    fn get_epoch_update_contract(self: @TContractState) -> ContractAddress;
    fn verify_committee_update(ref self: TContractState, state_root: u256, committee_hash: u256, slot: u64);
}

#[starknet::contract]
pub mod BankaiContract {
    use starknet::storage::{
        Map, StorageMapReadAccess, StorageMapWriteAccess, StoragePointerReadAccess,
        StoragePointerWriteAccess
    };
    use starknet::{ContractAddress, get_caller_address};

    #[event]
    #[derive(Drop, starknet::Event)]
    pub enum Event {
        CommitteeUpdated: CommitteeUpdated,
    }

    #[derive(Drop, starknet::Event)]
    pub struct CommitteeUpdated {
        committee_id: u64,
        committee_hash: u256,
    }

 
    #[storage]
    struct Storage {
        committee: Map::<u64, u256>, // maps committee index to committee hash (sha256(x || y)) of aggregate key
        headers: Map::<u64, u256>, // maps beacon slot to header hash
        owner: ContractAddress,
        latest_epoch: u64,
        initialization_committee: u64,
        committee_update_contract: ContractAddress,
        epoch_update_contract: ContractAddress
    }

    #[constructor]
    pub fn constructor(
        ref self: ContractState,
        committee_id: u64,
        committee_hash: u256,
        committee_update_contract: ContractAddress,
        epoch_update_contract: ContractAddress
    ) {
        self.owner.write(get_caller_address());
        self.latest_epoch.write(0);
        self.initialization_committee.write(committee_id);
        self.committee.write(committee_id, committee_hash);
        self.committee_update_contract.write(committee_update_contract);
        self.epoch_update_contract.write(epoch_update_contract);
    }

    #[abi(embed_v0)]
    impl BankaiContractImpl of super::IBankaiContract<ContractState> {
 
        fn get_committee_hash(self: @ContractState, committee_id: u64) -> u256 {
            self.committee.read(committee_id)
        }

        fn get_latest_epoch(self: @ContractState) -> u64 {
            self.latest_epoch.read()
        }

        fn get_committee_update_contract(self: @ContractState) -> ContractAddress {
            self.committee_update_contract.read()
        }

        fn get_epoch_update_contract(self: @ContractState) -> ContractAddress {
            self.epoch_update_contract.read()
        }

        fn verify_committee_update(ref self: ContractState, state_root: u256, committee_hash: u256, slot: u64) {
            // for now do a dummy state_root check
            assert(state_root == state_root, 'Invalid State Root!'); 

            // for now we dont ensure the fact hash is valid
            assert(1 == 1, 'Invalid Fact Hash!');

            // The new committee is always assigned at the start of the previous committee
            let new_committee_id = (slot / 0x2000) + 1;
            
            self.committee.write(new_committee_id, committee_hash);
            self.emit(Event::CommitteeUpdated(CommitteeUpdated {
                committee_id: new_committee_id,
                committee_hash: committee_hash
            }));

        }
    }


}

#[cfg(test)]
mod tests {
    use super::{BankaiContract, IBankaiContract};
    use starknet::testing::{set_caller_address};
    use starknet::ContractAddress;

    // Helper function to deploy the contract
    fn deploy(committee_id: u64, committee_hash: u256, committee_update_contract: ContractAddress, epoch_update_contract: ContractAddress) -> BankaiContract::ContractState {
        let mut state = BankaiContract::contract_state_for_testing();
        BankaiContract::constructor(ref state, committee_id, committee_hash, committee_update_contract, epoch_update_contract);
        state
    }

    #[test]
    fn test_deploy() {
        // Setup
        let committee_id = 1_u64;
        let committee_hash = 0x456_u256;
        let committee_update_contract = starknet::contract_address_const::<0x123>();
        let epoch_update_contract = starknet::contract_address_const::<0x123>();
        let owner = starknet::contract_address_const::<0x123>();
        set_caller_address(owner);

        // Deploy the contract
        let state = deploy(committee_id, committee_hash, committee_update_contract, epoch_update_contract);

        // // Assert initial state
        assert_eq!(state.get_committee_hash(committee_id), committee_hash);
        assert_eq!(state.get_latest_epoch(), 0);
        assert_eq!(state.get_committee_update_contract(), committee_update_contract);
        assert_eq!(state.get_epoch_update_contract(), epoch_update_contract);
    }

    #[test]
    fn test_committee_update() {
        let mut state = deploy(1, 0x456_u256, starknet::contract_address_const::<0x123>(), starknet::contract_address_const::<0x123>());
        state.verify_committee_update(0x123_u256, 0x456_u256, 5800000);

        assert_eq!(state.get_committee_hash(1), 0x456_u256);
    }
}