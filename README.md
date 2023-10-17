# Cairo Ethereum Consensus Verification
The long term goal of this repository is to enable the verification of Ethereum blocks in the Cairo language. This requires a number of cryptographic operations which will be added step by step. Currently, a blocks headers signature can be verified in cairo, which is the first step. Below is a quick overview of the steps required to verify a block header, and the steps that are currently implemented.

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
To use the CLI, access to a sepolia beacon chain rpc endpoint is required. Currently, quicknode offers a (free) endpoint that can be used for after registering. The CLI offers two commands that handle all of the preproccessing of the data (e.g. generate signing root, convert to curve points, etc.). The commands are:

### fetchProofBlock():
returns all required parameters top verifying the blocks signature. This includes the block hash, the signing root, the aggregated public key, the signature and the signer bits. This output is not compatible with cairo, but can be useful for debugging.

#### fetchProofBlockPoints():
returns the message, signature and public key as points on the curve. These values can then be used to verify the block header in the cairo program.

### Setup and Examples:
- run `npm i` to install all dependencies
- run `npm run cli --` to launch the CLI. The "--" are important, as the argument are not parsed correctly otherwise.

#### Example 1: Fetch Block Proof - Sepolia #3434343
`npm run cli -- fetchBlockProof -b 3434343 -r https://your-secret-sepolia-beacon-endpoint.com`

Returns:
```js
{
  blockRoot: '0xa9de3f6e28173037ca34257b42aab20fa8cc4c3c6183e73555a3ff3eef5e40d5',
  signingRoot: '0xd51c1dde35692d276d24cb181380c9325c77e835e7be40fa9ca07e97e735e258',
  signature: '0x95166efbcfc7a7bed65d4141108f5cbe15411a876db47ef556586ef87ef91ba5d172a6e4179c93d773beab4271cd3e6511a82e93aab8472c21dc79161abf15577b34564005d3c36863d097fa2f88a0e6253910cbb3e61fa0f1e61e822407cca7',
  signerBits: '0xffffffffffb7f7fffffbfefbfeffffdbffffffefdffff7ffffff77ff7fbfff7fbf7fdfffffffffef9ffffffffffeffbfffffffeefbfffffbfffffffdffffdeff',
  aggregaredPubkey: '0x8cb81d775883fcd8e6f9f32da1ce6d3e74a958aa6adbaf4402dc90a8bdb718fe202d894d10b4d62653b51e0ffff682b9'
}
```
The function `verifyAggregateSignature()` can be used to confirm the signature is valid.

#### Example 2: Fetch Block Proof Points - Sepolia #3434343
`npm run cli -- fetchBlockProofPoints -b 3434343 -r https://your-secret-sepolia-beacon-endpoint.com`

Returns:

```js
{
  sigx0: '2717654981033153384371002073109890108758842394922743387082180563420439571270475913025321986075025323716183278996647',
  sigx1: '3245683462426865563976215644211420631990628660398343262203501487531719433413082187443407207330314139968454595001957',
  sigy0: '2989423361219887010340602378842184712180956146851149327633000413492124172889934354047857860025839410798624825057865',
  sigy1: '364270301102007508604236414822913048770830799933086997082647176198028599859549106697093613457344305038067424683642',
  pkx: '1957663992887248273038721973612762572179510549810033620204590218948159893374339423935740985951737943879228199436985',
  pky: '337064653469649321059724378161505517480755454067531273175739420804454452913499694660046803110495723569852414811101',
  msgx0: '2376836005588823590832355686757551839806976611876938693520937831890474521087776986139112416473585415180961492929156',
  msgx1: '1206179909977510143702043790768032354203103146376304321938440279014752549848875845657875840476089206457626131805163',
  msgy0: '1098650632560096473256354234109145535790592340280843578325165666754124094423343926482992304695649017941057567932801',
  msgy1: '978459796102980576570505777445808813685038478608728143767319725825501583173045652828825481621594634090727144659837'
}
```

These points can then be passed as arguments to the cairo program to run the signature verification.

## G1 and G2 Curve Points:
The BLS12-381 parameters are represented as points on an elliptic curve, either in G1 or G2. This repository contains some classes to handle the conversion between the raw data and the curve points. In Ethereum, the message and signature are represented in G2, while the public key is represented in G1.


### Message (G2):
The message of the signature in Ethereum is not the block hash directly, but the signing root. This includes the block hash, but adds a domain to the message. Using `hashToCurve()` the message bytes can be converted to a point on G2.

### Signature (G2):
The signature is a point on G2 by default, as its generated via the BLS signature scheme. To verify it, it must be decoded from bytes to a point on G2.

### Public Key (G1):
The public key is a point on G1 by defaul, and must be converted from bytes to a point on G1 to verify the signature.