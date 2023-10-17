import { bls } from "./bls.js"

export const G1 = bls.PointG1;
export const G2 = bls.PointG2;
export class G2Point {
	name: string;
	value: bls.PointG2;

	constructor(name: string) {
		this.name = name;
	}

	async fromBytes(hex: string | Uint8Array) {
		if(typeof hex == "string") hex = hex.replace("0x", "")
		this.value = await bls.PointG2.hashToCurve(hex);
		return this;
	}

	fromPoint(value: bls.PointG2) {
		this.value = value;
		return this;
	}

	toDecimals(): any {
		let hex = this.toHexObject()
		let decimal = {};
		Object.keys(hex).forEach((key) => {
			decimal[key] = BigInt(hex[key]).toString();
		});
		return decimal;
	}

	toHexObject() {
		let val = this.value.toHex();
		if (val.length != 384) throw new Error("Invalid hex length");
		let points = splitPoints(val);
		return {
			[this.name + "x0"]: "0x" + points[1], // noble switches the print order of x0 and x1
			[this.name + "x1"]: "0x" + points[0],
			[this.name + "y0"]: "0x" + points[3],
			[this.name + "y1"]: "0x" + points[2],
		}
	}

	toHex() {
		return this.value.toHex();
	}
}

export class G1Point {
	value: bls.PointG1;
	name: string;

	constructor(name: string) {
		this.name = name;
	}

	toDecimals(): any {
		let hex = this.toHexObject()
		let decimal = {};
		Object.keys(hex).forEach((key) => {
			decimal[key] = BigInt(hex[key]).toString();
		});
		return decimal;
	}

	toHexObject() {
		let val = this.value.toHex();
		if (val.length != 192) throw new Error("Invalid hex length");
		let points = splitPoints(val);
		return {
			[this.name + "x"]: "0x" + points[0],
			[this.name + "y"]: "0x" + points[1]
		}
	}


}

export { }

const splitPoints = (hex: string, chunkSize: number = 96) => {
	let chunks = [];

	for (var i = 0, len = hex.length; i < len; i += chunkSize) {
        chunks.push(hex.slice(i, i + chunkSize));
    }
	return chunks
}