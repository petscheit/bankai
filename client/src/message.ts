import {G2Point, G2} from "./points.js";
import {bls} from "./bls.js";

export class Message extends G2Point {
	raw: Uint8Array | string;

	constructor(msg: string | Uint8Array) {
		super("msg");
		if(typeof msg == "string") msg = msg.replace("0x", "")

		this.raw = msg;

	}

	fromPoint(value: bls.PointG2) {
		this.value = value;
		return this;
	}

	async hashToCurve() {
		this.fromPoint( await G2.hashToCurve(this.raw));
		return this
	}
}