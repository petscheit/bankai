from definitions import UInt384, G1Point

func g1_negative() -> G1Point {
    return (G1Point(
        x=UInt384(
            77209383603911340680728987323,
            49921657856232494206459177023,
            24654436777218005952848247045,
            7410505851925769877053596556
        ),
        y=UInt384(
            4578755106311036904654095050,
            31671107278004379566943975610,
            64119167930385062737200089033,
            5354471328347505754258634440
        )
    ));
}