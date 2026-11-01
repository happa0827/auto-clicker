# 単独モードを 1 Step の Sequence として統一する

## Status
Accepted (2026-11-01)

## Context
Click Run には、現在位置の連打、固定座標のクリック、複数地点の Sequence の 3 モードがある。別々に実装すると、実行ループ、Interval、Round の数え方、UI が三重になる。

## Decision
現在位置と固定座標のモードは、Step が 1 つだけの Sequence として内部で扱う。実行ループは 1 つだけ持つ。「N 回で終了」の N は Round 数で、単独モードでは 1 クリック = 1 Round になる。UI の設定画面も共通にする。

## Alternatives
モードごとに別の実装を持つ: 単純に見えるが、停止条件や保存形式が分岐して増える。

## Consequences
Preset の保存形式は Sequence の 1 種類で済む。単独モード専用の機能を足したくなったときは、この前提と衝突する。
