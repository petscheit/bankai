import {bls} from "./bls.js";
import {G1Point} from "./points.js";

export class PublicKey extends G1Point {

	raw: string | Uint8Array;
	constructor() {
		super("pk");
	}

	fromPrivateKey(priv: string | Uint8Array) {
		this.raw = bls.getPublicKey(priv);
		this.fromBytes(this.raw);
	}

	fromBytes(hex: string | Uint8Array) {
		if(typeof hex == "string") hex = hex.replace("0x", "")
		this.value = bls.PointG1.fromHex(hex);
		this.raw = this.value.toRawBytes(true)
		return this;
	}

	fromRaw(value: bls.PointG1) {
		this.value = value;
		this.raw = this.value.toRawBytes(true)
		return this;
	}

	aggregateSubtract(pubkeys: PublicKey[]) {
		let result = this.value;
		for(let i = 0; i < pubkeys.length; i++) {
			result = result.subtract(pubkeys[i].value);
		}
		return new PublicKey().fromRaw(result);
	}

	aggregateAdd(pubkeys: PublicKey[]) {
		return new PublicKey().fromRaw(bls.aggregatePublicKeys(pubkeys.map((x) => x.value)))
	}

}