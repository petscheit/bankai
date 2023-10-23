%builtins range_check
from starkware.cairo.common.registers import get_fp_and_pc
from starkware.cairo.common.alloc import alloc
from starkware.cairo.common.serialize import serialize_word
from deps.garaga.src.bls12_381.g1 import G1PointFull, g1
from deps.garaga.src.bls12_381.fq import BigInt4
from deps.garaga.src.bls12_381.curve import CURVE, BASE, DEGREE, N_LIMBS, P0, P1, P2, P3

func main{range_check_ptr}() {
    alloc_locals;
     %{
        import subprocess, random, functools, re
        import numpy as np
        def get_p(n_limbs:int=ids.N_LIMBS):
            p=0
            for i in range(n_limbs):
                p+=getattr(ids, 'P'+str(i)) * ids.BASE**i
            return p
        P=p=get_p()
        def split(x, degree=ids.DEGREE, base=ids.BASE):
            coeffs = []
            for n in range(degree, 0, -1):
                q, r = divmod(x, base ** n)
                coeffs.append(q)
                x = r
            coeffs.append(x)
            return coeffs[::-1]
        def rgetattr(obj, attr, *args):
            def _getattr(obj, attr):
                return getattr(obj, attr, *args)
            return functools.reduce(_getattr, [obj] + attr.split('.'))

        def fill_g1_point_full_array(ptr, array):
            counter = 0
            for point in array:
                memory[ptr._reference_value + counter] = point[0][0]
                memory[ptr._reference_value + counter + 1] = point[0][1]
                memory[ptr._reference_value + counter + 2] = point[0][2]
                memory[ptr._reference_value + counter + 3] = point[0][3]
                memory[ptr._reference_value + counter + 4] = point[1][0]
                memory[ptr._reference_value + counter + 5] = point[1][1]
                memory[ptr._reference_value + counter + 6] = point[1][2]
                memory[ptr._reference_value + counter + 7] = point[1][3]
                counter += 8


        def print_g1(id):
            x0 = id.x.d0 + id.x.d1 * 2**96 + id.x.d2 * 2**192 + id.x.d3 * 2**288
            y0 = id.y.d0 + id.y.d1 * 2**96 + id.y.d2 * 2**192 + id.y.d3 * 2**288

            print(f"X={np.base_repr(x0,16).lower()}")
            print(f"Y={np.base_repr(y0,16).lower()}")
    %}

    let (__fp__, _) = get_fp_and_pc();

    local total_signers: felt;

    let (pubs: G1PointFull*) = alloc();
    %{
        args = []
        for var_name in list(program_input.keys()):
            if type(program_input[var_name]) == str : continue
            if var_name == 'totalSigners':
                ids.total_signers = program_input[var_name]
                print("Loading public keys for " + str(program_input[var_name]) + " signers")
            elif var_name == 'signers':
                for signer in program_input[var_name]:
                    splitX = split(int(program_input[var_name][signer]['x']))
                    splitY = split(int(program_input[var_name][signer]['y']))
                    args.append([splitX, splitY])

        fill_g1_point_full_array(ids.pubs, args)
    %}

    let (local aPk) = aggregate_keys(pubs=pubs, size=total_signers);

    %{
        print("Aggregate Public Key Point:")
        print_g1(ids.aPk)
        print("_____________")
    %}
    return ();
}

func aggregate_keys{range_check_ptr}(pubs: G1PointFull*, size) -> (res: G1PointFull) {
    alloc_locals;

    if (size == 1) {
        return ([pubs], );
    }

    let (local res) = aggregate_keys(pubs + G1PointFull.SIZE, size - 1);

    return g1.fast_ec_add(res, pubs[0]);
}
