# Cairo Ethereum Consensus Verification
A set of tools and cairo programs with the aim of verifying Ethereum consensus in Cairo using the sync committee. This is a work in progress. 

## Background: Steps to verify an Ethereum block:
A quick overview of the steps required to verify an Ethereum block. Two different operations are required:

### Verify Sync Committee Signature

Required State: Valid Sync committee of the block

- [ ] 1. Get the block hash of the block to verify
- [ ] 2. Generate the signing root of the block
- [ ] 3. Convert the signing root to a point on G2 (hash to curve)
- [ ] 4. Generate aggregated public key of block signers
- [ ] 5. Ensure signers are in the sync committee and have >2/3 majority
- [x] 6. Verify signature


### Update Sync Committee:
Required State: Verified header containing the new sync committee (verified by the above process)

- [ ] 1. Generate a state inclusion proof for the new sync committee
- [ ] 2. Recreate beacon chain state root via SSZ merkleization
- [ ] 3. Store new sync committee for respective epochs


## CLI:
To use the CLI, access to a beacon chain rpc endpoint is required. Currently, quicknode offers a (free) endpoint that can be used for after registering. The CLI offers two commands that handle all of the preproccessing of the data (e.g. generate signing root, convert to curve points, etc.). The commands are:

### fetchProofBlock():
returns all required parameters top verifying the blocks signature. This includes the block hash, the signing root, the aggregated public key, the signature and the signer bits. This output is not compatible with cairo, but can be useful for debugging.

#### fetchProofBlockPoints():
returns the message, signature and public key as points on the curve. These values can then be used to verify the block header in the cairo program.

## G1 and G2 Curve Points:
The BLS12-381 parameters are represented as points on an elliptic curve, either in G1 or G2. This repository contains some classes to handle the conversion between the raw data and the curve points. In Ethereum, the message and signature are represented in G2, while the public key is represented in G1.


### Message (G2):
The message of the signature in Ethereum is not the block hash directly, but the signing root. This includes the block hash, but adds a domain to the message. Using `hashToCurve()` the message bytes can be converted to a point on G2.

### Signature (G2):
The signature is a point on G2 by default, as its generated via the BLS signature scheme. To verify it, it must be decoded from bytes to a point on G2.

### Public Key (G1):
The public key is a point on G1 by defaul, and must be converted from bytes to a point on G1 to verify the signature.