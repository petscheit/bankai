%builtins range_check
from starkware.cairo.common.registers import get_label_location
from src.bls12_381.g1 import G1Point, g1
from src.bls12_381.g2 import G2Point, g2, E4
from src.bls12_381.towers.e12 import E12, e12
from src.bls12_381.towers.e2 import E2, e2
from src.bls12_381.towers.e6 import E6, e6
from src.bls12_381.curve import CURVE, BASE, DEGREE, N_LIMBS, P0, P1, P2, P3
from src.bls12_381.fq import BigInt4, fq_bigint4
from starkware.cairo.common.registers import get_fp_and_pc
from src.bls12_381.pairing import get_loop_digit, final_exponentiation, miller_loop

func main{range_check_ptr} (){
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
        def rsetattr(obj, attr, val):
            pre, _, post = attr.rpartition('.')
            return setattr(rgetattr(obj, pre) if pre else obj, post, val)

        def fill_element(element:str, value:int):
            s = split(value)
            print(s)
            for i in range(ids.N_LIMBS): rsetattr(ids,element+'.d'+str(i),s[i])

        def print_e12(param):
            z0 = param.c0.b0.a0.d0 + param.c0.b0.a0.d1 * 2**96 + param.c0.b0.a0.d2 * 2**192 + param.c0.b0.a0.d3 * 2**288
            z1 = param.c0.b0.a1.d0 + param.c0.b0.a1.d1 * 2**96 + param.c0.b0.a1.d2 * 2**192 + param.c0.b0.a1.d3 * 2**288
            z2 = param.c0.b1.a0.d0 + param.c0.b1.a0.d1 * 2**96 + param.c0.b1.a0.d2 * 2**192 + param.c0.b1.a0.d3 * 2**288
            z3 = param.c0.b1.a1.d0 + param.c0.b1.a1.d1 * 2**96 + param.c0.b1.a1.d2 * 2**192 + param.c0.b1.a1.d3 * 2**288
            z4 = param.c0.b2.a0.d0 + param.c0.b2.a0.d1 * 2**96 + param.c0.b2.a0.d2 * 2**192 + param.c0.b2.a0.d3 * 2**288
            z5 = param.c0.b2.a1.d0 + param.c0.b2.a1.d1 * 2**96 + param.c0.b2.a1.d2 * 2**192 + param.c0.b2.a1.d3 * 2**288
            z6 = param.c1.b0.a0.d0 + param.c1.b0.a0.d1 * 2**96 + param.c1.b0.a0.d2 * 2**192 + param.c1.b0.a0.d3 * 2**288
            z7 = param.c1.b0.a1.d0 + param.c1.b0.a1.d1 * 2**96 + param.c1.b0.a1.d2 * 2**192 + param.c1.b0.a1.d3 * 2**288
            z8 = param.c1.b1.a0.d0 + param.c1.b1.a0.d1 * 2**96 + param.c1.b1.a0.d2 * 2**192 + param.c1.b1.a0.d3 * 2**288
            z9 = param.c1.b1.a1.d0 + param.c1.b1.a1.d1 * 2**96 + param.c1.b1.a1.d2 * 2**192 + param.c1.b1.a1.d3 * 2**288
            z10 = param.c1.b2.a0.d0 + param.c1.b2.a0.d1 * 2**96 + param.c1.b2.a0.d2 * 2**192 + param.c1.b2.a0.d3 * 2**288
            z11 = param.c1.b2.a1.d0 + param.c1.b2.a1.d1 * 2**96 + param.c1.b2.a1.d2 * 2**192 + param.c1.b2.a1.d3 * 2**288

            res = [z0,z1,z2,z3,z4,z5,z6,z7,z8,z9,z10,z11]

            for i in range(12):
                print(f"z{i} = {np.base_repr(res[i],16)}")

        def print_g2(id):
            x0 = id.x.a0.d0 + id.x.a0.d1 * 2**96 + id.x.a0.d2 * 2**192 + id.x.a0.d3 * 2**288
            x1 = id.x.a1.d0 + id.x.a1.d1 * 2**96 + id.x.a1.d2 * 2**192 + id.x.a1.d3 * 2**288
            y0 = id.y.a0.d0 + id.y.a0.d1 * 2**96 + id.y.a0.d2 * 2**192 + id.y.a0.d3 * 2**288
            y1 = id.y.a1.d0 + id.y.a1.d1 * 2**96 + id.y.a1.d2 * 2**192 + id.y.a1.d3 * 2**288

            print(f"X={np.base_repr(x0,16).lower()} + {np.base_repr(x1,16).lower()}*u")
            print(f"Y={np.base_repr(y0,16).lower()} + {np.base_repr(y1,16).lower()}*u")

        def print_g1(id):
            x0 = id.x.d0 + id.x.d1 * 2**96 + id.x.d2 * 2**192 + id.x.d3 * 2**288
            y0 = id.y.d0 + id.y.d1 * 2**96 + id.y.d2 * 2**192 + id.y.d3 * 2**288

            print(f"X={np.base_repr(x0,16).lower()}")
            print(f"Y={np.base_repr(y0,16).lower()}")

    %}
    let (__fp__, _) = get_fp_and_pc();
    local msgx0: BigInt4;
    local msgx1: BigInt4;
    local msgy0: BigInt4;
    local msgy1: BigInt4;

    local sigx0: BigInt4;
    local sigx1: BigInt4;
    local sigy0: BigInt4;
    local sigy1: BigInt4;

    local pubx: BigInt4;
    local puby: BigInt4;

    %{
        for var_name in list(program_input.keys()):
            if type(program_input[var_name]) == str : continue
            print(var_name)
            print(program_input[var_name])
            if var_name == "pub":
                fill_element(var_name+"x", int(program_input[var_name]["x"]))
                fill_element(var_name+"y", int(program_input[var_name]["y"]))
                print("Filled PK")
            elif var_name == "msg" or var_name == "sig":
                fill_element(var_name+"x0", int(program_input[var_name]["x0"]))
                fill_element(var_name+"x1", int(program_input[var_name]["x1"]))
                fill_element(var_name+"y0", int(program_input[var_name]["y0"]))
                fill_element(var_name+"y1", int(program_input[var_name]["y1"]))
    %}

    // Construct PK
    local pub: G1Point = G1Point(&pubx, &puby);
    g1.assert_on_curve(pub);

    // Construct Inverse G1 generator point (-G1)
    local x: BigInt4 = BigInt4(
        77209383603911340680728987323,
        49921657856232494206459177023,
        24654436777218005952848247045,
        7410505851925769877053596556
    );
    local y: BigInt4 = BigInt4(
        4578755106311036904654095050,
        31671107278004379566943975610,
        64119167930385062737200089033,
        5354471328347505754258634440
    );
    local g1_gen_neg: G1Point = G1Point(&x,&y);
    g1.assert_on_curve(g1_gen_neg);

    // Construct Signature
    local sigx: E2 = E2(&sigx0, &sigx1);
    local sigy: E2 = E2(&sigy0, &sigy1);
    local sig: G2Point = G2Point(&sigx, &sigy);
    g2.assert_on_curve(sig);

    // Construct Message
    local msgx: E2 = E2(&msgx0, &msgx1);
    local msgy: E2 = E2(&msgy0, &msgy1);
    local msg: G2Point = G2Point(&msgx, &msgy);
    g2.assert_on_curve(msg);

    %{
        print("pub");
        print_g1(ids.pub);

        print("-G1")
        print_g1(ids.g1_gen_neg);

        print("MSG")
        print_g2(ids.msg);

        print("SIG")
        print_g2(ids.sig);
    %}

    let c1 = miller_loop(&g1_gen_neg, &sig);
    let c2 = miller_loop(&pub, &msg);

    %{
        print("e(-G1, SIG)")
        print_e12(ids.c1)

        print("e(PK, SIG)")
        print_e12(ids.c2)

    %}

    let m = e12.mul(c1, c2);
    let res = final_exponentiation(m);
    let one = e12.one();

    e12.assert_E12(res, one);

    %{ print(f"Signature Verified!") %}
    return ();
}
