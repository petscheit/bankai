import axios from "axios";
import {capella, Slot} from "@lodestar/types"
import { ssz } from "@lodestar/types";
import {toHexString} from "@chainsafe/ssz";
import {DOMAIN_SYNC_COMMITTEE} from "@chainsafe/lodestar-params";
import { generateSigningRoot, aggregatePubkey } from "./utils/beacon.js";
import {bls} from "./bls.js";

export type BeaconHeaderResponse = {
	root: string;
	canonical: boolean;
	header: {
		message: {
			slot: string;
			proposer_index: string;
			parent_root: string;
			state_root: string;
			body_root: string;
		}
		signature: string;
	}
}

export type BlockProof = {
	blockRoot: string;
	signingRoot: string;
	signature: string;
	signerBits: string;
	aggregaredPubkey: string;
}

export type SyncCommitteeSignature = {
	signature: string;
	signerBits: string;
}

export class BeaconClient {
	rpc: string;

	constructor(rpc: string) {
		this.rpc = rpc;
	}

	getHead(): Promise<BeaconHeaderResponse> {
		return this.getHeader("head");
	}

	async getBlockProof(blockId: string | number): Promise<BlockProof> {
		const block = await this.getBlock(blockId)
		const slot = ssz.Slot.fromJson(block.message.slot) // converts to number
		const syncCommittee = await this.getSyncCommitteeSignature(slot)	
		const signingRoot = await this.getSigningRoot(block)
		const validatorPubs = await this.getSyncCommitteeValidatorPubs(block.message.slot)
		const aggPubkey = aggregatePubkey(validatorPubs, syncCommittee.signerBits);

		return {
			blockRoot: await this.getHeader(blockId).then(resp => resp.root),
			signingRoot: toHexString(signingRoot),
			signature: syncCommittee.signature,
			signerBits: syncCommittee.signerBits,
			aggregaredPubkey: aggPubkey
		}
	}

	async getHeader(slotId: string | number): Promise<BeaconHeaderResponse> {
		const endpoint = `${this.rpc}/eth/v1/beacon/headers/${slotId}`
		let resp = await this.getRequest(endpoint) as BeaconHeaderResponse;
		return resp;
	}

	async getBlock(blockId: string | number): Promise<capella.SignedBeaconBlock> {
		const endpoint = `${this.rpc}/eth/v2/beacon/blocks/${blockId}`
		const resp = await this.getRequest(endpoint) as capella.SignedBeaconBlock;
		return resp;
	}

	async getSigningRoot(block: capella.SignedBeaconBlock) {
		const view = this.createView(ssz.capella.BeaconBlock, block.message);
		const root = toHexString(view.hashTreeRoot());
		return generateSigningRoot(block.message.slot, root, DOMAIN_SYNC_COMMITTEE, this.rpc);
	}

	async getSyncCommitteeSignature(slot: number): Promise<SyncCommitteeSignature> {
		const nextBlock = await this.getBlock(slot + 1);
		let body = ssz.capella.BeaconBlockBody.fromJson(nextBlock.message.body)
		const signature = toHexString(body.syncAggregate.syncCommitteeSignature)
		const signerBits = toHexString(ssz.altair.SyncCommitteeBits.toView(body.syncAggregate.syncCommitteeBits).uint8Array)
		return {signature, signerBits}
	}

	async getSyncCommitteeValidatorPubs(slot: Slot | number): Promise<string[]> {
		slot = ssz.Slot.fromJson(slot) + 1;
		const endpoint = `${this.rpc}/eth/v1/beacon/states/${slot}/sync_committees`
		const indexes = await this.getRequest(endpoint).then(resp => resp.validators);

		const pubEndpoint = `${this.rpc}/eth/v1/beacon/states/head/validators?${indexes.map((n: number) => "id=" + n).join('&')}`;

		const validators = await this.getRequest(pubEndpoint);

		// Nasty complexity, but we need to order the keys based on the indexes to aggregate correctly
		let result = [];
		for (let i = 0; i < indexes.length; i++) {
			const entry = validators.find((v: any) => v.index === indexes[i]);
			result.push(entry.validator.pubkey);
		}

		return result;
	}

	async verifyBlockProof(blockProof: BlockProof): Promise<boolean> {
		return bls.verify(
			blockProof.signature.replace("0x", ""),
			blockProof.signingRoot.replace("0x", ""),
			blockProof.aggregaredPubkey.replace("0x", "")
		);
	}

	// creates ssz view, which enables the standard ssz operations
	private createView(type: any, value: any) {
		return type.toView(type.fromJson(value))
	}

	private getRequest(endpoint: string) {
		return axios.get(endpoint,
			{
				headers: {
					'Content-Type': 'application/json',
					'Accept': '*/*'
				},
			})
			.then(resp => resp.data.data)
			.catch(err => {
				console.error("Error requesting:", endpoint)
				console.error(err.toString())
				process.exit(1)
			});
	}
}