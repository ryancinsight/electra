# Consumer integration previews

The two patches replace existing production-path code with calls to Electra. They were tested in isolated checkouts of the commits below using a local override for the unpublished `electra-network 0.1.0` package. They are candidates for the downstream repositories. Neither consumer has been switched on its default branch.

| Consumer and baseline | Change | Local evidence, macOS arm64 · Rust 1.97.0 |
| --- | --- | --- |
| [CFDrs 40021147](https://github.com/ryancinsight/CFDrs/commit/40021147e365520bd0e763f258638540f19ba1ff) | [cfdrs.patch](cfdrs.patch) delegates the four fixed/free endpoint cases to the generic branch stamp, emitting directly into CFDrs’ COO and RHS. Boundary classification, pressure/flow types and solver selection remain in CFDrs. | **523/523** `cfd-1d` library tests passed. Four matrix-assembly tests cover the hand-computed Laplacian, identity rows, symmetric active block and Neumann source. |
| [kwavers 7d6c4b89](https://github.com/ryancinsight/kwavers/commit/7d6c4b89d5acbe223391d08abba2ec144382c7bb) | [kwavers-driver.patch](kwavers-driver.patch) delegates each fixed-budget IR-drop iteration to the generic sweep over borrowed CSR runs. It preserves the existing row and neighbour summation order. | **495/495** `kwavers-driver` library tests passed. The 100-segment, 40,000-sweep fixture retained the required `f64` bits `4591621414013336774`. |

## Reproduce

Apply each patch in a clean checkout of its corresponding baseline. Set `ELECTRA_ROOT` to the absolute path of this Electra checkout. Run the applicable pair of commands in that consumer repository:

```sh
# CFDrs
git apply "$ELECTRA_ROOT/integrations/cfdrs.patch"
cargo +1.97.0 --config "patch.crates-io.electra-network.path='${ELECTRA_ROOT}/crates/electra-network'" test -p cfd-1d --lib

# kwavers
git apply "$ELECTRA_ROOT/integrations/kwavers-driver.patch"
cargo +1.97.0 --config "patch.crates-io.electra-network.path='${ELECTRA_ROOT}/crates/electra-network'" test -p kwavers-driver --lib
```

Add `--offline` after `test` when the consumers’ pre-existing dependency graphs are cached. The local override enables qualification before publication. On adoption, resolve the actual Atlas release, regenerate the consumer lockfile, rerun the complete repository gates and remove the corresponding preview from Electra once the downstream change is linked.
