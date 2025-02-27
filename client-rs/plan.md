# Cairo Ethereum Project Reorganization Plan

This document outlines a methodical approach to reorganizing the Cairo Ethereum project into a Cargo workspace with multiple crates. Each step includes specific actions and considerations to ensure a smooth transition.

## Phase 1: Setup and Preparation

- [ ] **Step 1.1: Create a backup of the current codebase**
  - Create a git branch or copy of the entire project to ensure we can revert if needed
  - Commit all current changes to have a clean starting point

- [ ] **Step 1.2: Create the workspace Cargo.toml**
  - Create a new Cargo.toml file at the root of the project
  - Configure it as a workspace that includes all the planned crates
  - Example:
    ```toml
    [workspace]
    members = [
        "crates/core",
        "crates/beacon",
        "crates/starknet",
        "crates/api",
        "crates/db",
        "crates/proofs",
        "crates/cli",
        "crates/daemon",
    ]
    resolver = "2"
    ```

- [ ] **Step 1.3: Create directory structure**
  - Create the `crates` directory
  - Create subdirectories for each crate:
    - `crates/core`
    - `crates/beacon`
    - `crates/starknet`
    - `crates/api`
    - `crates/db`
    - `crates/proofs`
    - `crates/cli`
    - `crates/daemon`

## Phase 2: Create Individual Crates

- [ ] **Step 2.1: Create the core crate**
  - Create `crates/core/Cargo.toml` with basic metadata and dependencies
  - Create `crates/core/src/lib.rs` as the crate root
  - Move shared types, traits, constants, and utilities into this crate
  - Files to move:
    - `constants.rs` → `crates/core/src/constants.rs`
    - `config.rs` → `crates/core/src/config.rs`
    - `traits.rs` → `crates/core/src/traits.rs`
    - `helpers.rs` → `crates/core/src/helpers.rs`
    - `utils/hashing.rs` → `crates/core/src/hashing.rs`
    - `utils/events.rs` → `crates/core/src/events.rs`
  - Update imports and exports in `lib.rs`

- [ ] **Step 2.2: Create the beacon crate**
  - Create `crates/beacon/Cargo.toml` with dependencies including the core crate
  - Create `crates/beacon/src/lib.rs` as the crate root
  - Move beacon chain related code:
    - `utils/rpc.rs` → `crates/beacon/src/rpc.rs`
    - `utils/atlantic_client.rs` → `crates/beacon/src/atlantic_client.rs`
    - `epoch_update.rs` → `crates/beacon/src/epoch_update.rs`
    - `sync_committee.rs` → `crates/beacon/src/sync_committee.rs`
  - Update imports to use the core crate
  - Update exports in `lib.rs`

- [ ] **Step 2.3: Create the starknet crate**
  - Create `crates/starknet/Cargo.toml` with dependencies including the core crate
  - Create `crates/starknet/src/lib.rs` as the crate root
  - Move StarkNet related code:
    - `utils/starknet_client.rs` → `crates/starknet/src/client.rs`
    - `utils/cairo_runner.rs` → `crates/starknet/src/cairo_runner.rs`
    - `contract_init.rs` → `crates/starknet/src/contract_init.rs`
    - `utils/transactor_client.rs` → `crates/starknet/src/transactor_client.rs`
  - Update imports to use the core crate
  - Update exports in `lib.rs`

- [ ] **Step 2.4: Create the api crate**
  - Create `crates/api/Cargo.toml` with dependencies including the core and db crates
  - Create `crates/api/src/lib.rs` as the crate root
  - Move API related code:
    - `routes/mod.rs` → `crates/api/src/routes/mod.rs`
    - `routes/dashboard.rs` → `crates/api/src/routes/dashboard.rs`
  - Update imports to use the core and db crates
  - Update exports in `lib.rs`

- [ ] **Step 2.5: Create the db crate**
  - Create `crates/db/Cargo.toml` with dependencies including the core crate
  - Create `crates/db/src/lib.rs` as the crate root
  - Move database related code:
    - `utils/database_manager.rs` → `crates/db/src/manager.rs`
    - `state.rs` → `crates/db/src/state.rs`
  - Update imports to use the core crate
  - Update exports in `lib.rs`

- [ ] **Step 2.6: Create the proofs crate**
  - Create `crates/proofs/Cargo.toml` with dependencies including the core, beacon, and starknet crates
  - Create `crates/proofs/src/lib.rs` as the crate root
  - Move proof related code:
    - `epoch_batch.rs` → `crates/proofs/src/epoch_batch.rs`
    - `execution_header.rs` → `crates/proofs/src/execution_header.rs`
    - `utils/merkle.rs` → `crates/proofs/src/merkle.rs`
    - `utils/bankai_rpc_client.rs` → `crates/proofs/src/bankai_rpc_client.rs`
  - Update imports to use the core, beacon, and starknet crates
  - Update exports in `lib.rs`

- [ ] **Step 2.7: Create the cli crate**
  - Create `crates/cli/Cargo.toml` with dependencies on all other crates
  - Create `crates/cli/src/main.rs` as the binary entry point
  - Refactor the current `main.rs` to use the new crate structure
  - Update imports to use the new crates

- [ ] **Step 2.8: Create the daemon crate**
  - Create `crates/daemon/Cargo.toml` with dependencies on all other crates
  - Create `crates/daemon/src/main.rs` as the binary entry point
  - Refactor the current `daemon.rs` to use the new crate structure
  - Update imports to use the new crates

## Phase 3: Update Dependencies and Imports

- [ ] **Step 3.1: Update core crate dependencies**
  - Review and update dependencies in `crates/core/Cargo.toml`
  - Include only the dependencies needed by the core functionality

- [ ] **Step 3.2: Update beacon crate dependencies**
  - Review and update dependencies in `crates/beacon/Cargo.toml`
  - Include the core crate as a dependency
  - Include only the dependencies needed for beacon chain functionality

- [ ] **Step 3.3: Update starknet crate dependencies**
  - Review and update dependencies in `crates/starknet/Cargo.toml`
  - Include the core crate as a dependency
  - Include only the dependencies needed for StarkNet functionality

- [ ] **Step 3.4: Update api crate dependencies**
  - Review and update dependencies in `crates/api/Cargo.toml`
  - Include the core and db crates as dependencies
  - Include only the dependencies needed for API functionality

- [ ] **Step 3.5: Update db crate dependencies**
  - Review and update dependencies in `crates/db/Cargo.toml`
  - Include the core crate as a dependency
  - Include only the dependencies needed for database functionality

- [ ] **Step 3.6: Update proofs crate dependencies**
  - Review and update dependencies in `crates/proofs/Cargo.toml`
  - Include the core, beacon, and starknet crates as dependencies
  - Include only the dependencies needed for proof generation and verification

- [ ] **Step 3.7: Update cli crate dependencies**
  - Review and update dependencies in `crates/cli/Cargo.toml`
  - Include all other crates as dependencies
  - Include only the dependencies needed for CLI functionality

- [ ] **Step 3.8: Update daemon crate dependencies**
  - Review and update dependencies in `crates/daemon/Cargo.toml`
  - Include all other crates as dependencies
  - Include only the dependencies needed for daemon functionality

## Phase 4: Refactoring and Testing

- [ ] **Step 4.1: Update imports in core crate**
  - Review and update all imports in the core crate
  - Ensure all modules are properly exported in `lib.rs`

- [ ] **Step 4.2: Update imports in beacon crate**
  - Review and update all imports in the beacon crate
  - Replace imports from the old structure with imports from the core crate
  - Ensure all modules are properly exported in `lib.rs`

- [ ] **Step 4.3: Update imports in starknet crate**
  - Review and update all imports in the starknet crate
  - Replace imports from the old structure with imports from the core crate
  - Ensure all modules are properly exported in `lib.rs`

- [ ] **Step 4.4: Update imports in api crate**
  - Review and update all imports in the api crate
  - Replace imports from the old structure with imports from the core and db crates
  - Ensure all modules are properly exported in `lib.rs`

- [ ] **Step 4.5: Update imports in db crate**
  - Review and update all imports in the db crate
  - Replace imports from the old structure with imports from the core crate
  - Ensure all modules are properly exported in `lib.rs`

- [ ] **Step 4.6: Update imports in proofs crate**
  - Review and update all imports in the proofs crate
  - Replace imports from the old structure with imports from the core, beacon, and starknet crates
  - Ensure all modules are properly exported in `lib.rs`

- [ ] **Step 4.7: Update imports in cli crate**
  - Review and update all imports in the cli crate
  - Replace imports from the old structure with imports from the new crates

- [ ] **Step 4.8: Update imports in daemon crate**
  - Review and update all imports in the daemon crate
  - Replace imports from the old structure with imports from the new crates

- [ ] **Step 4.9: Compile and fix errors**
  - Run `cargo check` to identify compilation errors
  - Fix any errors that arise from the reorganization
  - Ensure all crates compile successfully

- [ ] **Step 4.10: Run tests**
  - Run `cargo test` to ensure all tests pass
  - Fix any test failures that arise from the reorganization

## Phase 5: Cleanup and Documentation

- [ ] **Step 5.1: Remove old code**
  - Once all functionality has been moved to the new crates and tested, remove the old code
  - Update the project's .gitignore file to exclude build artifacts from the new structure

- [ ] **Step 5.2: Update documentation**
  - Update the README.md to reflect the new project structure
  - Add documentation for each crate explaining its purpose and responsibilities
  - Add examples of how to use each crate

- [ ] **Step 5.3: Update build scripts and CI/CD**
  - Update any build scripts to work with the new structure
  - Update CI/CD configuration to build and test the new crates

- [ ] **Step 5.4: Final review**
  - Review the entire codebase to ensure all functionality has been preserved
  - Check for any remaining references to the old structure
  - Ensure all dependencies are correctly specified

## Phase 6: Deployment and Monitoring

- [ ] **Step 6.1: Deploy the updated application**
  - Deploy the application with the new structure
  - Monitor for any issues that may arise

- [ ] **Step 6.2: Update development workflows**
  - Update any development workflows to work with the new structure
  - Ensure all team members understand the new structure and how to work with it

## Conclusion

This plan provides a methodical approach to reorganizing the Cairo Ethereum project into a more modular and maintainable structure. By following these steps, the transition should be smooth and result in a codebase that is easier to understand, maintain, and extend. 