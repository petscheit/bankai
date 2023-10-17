import {BeaconClient} from "./beaconClient.js";
import {verifyAggregateSignature} from "./utils/beacon.js";
import {Message} from "./message.js";
import {PublicKey} from "./pubkey.js";
import {Signature} from "./signature.js";
import { program } from 'commander';
import * as fs from 'fs';

async function fetchBlockProof(blockId: number | string, rpc: string) {
	let client = new BeaconClient(rpc);
	return client.getBlockProof(blockId);
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
		...sigPoints,
		...pubPoints,
		...msgPoints
	}

	return result
	
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

program.parse(process.argv);