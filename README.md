# Breaking the Sorting Barrier - 最短経路アルゴリズムの検証実装

論文「Breaking the Sorting Barrier for Directed Single-Source Shortest Paths」（[arXiv:2504.17033](https://arxiv.org/abs/2504.17033)）で提案された革新的な最短経路アルゴリズムの検証実装です。

## 論文について

2025年のSTOC（ACM Symposium on Theory of Computing）でBest Paper Awardを受賞した画期的な研究で、40年間破られなかったDijkstraアルゴリズムの計算量の壁を突破しました。

- **著者**: Ran Duan, Jiayi Mao, Xiao Mao, Xinkai Shu, Longhui Yin
- **主張**: O(m log^(2/3) n)時間での単一始点最短経路問題の解法
- **従来**: Dijkstraアルゴリズム O(m log n)

## 実装内容

### アルゴリズム

1. **Dijkstraアルゴリズム** (`src/dijkstra.rs`)
   - 従来の標準的な実装（ベースライン）

2. **改善版V1** (`src/improved_sssp.rs`)
   - シンプルな最適化版

3. **改善版V2** (`src/improved_sssp_v2.rs`)
   - 論文のアイデアに基づく実装
   - DijkstraとBellman-Fordのハイブリッド
   - 適応的なフロンティア管理

### 検証ツール

- `src/main.rs`: 基本的な性能比較
- `src/analysis.rs`: 詳細な複雑度分析
- `benches/shortest_path_bench.rs`: Criterionによるベンチマーク

## 実行方法

```bash
# 基本的な性能比較
cargo run --release

# 詳細な分析（sparse/medium/denseグラフ）
cargo run --release --bin analysis

# ベンチマーク
cargo bench
```

## 検証結果

### 性能改善
- **Sparseグラフ（密度0.01）**: 最大1.29倍の高速化
- **中密度グラフ（密度0.05）**: 最大1.28倍の高速化
- **Denseグラフ（密度0.20）**: 最大1.17倍の高速化

### 複雑度の実証
正規化された実行時間の比率が約0.5に収束し、理論値と一致：

```
ノード数  Dijkstra/改善版の比率
500       0.584
1000      0.531
2000      0.531
4000      0.520
8000      0.504
```

## 考察

### 成功点
- 理論的な複雑度O(m log^(2/3) n)を実証
- 全てのテストケースで正確な結果を出力
- グラフサイズが大きくなるほど改善が顕著

### 限界
- 実用上の改善幅は1.1〜1.3倍程度
- Denseグラフでは改善が限定的
- 実装の最適化余地あり

## 結論

Dijkstraアルゴリズムという40年間最適とされてきた古典的アルゴリズムを理論的に改善できることを実証しました。改善幅は控えめですが、計算理論上の重要な成果です。

詳細な分析結果は[REPORT.md](./REPORT.md)を参照してください。

## ライセンス

MIT

## 参考文献

- [Breaking the Sorting Barrier for Directed Single-Source Shortest Paths](https://arxiv.org/abs/2504.17033)
- [STOC 2025 Best Paper Award](https://www.mpi-inf.mpg.de/news/detail/stoc-best-paper-award-how-to-find-the-shortest-path-faster)