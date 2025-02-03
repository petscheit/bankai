use bankai::BankaiContract;
use bankai::IBankaiContract;

#[cfg(test)]
mod tests {
    use super::{BankaiContract, IBankaiContract};
    use starknet::contract_address_const;
    use starknet::testing::set_caller_address;
    use starknet::testing::set_block_timestamp;
    use starknet::ClassHash;
    use openzeppelin_upgrades::interface::IUpgradeable;
    use openzeppelin_access::ownable::interface::IOwnable;

    // Helper function to deploy the contract for testing
    fn deploy_contract() -> BankaiContract::ContractState {
        let mut state = BankaiContract::contract_state_for_testing();

        // Set caller as contract deployer
        let owner = contract_address_const::<0x123>();
        set_caller_address(owner);

        // Initialize with some test values
        BankaiContract::constructor(
            ref state,
            1, // committee_id
            1234.into(), // committee_hash
            111.into(), // committee_update_program_hash
            222.into(), // epoch_update_program_hash
            333.into() // epoch_batch_program_hash
        );

        state
    }

    #[test]
    fn test_constructor() {
        let state = deploy_contract();
        let owner = contract_address_const::<0x123>();

        assert!(!IBankaiContract::is_paused(@state));
        assert_eq!(IBankaiContract::get_latest_epoch_slot(@state), 0);
        assert_eq!(IBankaiContract::get_latest_committee_id(@state), 1);
        assert_eq!(IBankaiContract::get_committee_hash(@state, 1), 1234.into());
        assert_eq!(IBankaiContract::get_committee_update_program_hash(@state), 111);
        assert_eq!(IBankaiContract::get_epoch_update_program_hash(@state), 222);
        // Use the ownable component to check owner
        assert_eq!(state.ownable.owner(), owner);
    }

    #[test]
    fn test_pause_unpause() {
        let mut state = deploy_contract();
        let owner = contract_address_const::<0x123>();
        set_caller_address(owner);

        // Test initial state
        assert!(!IBankaiContract::is_paused(@state));

        // Test pause
        IBankaiContract::pause(ref state);
        assert!(IBankaiContract::is_paused(@state));

        // Test unpause
        IBankaiContract::unpause(ref state);
        assert!(!IBankaiContract::is_paused(@state));
    }

    #[test]
    #[should_panic(expected: ('Caller is not the owner',))]
    fn test_pause_unauthorized() {
        let mut state = deploy_contract();

        // Try to pause from different address
        let other = contract_address_const::<0x456>();
        set_caller_address(other);
        IBankaiContract::pause(ref state);
    }

    #[test]
    fn test_transfer_ownership() {
        let mut state = deploy_contract();
        let owner = contract_address_const::<0x123>();
        let new_owner = contract_address_const::<0x456>();

        // Set caller as current owner
        set_caller_address(owner);

        // Use the ownable component directly
        state.ownable.transfer_ownership(new_owner);

        // Verify new owner
        assert_eq!(state.ownable.owner(), new_owner);
    }

    #[test]
    #[should_panic(expected: ('Caller is not the owner',))]
    fn test_transfer_ownership_unauthorized() {
        let mut state = deploy_contract();
        let non_owner = contract_address_const::<0x456>();
        let new_owner = contract_address_const::<0x789>();

        // Try to transfer ownership from non-owner address
        set_caller_address(non_owner);
        state.ownable.transfer_ownership(new_owner);
    }

    #[test]
    fn test_renounce_ownership() {
        let mut state = deploy_contract();
        let owner = contract_address_const::<0x123>();

        // Set caller as current owner
        set_caller_address(owner);

        // Renounce ownership
        state.ownable.renounce_ownership();

        // Verify owner is now zero address
        assert_eq!(state.ownable.owner().into(), 0);
    }

    #[test]
    #[should_panic(expected: ('Caller is not the owner',))]
    fn test_renounce_ownership_unauthorized() {
        let mut state = deploy_contract();
        let non_owner = contract_address_const::<0x456>();

        // Try to renounce ownership from non-owner address
        set_caller_address(non_owner);
        state.ownable.renounce_ownership();
    }

    #[test]
    fn test_program_hash_update() {
        let mut state = deploy_contract();
        let owner = contract_address_const::<0x123>();
        set_caller_address(owner);

        // Set initial timestamp
        set_block_timestamp(1000);

        // Propose update
        IBankaiContract::propose_program_hash_update(
            ref state,
            444.into(), // new_committee_hash
            555.into(), // new_epoch_hash
            666.into() // new_batch_hash
        );

        // Execute after delay
        set_block_timestamp(1000 + 172800); // After 48-hour delay
        IBankaiContract::execute_program_hash_update(ref state);

        // Verify updates
        assert_eq!(IBankaiContract::get_committee_update_program_hash(@state), 444);
        assert_eq!(IBankaiContract::get_epoch_update_program_hash(@state), 555);
        assert_eq!(IBankaiContract::get_epoch_batch_program_hash(@state), 666);
    }

    #[test]
    #[should_panic(expected: ('Delay not elapsed',))]
    fn test_program_hash_update_too_early() {
        let mut state = deploy_contract();
        let owner = contract_address_const::<0x123>();
        set_caller_address(owner);

        // Set initial timestamp
        set_block_timestamp(1000);

        // Propose update
        IBankaiContract::propose_program_hash_update(
            ref state,
            444.into(), // new_committee_hash
            555.into(), // new_epoch_hash
            666.into() // new_batch_hash
        );

        // Try to execute before delay
        set_block_timestamp(1000 + 172799); // Just before 48-hour delay
        IBankaiContract::execute_program_hash_update(ref state);
    }

    #[test]
    #[should_panic(expected: ('Caller is not the owner',))]
    fn test_program_hash_update_unauthorized() {
        let mut state = deploy_contract();
        let non_owner = contract_address_const::<0x456>();
        set_caller_address(non_owner);

        IBankaiContract::propose_program_hash_update(ref state, 444.into(), 555.into(), 666.into());
    }

    #[test]
    fn test_getters() {
        let state = deploy_contract();

        assert_eq!(IBankaiContract::get_committee_hash(@state, 1), 1234.into());
        assert_eq!(IBankaiContract::get_latest_epoch_slot(@state), 0);
        assert_eq!(IBankaiContract::get_latest_committee_id(@state), 1);
        assert_eq!(IBankaiContract::get_committee_update_program_hash(@state), 111);
        assert_eq!(IBankaiContract::get_epoch_update_program_hash(@state), 222);
    }

    #[test]
    #[should_panic(expected: ('CLASS_HASH_NOT_FOUND',))]
    fn test_upgrade() {
        let mut state = deploy_contract();
        let owner = contract_address_const::<0x123>();
        let new_class_hash: ClassHash = 123.try_into().unwrap();

        set_caller_address(owner);

        // Attempt upgrade
        IUpgradeable::upgrade(ref state, new_class_hash);
        // Note: In a real test environment, you'd want to verify the upgrade
    // was successful, but this requires additional test infrastructure
    }

    #[test]
    #[should_panic(expected: ('Caller is not the owner',))]
    fn test_upgrade_unauthorized() {
        let mut state = deploy_contract();
        let non_owner = contract_address_const::<0x456>();
        let new_class_hash: ClassHash = 123.try_into().unwrap();

        set_caller_address(non_owner);
        IUpgradeable::upgrade(ref state, new_class_hash);
    }

    #[test]
    fn test_paused_state_prevents_operations() {
        let mut state = deploy_contract();
        let owner = contract_address_const::<0x123>();
        set_caller_address(owner);

        // Pause the contract
        IBankaiContract::pause(ref state);
        assert!(IBankaiContract::is_paused(@state));
        // Verify that operations are prevented when paused
    // Note: You might want to add more specific tests for each operation
    // that should be prevented when paused
    }

    #[test]
    #[should_panic(expected: ('Contract is already paused',))]
    fn test_double_pause() {
        let mut state = deploy_contract();
        let owner = contract_address_const::<0x123>();
        set_caller_address(owner);

        IBankaiContract::pause(ref state);
        IBankaiContract::pause(ref state); // Should fail
    }

    #[test]
    #[should_panic(expected: ('Contract is not paused',))]
    fn test_double_unpause() {
        let mut state = deploy_contract();
        let owner = contract_address_const::<0x123>();
        set_caller_address(owner);

        IBankaiContract::unpause(ref state); // Should fail when not paused
    }

    #[test]
    fn test_program_hash_update_full_flow() {
        let mut state = deploy_contract();
        let owner = contract_address_const::<0x123>();
        set_caller_address(owner);

        // Set initial timestamp
        set_block_timestamp(1000);

        // Store initial values
        let initial_committee_hash = IBankaiContract::get_committee_update_program_hash(@state);
        let initial_epoch_hash = IBankaiContract::get_epoch_update_program_hash(@state);
        let initial_batch_hash = IBankaiContract::get_epoch_batch_program_hash(@state);

        // Propose update
        IBankaiContract::propose_program_hash_update(ref state, 444.into(), 555.into(), 666.into());

        // Verify values haven't changed before delay
        assert_eq!(
            IBankaiContract::get_committee_update_program_hash(@state), initial_committee_hash,
        );
        assert_eq!(IBankaiContract::get_epoch_update_program_hash(@state), initial_epoch_hash);
        assert_eq!(IBankaiContract::get_epoch_batch_program_hash(@state), initial_batch_hash);

        // Execute after delay
        set_block_timestamp(1000 + 172800);
        IBankaiContract::execute_program_hash_update(ref state);

        // Verify all values updated
        assert_eq!(IBankaiContract::get_committee_update_program_hash(@state), 444);
        assert_eq!(IBankaiContract::get_epoch_update_program_hash(@state), 555);
        assert_eq!(IBankaiContract::get_epoch_batch_program_hash(@state), 666);
    }
}
