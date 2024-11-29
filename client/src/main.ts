import {BeaconClient} from "./beaconClient.js";
import {aggregatePubkey, decodeSignerBits, verifyAggregateSignature} from "./utils/beacon.js";
import {Message} from "./message.js";
import {PublicKey} from "./pubkey.js";
import {Signature} from "./signature.js";
import { program } from 'commander';
import * as fs from 'fs';
import {bls} from "./bls.js";
import {toHexString} from "@chainsafe/ssz";
import {ssz} from "@lodestar/types";

async function fetchEpochProof(epoch: number, rpc: string) {
	let client = new BeaconClient(rpc);
	// Fetch the first block of the following epoch, as this seal the targeted epoch
	const slot = (epoch + 1) * 32
	let blockProof = await client.getBlockProof(slot)
	console.log("Block Proof: ", blockProof)
	const header = await client.getHeader(slot)
	const valid = await client.verifyBlockProof(blockProof)
	console.log("Proof verifies:", valid)
	const syncCommittee = await client.getSyncCommitteeSignature(slot)
	const validatorPubs = await client.getSyncCommitteeValidatorPubs(slot)
	const signerBits = decodeSignerBits(syncCommittee.signerBits);
	const signers = validatorPubs.filter((_, i) => signerBits[i] === true).map((x) => new PublicKey().fromBytes(x).toHexObject())
	const nonSigners = validatorPubs.filter((_, i) => signerBits[i] === false).map((x) => new PublicKey().fromBytes(x).toHexObject())

	const result = {
		header: header["header"]["message"],
		signature_point: (await new Signature().fromBytes(blockProof.signature)).toHexObject(),
		sync_committee_agg_pub: new PublicKey().fromBytes(bls.aggregatePublicKeys(validatorPubs.map((x) => x.replace("0x", "")))).toHexObject(),
		non_signers: nonSigners,
	}

	return result
}

async function fetchBlockProof(blockId: number | string, rpc: string) {
	let client = new BeaconClient(rpc);
	const blockProof =  await client.getBlockProof(blockId);
	const valid = await client.verifyBlockProof(blockProof)
	console.log("Proof verifies:", valid)
	return blockProof
}

async function fetchBlockProofPoints(blockId: number | string, rpc: string) {
	let client = new BeaconClient(rpc);
	const blockProof = await fetchBlockProof(blockId, rpc);

	const msg = await new Message(blockProof.signingRoot).hashToCurve();
	const signature = await new Signature().fromBytes(blockProof.signature);
	const publicKey = new PublicKey().fromBytes(blockProof.aggregaredPubkey);
	
	const valid = await signature.verify(msg, publicKey);

	const sigPoints = signature.toDecimals();
	const pubPoints = publicKey.toDecimals();
	const msgPoints = msg.toDecimals();

	const result = {
		sig: sigPoints,
		pub: pubPoints,
		msg: msgPoints,
	}

	return result
}

async function fetchBlockSigners(blockId: number | string, rpc: string) {
	let client = new BeaconClient(rpc);
	const block = await client.getBlock(blockId)
	const slot = ssz.Slot.fromJson(block.message.slot) // converts to number
	const syncCommittee = await client.getSyncCommitteeSignature(slot)
	const validatorPubs = await client.getSyncCommitteeValidatorPubs(block.message.slot)
	const signerBits = decodeSignerBits(syncCommittee.signerBits);
	const signers = validatorPubs.filter((_, i) => signerBits[i] === true)

	let res = {
		totalSigners: signers.length,
		signers: {},
		aggregates: {
			committee: toHexString(bls.aggregatePublicKeys(validatorPubs.map((x) => x.replace("0x", "")))),
			signers: aggregatePubkey(validatorPubs, syncCommittee.signerBits),
			nonSigners: aggregatePubkey(validatorPubs, syncCommittee.signerBits, false)
		}
	};

	const aggregateAddResult = new PublicKey().fromBytes(res.aggregates.committee).aggregateAdd(signers.map((x) => new PublicKey().fromBytes(x)));

	for (let i = 0; i < signers.length; i++) {
		const decimals = new PublicKey().fromBytes(signers[i]).toDecimals();

		res["signers"][`pk${i}`] = decimals
	}

	return res
}

function exportToJsonFile(filename: string, data: any) {
	let jsonString = JSON.stringify(data, null, 4);
	jsonString = jsonString.replace(/"(-?\d+)n"/g, '$1');
	fs.writeFileSync(filename, jsonString, 'utf8');
	console.log(`Data has been written to ${filename}`);
}

program
	.version('0.1.0')
	.description('Cairo Ethereum Consensus Verification Utils');

program
	.command('fetchBlockProof')
	.description('Fetch proof for a blockm, containing everything needed for verification.')
	.requiredOption('-b, --block <string | number>', 'Block hash or slot number.')
	.requiredOption('-r, --rpc <string>', 'Beacon Chain RPC endpoint. (Quicknode free-tier recommended)')
	.option('-e, --export <path>', 'Path to export the results as a JSON file.')
	.action(async (cmdObj) => {
		const { block, rpc, export: exportPath } = cmdObj;

		console.log(`Fetching block proof: ${block}`);
		const result = await fetchBlockProof(block, rpc);
		console.log(result)

		if (exportPath) {
			exportToJsonFile(exportPath, result);
		}

	});

program
	.command('fetchBlockProofPoints')
	.description('Fetch the proof points needed for verification in cairo. This handles all preprocessing and exports in garaga conpatible decimals.')
	.requiredOption('-b, --block <string | number>', 'Block hash or slot number.')
	.requiredOption('-r, --rpc <string>', 'Beacon Chain RPC endpoint. (Quicknode free-tier recommended)')
	.option('-e, --export <path>', 'Path to export the results as a JSON file.')
	.action(async (cmdObj) => {
		const { block, rpc, export: exportPath } = cmdObj;

		console.log(`Fetching block proof points: ${block}`);
		const result = await fetchBlockProofPoints(block, rpc);
		console.log(result)

		if (exportPath) {
			exportToJsonFile(exportPath, result);
		}
	});

program
	.command('fetchBlockSigners')
	.description('Fetches the signers of the block.')
	.requiredOption('-b, --block <string | number>', 'Block hash or slot number.')
	.requiredOption('-r, --rpc <string>', 'Beacon Chain RPC endpoint. (Quicknode free-tier recommended)')
	.option('-e, --export <path>', 'Path to export the results as a JSON file.')
	.action(async (cmdObj) => {
		const { block, rpc, export: exportPath } = cmdObj;

		console.log(`Fetching block singers: ${block}`);
		const result = await fetchBlockSigners(block, rpc);
		console.log(result)

		if (exportPath) {
			exportToJsonFile(exportPath, result);
		}

	});

program
	.command('fetchEpochProof')
	.description('Fetch the proof for an epoch.')
	.requiredOption('-e, --epoch <number>', 'Epoch number.')
	.requiredOption('-r, --rpc <string>', 'Beacon Chain RPC endpoint. (Quicknode free-tier recommended)')
	.option('-x, --export <path>', 'Path to export the results as a JSON file.')
	.action(async (cmdObj) => {
		const { epoch, rpc, export: exportPath } = cmdObj;

		const result = await fetchEpochProof(parseInt(epoch), rpc);

		if (exportPath) {
			exportToJsonFile(exportPath, result);
		}
	})


program.parse(process.argv);