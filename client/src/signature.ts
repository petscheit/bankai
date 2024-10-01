import {Message} from "./message.js";
import {bls} from "./bls.js";
import {G2Point} from "./points.js";
import {PublicKey} from "./pubkey.js";

export class Signature extends G2Point {
	raw: Uint8Array | string;

	constructor() {
		super("sig");
	}

	async fromBytes(hex: string | Uint8Array) {
		if(typeof hex == "string") hex = hex.replace("0x", "")
		this.value = await bls.PointG2.fromSignature(hex);
		return this;
	}

	fromPoint(value: bls.PointG2) {
		this.value = value;
		return this;
	}

	async sign(msg: Message, priv: string) {
		this.value = await bls.sign(msg.value, priv);
		this.raw = this.value.toSignature();
	}

	async verify(msg: Message, pub: PublicKey): Promise<boolean> {
		return bls.verify(this.value, msg.value, pub.value);
	}
}