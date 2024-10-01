import { createBeaconConfig, } from "@lodestar/config";
import { fromHexString, toHexString } from "@chainsafe/ssz";
import axios from "axios";
import { ssz } from "@lodestar/types";
import { networksChainConfig } from "@lodestar/config/networks";
import bls from "@chainsafe/bls";

export const fetchGenesisValidatorRoot = async (rpc: string) => {
    let res = await axios.get(rpc + "/eth/v1/beacon/genesis")
        .then(res => res)
    return res.data.data.genesis_validators_root
}
export const getDomain = async (slot: number, domain: any, rpc: string) => {
    const chainConfig = networksChainConfig.sepolia;
    const valRoot = await fetchGenesisValidatorRoot(rpc);
    const config = createBeaconConfig(chainConfig, fromHexString(valRoot));

    return config.getDomain(slot, domain);
}

export const generateSigningRoot = async (slot: number, root: string, domainId: any, rpc: string) => {
    const domain = await getDomain(slot, domainId, rpc);

    const signingRoot = ssz.phase0.SigningData.hashTreeRoot({
        objectRoot: fromHexString(root),
        domain
    })

    return signingRoot;
}

export const aggregatePubkey = (pubkeys: string[], signerBits: string, signers: boolean = true): string => {
    const signerBitsArray = decodeSignerBits(signerBits);
    const signed = signerBitsArray.filter((_, i) => signerBitsArray[i] === signers)

    const pubkeysArray = pubkeys.map((x) => Buffer.from(x.replace("0x", ""), "hex"));
    const aggPubkey = bls.aggregatePublicKeys(pubkeysArray.filter((_, i) => signerBitsArray[i] === signers));
    return toHexString(aggPubkey);
}

export const decodeSignerBits = (signerBits: string): boolean[] => {
    signerBits = signerBits.replace("0x", "");

    // split into byte groups
    const bytes = signerBits.match(/.{1,2}/g) ?? [];
    let acc: boolean[][] = []
    for (let i = 0; i < bytes.length; i++) {
        let binaries = (parseInt(bytes[i], 16)
            .toString(2))
            .split('')
            .reverse()
            .map((x) => parseInt(x) === 1 ? true : false);

        // pad remaining 0s
        while (binaries.length < 8) {
            binaries.push(false)
        }
        acc.push(binaries);
    }

    return acc.flat()
}

export const verifyAggregateSignature = (aggrPub: string, msg: string, sig: string): boolean => {
    const aggrPubBytes = fromHexString(aggrPub);
    const msgBytes = fromHexString(msg);
    const sigBytes = fromHexString(sig);
    return bls.verify(aggrPubBytes, msgBytes, sigBytes);
}