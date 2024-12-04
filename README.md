# Bankai - Cairo Ethereum Consensus Verification

A Cairo implementation for verifying Ethereum consensus via sync committee.

## Table of Contents
- [Overview](#overview)
- [Background](#background)
  - [Block Verification Process](#block-verification-process)
  - [Sync Committee Updates](#sync-committee-updates)
- [Getting Started](#getting-started)
  - [Prerequisites](#prerequisites)
  - [Installation](#installation)
- [Usage](#usage)
  - [CLI Commands](#cli-commands)
  - [Running Cairo Programs](#running-cairo-programs)
- [Examples](#examples)
- [Acknowledgments](#acknowledgments)

## Overview
Bankai enables Ethereum consensus verification in Cairo by implementing sync committee verification logic. The project consists of two main components: block verification and sync committee updates.

## Background

### Block Verification Process
The verification of an Ethereum block requires the following steps:

1. ✓ Compute block hash and signing root
2. ✓ Convert message to G2 point (hash_to_curve)
3. ✓ Aggregate signer public keys
4. ✓ Compute committee hash
5. ✓ Validate signature
6. ✓ Verify signer threshold
7. ✓ Generate verification outputs

Implementation details can be found in `main.cairo` (~200k steps).

### Sync Committee Updates
To maintain continuous operation, the system validates sync committee updates through:

1. ✓ Merkle path validation
2. ✓ Public key decompression
3. ✓ Committee hash computation
4. ✓ Hash verification

Implementation details can be found in `committee_update.cairo` (~40k steps).

## Getting Started

### Prerequisites
- Beacon Chain RPC endpoint
- Rust toolchain
- Cairo development environment

### Installation
```bash
# Install dependencies and setup environment
make setup

# Set your Beacon RPC URL
export RPC_URL_BEACON=<YOUR_BEACON_RPC_URL>

# Activate Python environment
source .venv/bin/activate
```

## Usage

### CLI Commands

#### Generate Epoch Update Proof
```bash
cargo run -- epoch-update --slot <SLOT> [--export <OUTPUT_FILE>]
```

#### Generate Committee Update Proof
```bash
cargo run -- committee-update --slot <SLOT> [--export <OUTPUT_FILE>]
```

**CLI Options:**
- `--rpc-url, -r`: Beacon Chain RPC URL (defaults to RPC_URL_BEACON env var)
- `--slot, -s`: Target slot number
- `--export, -e`: Output file path (optional)

### Running Cairo Programs

#### Epoch Update Verification
```bash
# Copy CLI output to main_input.json, then:
make build-main
make run-main
```

#### Committee Update Verification
```bash
# Copy CLI output to committee_input.json, then:
make build-committee
make run-committee
```

## Examples

### Epoch Update Proof Output
```json
{
    "header": {
        "slot": "5169248",
        "proposer_index": "191",
        "parent_root": "0x5e40ffc16ab99419cd8f5c3c4394144811b3c27e6d2d6c4ddb8a1ff15ff54552",
        "state_root": "0x5d7dfe0e508d03c8caf05fce7df40b4083f9ad72c8a2e6db5555748040a7efed",
        "body_root": "0x1ee2c12b52d6ff28f8da7f604c7f7d54a06ff76d2803657cbb19cc3c9f87baf4"
    },
    "signature_point": {
        "x0": "0x0ab7ccea53fe7f14c6873c54bde8c522640645a9da00bd90668617eb4ac0f7c631bdb854627353332cbe8a3bc8d2847a",
        "x1": "0x021cf2a040faa3c9eb4f9f708946ad7553032edc8bc55ba3dd22cc2ac380c083dde25e6e66fef7f3efe016792d8f9ff7",
        "y0": "0x18dce2581664c9e7ab5608bce9093057657705792557a443561e4d51eaaf2f8b4d061a8ed739375975da38bf5949fddf",
        "y1": "0x00e22da68c5f8675fddf8ec9ddded7604e6be523af5753ac6edd7da08623b4a8b2d9bb1026d12262ce426367b36cea6a"
    },
    "sync_committee_agg_pub": {
        "x": "0x155a4ea93d92fb321c0229ad33055a04903007e77ce41bbba1812f8464e73478d64d2c59296a22ce66607d8a7c8d06d0",
        "y": "0x151ee28a26cd768a9900f142749ee84cda21ccde6672330a321266f66648e2d4068621a093ee8e45574280b0c76bbf9c"
    },
    "non_signers": []
}
```

### Committee Update Proof Output
```json
{
  "beacon_slot": "0x61ac23",
  "next_sync_committee_branch": [
    "0x19338363d25e56f44f7f86c05d3572ea3e8261908d4ae18180cd79cb81b223df",
    "0xbafd8a06427c883bbfb30daf5982bc6be18b6ad2283330436c7282aac17b8f0c",
    "0xa5bd9ea26eff6d9bb8bfd9b9263648356d9c9b22e5102426484e5fb9bba8cbc5",
    "0x0a5cff8418c296764b19680920d2e22338ebb2bc4715a9705262fa78780b94e2",
    "0x15137476162cdfade235b4aa9b289b1392f693fc1283ff31575ad2f93bd9fa46"
  ],
  "next_aggregate_sync_committee": "a0bacb01ef15aba46b6b067b6abf214343ae826ef7ea7efc8861e13c1c6c708266cfff55f7be6b520e58feebbea6061d",
  "committee_keys_root": "9042f86bfc6e2826da86dcfa645530060cfc9347a1c733ca05350041d8993046"
}
```

## Acknowledgments
This project is built upon [Garaga](https://github.com/keep-starknet-strange/garaga). Special thanks to the Felt team for making this implementation possible.
