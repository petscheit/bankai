Cairo Ethereum Consensus Verification
The long term goal of this repository is to enable the verification of Ethereum blocks in the Cairo language. This requires a number of cryptographic operations which will be added step by step. Currently, a blocks headers signature can be verified in cairo, which is the first step. Below is a quick overview of the steps required to verify a block header, and the steps that are currently implemented.

## Table of Contents
- [Cairo Ethereum Consensus Verification](#cairo-ethereum-consensus-verification)
  - [Background: Steps to verify an Ethereum block](#background-steps-to-verify-an-ethereum-block)
    - [Verify Sync Committee Signature](#verify-sync-committee-signature)
    - [Update Sync Committee](#update-sync-committee)
  - [Getting Started](#getting-started)
    - [CLI](#cli)
      - [Fetch Block Proof](#fetch-block-proof)
      - [Fetch Block Proof Points](#fetch-block-proof-points)
      - [Fetch Block Signers](#fetch-block-signers)
  - [Cairo Programs](#cairo-programs)
    - [Verify Block Signature](#verify-block-signature)
      - [Example](#example)
      - [ToDo](#todo)
    - [Aggregate Public Key](#aggregate-public-key)
      - [Example](#example-1)
      - [ToDo](#todo-1)
  - [G1 and G2 Curve Points](#g1-and-g2-curve-points)
    - [Message (G2)](#message-g2)
    - [Signature (G2)](#signature-g2)
    - [Public Key (G1)](#public-key-g1)


## Background: Steps to verify an Ethereum block:
A quick overview of the steps required to verify an Ethereum block. Two different operations are required:

### Verify Sync Committee Signature

Required State: Valid Sync committee of the block

- [x] 1. Fetch block hash (client)
- [x] 2. Generate the signing root of the block (client)
- [x] 3. Convert the signing root to a point on G2 (client)
- [x] 4. Generate aggregated public key of block signers (cairo)
- [ ] 5. Ensure signers are in the sync committee and have >2/3 majority
- [x] 6. Verify signature (cairo)

To make this verification trustless, step 2-6 must be done in a cairo program.

### Update Sync Committee:
Required State: Verified header containing the new sync committee (verified by the above process)

- [ ] 1. Generate a state inclusion proof for the new sync committee
- [ ] 2. Recreate beacon chain state root via SSZ merkleization
- [ ] 3. Store new sync committee for respective epochs

## Getting Started
- run `npm i` to install all dependencies
- run `npm run setup` to setup the local cairo environment

### CLI
The CLI can be used to fetch the required parameters for the verification. To use the CLI a beacon chain RPC endpoint is required. Currently, quicknode offers a free endpoint that can be used for testing. To use the CLI, run `npm run cli -- <command> <args>`. The following commands are available:

#### Fetch Block Proof
This command is not required for using the cairo programs, but can be useful for debugging. It fetches all required parameters for verifying the signature of a block and is hex encoded, making it easier to deal with.

Parameters:
- `-b` or `--block` - The block number to fetch the proof for
- `-r` or `--rpc` - The RPC endpoint to fetch the proof from
- `-e` or `--export` - Exports the output as a JSON file

`npm run cli -- fetchBlockProof -b 3434343 -r https://your-secret-sepolia-beacon-endpoint.com -e proof.json`

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


#### Fetch Block Proof Points
Fetches and generates the data required for running the signature check in cairo. A couple of things happen under the hood:
- Generates the signing root of the block, by applying the domaining logic to the block hash
- Fetches the sync committee signers, and generates the aggregated public key
- Fetches the signature of the block header
- Converts all values to (G1/G2) points on the curve

These values can then be used as inputs for the cairo verification program.

Parameters:
- `-b` or `--block` - The block number to fetch the proof for
- `-r` or `--rpc` - The RPC endpoint to fetch the proof from
- `-e` or `--export` - Exports the output as a JSON file

`npm run cli -- fetchBlockProofPoints -b 3434343 -r https://your-secret-sepolia-beacon-endpoint.com -e MY_INPUTS.json`

Returns:

```js
{
  sig: {
    x0: '2717654981033153384371002073109890108758842394922743387082180563420439571270475913025321986075025323716183278996647',
    x1: '3245683462426865563976215644211420631990628660398343262203501487531719433413082187443407207330314139968454595001957',
    y0: '2989423361219887010340602378842184712180956146851149327633000413492124172889934354047857860025839410798624825057865',
    y1: '364270301102007508604236414822913048770830799933086997082647176198028599859549106697093613457344305038067424683642'
  },
  pub: {
    x: '1957663992887248273038721973612762572179510549810033620204590218948159893374339423935740985951737943879228199436985',
    y: '337064653469649321059724378161505517480755454067531273175739420804454452913499694660046803110495723569852414811101'
  },
  msg: {
    x0: '2376836005588823590832355686757551839806976611876938693520937831890474521087776986139112416473585415180961492929156',
    x1: '1206179909977510143702043790768032354203103146376304321938440279014752549848875845657875840476089206457626131805163',
    y0: '1098650632560096473256354234109145535790592340280843578325165666754124094423343926482992304695649017941057567932801',
    y1: '978459796102980576570505777445808813685038478608728143767319725825501583173045652828825481621594634090727144659837'
  }
}
```

#### Fetch Block Signers:
Fetches and returns a list of all the sync committe members public keys that signed a specific block. The order of the keys is important for aggregating the public key later on. The resulting values are encoded as G1 points, and can be used as inputs for the cairo program. The command also returns the committee aggregate (all members), the signers aggregate (all signers) and the non-signers aggregate (all non-signers). These values are not required for the verification, but can be useful for debugging.

Parameters:
- `-b` or `--block` - The block number to fetch the proof for
- `-r` or `--rpc` - The RPC endpoint to fetch the proof from
- `-e` or `--export` - Exports the output as a JSON file

`npm run cli -- fetchBlockSigners -b 3434343 -r https://your-secret-sepolia-beacon-endpoint.com -e MY_INPUTS.json`

Returns:
```js
{
  totalSigners: 480,
  signers: {
    pk0: {
      x: '327760309966947479300718637057990157694893189073459534177163952376017427978170470717349116193385194069106752359760',
      y: '432766611839774662871458728216626394713771940885954716148704575376319526233897834864607974129351702495035260326023'
    },
    pk1: {
      x: '1427380063432123607512340653230045875355175784259329042437673993960025718252096591713666287746656641099748193141546',
      y: '730333754263170048824130221636284292353056570341776516393879644972004546278474496120959327494584262973772166574531'
    },
    ...
    pk479: {
      x: '2986200977995358693501988905818968241864897212637291820947499821072408806050376725972204506692328269651102950621711',
      y: '317884619498278685450994081234396718788211723448541288494462531516736381680362608624234807353666199972882194921898'
    }
  },
  aggregates: {
    committee: '0xaa6d2746b6f607fcb807db56def0dd08aa4a0cd3202d6f4ca4999cc55f58fb97c6938823b199d816d232ac5c6eb08d99',
    signers: '0x8cb81d775883fcd8e6f9f32da1ce6d3e74a958aa6adbaf4402dc90a8bdb718fe202d894d10b4d62653b51e0ffff682b9',
    nonSigners: '0x934008434c03c35332841f1b1675532c54c214e855215fc77cd1b2371330049a8d68f310e75c36ebce62349b7fed8f1f'
  }
}
```

## Cairo Programs
At the moment, this repo contains two different cairo programs that handle a substep of the entire verification. 

### Verify Block Signature:
Verifies the BLS12-381 Signature of a given message and public key. These values can be fetched from a beacon endpoint via the CLI.

Inputs:
- Signing Root Point (G2)
- Signature Point (G2)
- Aggregated Signers Key Point (G1)

The signature is verified by comparing the following pairings:
```
e(P, H(m)) = e(G,S)
```

This works because:
```
e(P, H(m)) = e(pk x G, H(m))
  = e(G, pk x H(m))
  = e(G, S)
```

#### Example: 
- run `npm run cairo-compile:verify_sig` to compile the cairo program
- run `npm run cairo-run:verify_sig -- <MY_INPUTS.json>` to run the program. You can use the data exported with the CLI as input. The program will return `true` if the signature is valid, and `false` if its invalid.

#### ToDo: 
- [ ] Verify negating G1 is equivilant to negating the aggregated public key.

### Aggregate Public Key:
This program can be used to aggregate the public key of a number of signers. This is required when verifying an Ethereum block, as the signature is checked against an aggregated key.

Inputs:
- Signer Points ([]G1)
- Total Signers

Returns: Aggregated Public Key Point (G1)

Since the public key is a point on G1, the aggregation is done by adding all points together. This is done by recursivly adding the points together. The order of the keys matter, so its important to pass the keys in the correct order.

#### Example:
- run `npm run cairo-compile:aggregate` to compile the cairo program
- run `npm run cairo-run:aggregate -- MY_INPUTS.json` to run the program. You can use the data exported with the CLI as input. The program will return the aggregated public key as a point on G1.

#### ToDo:
- [ ] Instead of adding all signers, we can use the aggregated committee key, and subtract all non-signers. This would reduce the number of operations required significantly. Need to implement subtraction in garaga first

## G1 and G2 Curve Points:
The BLS12-381 parameters are represented as points on an elliptic curve, either in G1 or G2. This repository contains some classes to handle the conversion between the raw data and the curve points. In Ethereum, the message and signature are represented in G2, while the public key is represented in G1.


### Message (G2):
The message of the signature in Ethereum is not the block hash directly, but the signing root. This includes the block hash, but adds a domain to the message. Using `hashToCurve()` the message bytes can be converted to a point on G2.

### Signature (G2):
The signature is a point on G2 by default, as its generated via the BLS signature scheme. To verify it, it must be decoded from bytes to a point on G2.

### Public Key (G1):
The public key is a point on G1 by defaul, and must be converted from bytes to a point on G1 to verify the signature.