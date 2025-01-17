import time

import flexitest

from envs import testenv
from utils import wait_until

REORG_DEPTH = 3


@flexitest.register
class CLBlockWitnessDataGenerationTest(testenv.StrataTester):
    def __init__(self, ctx: flexitest.InitContext):
        ctx.set_env("basic")

    def main(self, ctx: flexitest.RunContext):
        seq = ctx.get_service("sequencer")
        seqrpc = seq.create_rpc()

        # Wait for seq
        wait_until(
            lambda: seqrpc.strata_protocolVersion() is not None,
            error_with="Sequencer did not start on time",
        )

        time.sleep(1)
        ckp_idx = seqrpc.strata_getLatestCheckpointIndex()
        assert ckp_idx is not None

        ckp = seqrpc.strata_getCheckpointInfo(ckp_idx)
        assert ckp is not None
