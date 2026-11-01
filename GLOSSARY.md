# Glossary

## Click Run
開始操作から停止までの、1 回の連続したクリック実行。手動停止、または指定回数の完了で終わる。

## Click Target
クリックする 1 地点。位置（現在のカーソル位置、または固定座標）、ボタン（左・右・中）、クリックの種類（単発・ダブル・長押し）を持つ。

## Hold Duration
長押しの Click Target がボタンを押し続ける時間。単位（ミリ秒・秒・分）と数値で指定する。Interval とは別の設定で、単発・ダブルクリックには持たない。

## Sequence
Step を順に並べたもの。Click Run は 1 つの Click Target、または 1 つの Sequence を繰り返す。

## Step
Sequence の 1 要素。Click Target と、クリック後の Interval を持つ。

## Interval
クリック（長押しなら離した後）から次のクリックまでの待ち時間。単位（ミリ秒・秒・分）と数値で指定する。最小は 1 ms。

## Round
Sequence の全 Step を 1 周すること。「N 回で終了」の N は Round の数。単独の Click Target では 1 クリック = 1 Round。

## Capture
その時点のカーソル位置を、新しい Step として Sequence に追加する操作。

## Preset
名前を付けて保存した Sequence。
