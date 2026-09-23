# Breaking the Sorting Barrier - Validation of a New Shortest-Path Algorithm

English | [日本語](./README-ja.md)

This repository validates the shortest-path algorithm proposed in "Breaking the Sorting Barrier for Directed Single-Source Shortest Paths" ([arXiv:2504.17033](https://arxiv.org/abs/2504.17033)).

> **2026-09 replication ([docs/chd-replication.md](./docs/chd-replication.md), in Japanese)**
> vals.ai announced C-HD as a shortest-path algorithm "faster than Dijkstra"
> ([blog post](https://www.vals.ai/blogs/faster-shortest-path-algorithm),
> [spicylemonade/c-hd-proof](https://github.com/spicylemonade/c-hd-proof)).
> We implemented it in Rust from its paper, re-implemented the BMSSP algorithm of arXiv 2504.17033 faithfully,
> and compared both against Dijkstra.
> Both are slower than Dijkstra at practical sizes:
> - C-HD is 1.5–7.3× slower including preprocessing, and performs 2.2–4× more comparisons.
> - DMM+25 is 14–121× slower.
>
> The replication also showed that the speedup claims for the earlier implementations
> (in the "Results", "Discussion" and "Conclusion" sections below) do not hold:
> - V1 is identical to Dijkstra.
> - V2 never reaches its Bellman–Ford phase.
> - The "core algorithm" falls through to a full Dijkstra on its top-level call.
>
> See §6 of the report for details.

## About the paper

The paper received the Best Paper Award at STOC 2025 (ACM Symposium on Theory of Computing). It breaks the complexity barrier that Dijkstra's algorithm had held for 40 years.

- **Authors**: Ran Duan, Jiayi Mao, Xiao Mao, Xinkai Shu, Longhui Yin
- **Claim**: single-source shortest paths in O(m log^(2/3) n) time
- **Previous bound**: Dijkstra's algorithm, O(m log n)

## Implementations

### Algorithms

1. **Dijkstra** (`src/dijkstra.rs`)
   - Standard implementation (baseline)
2. **Improved V1** (`src/improved_sssp.rs`)
   - Originally described as a simple optimization. In fact it is identical to Dijkstra.
3. **Improved V2** (`src/improved_sssp_v2.rs`)
   - Originally described as a Dijkstra / Bellman–Ford hybrid with adaptive frontier management.
   - In fact the Bellman–Ford phase is unreachable, so it is Dijkstra.
4. **Core algorithm** (`src/core_algorithm.rs`)
   - Originally described as an exact implementation of the paper (BMSSP recursion, FindPivots, partial-sorting structure).
   - In fact the top-level call goes straight to a full Dijkstra.
5. **Faithful DMM+25** (`src/dmm25.rs`), added 2026-09
   - The paper's Algorithms 1–3, the Lemma 3.3 block data structure and the constant-degree transform
6. **C-HD** (`src/chd.rs`), added 2026-09
   - §2–§6 of the vals.ai / c-hd-proof paper: preprocessing, FindPivots-HD, pointer scans, parameters
7. **Dijkstra baselines** (`src/dijkstra.rs`: `dijkstra_csr`, `dijkstra_dary`), added 2026-09

### Validation tools

- `src/bin/compare.rs`: benchmark with exact per-vertex checks against Dijkstra, including unreachable vertices
- `examples/opcount.rs`: comparison counts, the cost measure of the comparison-addition model (`--features count-cmp`)
- `tests/bmssp_family_test.rs`: exactness tests for DMM+25 and C-HD
- `src/main.rs`: original performance comparison. Its correctness check ignores vertices at +∞.
- `src/analysis.rs`: original complexity analysis
- `tests/core_algorithm_test.rs`: unit tests for the core algorithm (8 tests)
- `benches/shortest_path_bench.rs`: Criterion benchmarks

## How to run

```bash
# 2026-09 replication: correctness tests and comparison benchmarks
cargo test --release --test bmssp_family_test
cargo run --release --bin compare -- --lg 14,16,18,20 --t0 16,4
cargo run --release --features count-cmp --example opcount

# Original performance comparison (4 implementations)
cargo run --release --bin shortest-path-validation

# Original detailed analysis (sparse/medium/dense graphs)
cargo run --release --bin analysis

# Unit tests for the core algorithm
cargo test --test core_algorithm_test

# Criterion benchmarks
cargo bench
```

## Results (2026-09 replication)

These are execution times relative to the fastest Dijkstra (a 4-ary heap). Values above 1 mean slower than Dijkstra.
Every run matched Dijkstra exactly on every vertex.

| Graphs | C-HD (incl. preprocessing) | C-HD core only | DMM+25 |
|---|---|---|---|
| Random sparse, n = 2^14–2^22 | 1.47–7.3× | 1.12–3.9× | 14–121× |
| Grid, n = 2^16–2^22 | 4.5–7.3× | 3.2–5.1× | 38–50× |

C-HD also performs 2.2–4.0× more label/weight comparisons than Dijkstra.
Across the measured range, n = 2^14–2^22, there is no sign of a crossover.
The theoretical gain of lg^{1/12} n is at most 1.29 in this range.
The full tables and the analysis are in [docs/chd-replication.md](./docs/chd-replication.md).
Raw data is in `results/`.

## Original results (2025)

> ⚠️ These are the original 2025 results. They are kept for the record.
> Each benchmark was timed once, and the correctness check missed vertices left at +∞.
> All three compared implementations are effectively Dijkstra, so the reported "speedups" are measurement noise
> (see §6 of [docs/chd-replication.md](./docs/chd-replication.md)).

### Benchmark (4 implementations)

```
Nodes     Edges      Dijkstra(ms)  V1(ms)      V2(ms)      Core(ms)   Best speedup
1000      50,193     0.247         0.232       0.237       0.705      1.06x
2000      200,205    0.962         0.676       0.689       3.089      1.42x
5000      1,250,309  5.130         4.316       4.540       19.523     1.19x
```

### Claimed characteristics (not valid; see above)

| Implementation | Speed | Theoretical fidelity | Complexity |
|------|------|------------|------------|
| **Dijkstra** | baseline | ✅ complete | simple |
| **V1** | 1.1–1.2x | ⚠️ simplified | simple |
| **V2** | 1.1–1.6x | ✅ good | moderate |
| **Core algorithm** | 0.3–0.5x | "faithful to the paper" | complex |

The original README also claimed the following. None of it holds, because every implementation compared is Dijkstra.
- V2 speedups of up to 1.29× on sparse graphs, 1.28× at medium density and 1.17× on dense graphs.
- The O(m log^(2/3) n) complexity was demonstrated, because a normalized time ratio converged to about 0.5.

The 8 unit tests of the core algorithm pass, but they never exercise the BMSSP recursion on a real run.

## Conclusion (revised 2026-09)

- A faithful implementation of arXiv 2504.17033 is correct but 14–121× slower than Dijkstra at n ≤ 2^22.
  Its advantage is purely asymptotic.
- C-HD's "faster than Dijkstra" is a formal asymptotic upper bound.
  The gain is lg^{1/12} n, it applies only in a narrow density window, and the constants are astronomically large.
  Its authors do not claim a practical speedup.
  In our measurements it is slower than Dijkstra in every case, both in time and in comparison count.

The original 2025 analysis is in [REPORT.md](./REPORT.md). Its conclusions are superseded by the replication report.

## License

MIT

## References

- [Breaking the Sorting Barrier for Directed Single-Source Shortest Paths](https://arxiv.org/abs/2504.17033)
- [STOC 2025 Best Paper Award](https://www.mpi-inf.mpg.de/news/detail/stoc-best-paper-award-how-to-find-the-shortest-path-faster)
- [A Faster Shortest Path Algorithm (vals.ai)](https://www.vals.ai/blogs/faster-shortest-path-algorithm)
- [spicylemonade/c-hd-proof](https://github.com/spicylemonade/c-hd-proof)
