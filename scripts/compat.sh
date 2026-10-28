#!/usr/bin/env bash
# 実サイトの互換性の目安を測る。サイトごとに別プロセスで開き、読み込み・描画・JS・メモリ・時間を1行にまとめる。
#   使い方: scripts/compat.sh [サイト一覧ファイル] [出力ディレクトリ]
#   一覧は 1 行 1 URL（# で始まる行は無視）。既定は scripts/compat-sites.txt。
# 判定の目安（自動）：
#   OK    読み込めて、描画命令が 30 件以上あり、JS の例外が 10 件以下
#   WARN  読み込めたが、描画が少ない・例外が多い・読み込みが遅いなど
#   FAIL  エラーページ・タイムアウト・クラッシュ
set -u
LIST="${1:-scripts/compat-sites.txt}"
OUT="${2:-target/compat}"
BIN="${KUMO_BIN:-target/release/kumo-viewer}"
mkdir -p "$OUT"
printf '%-6s %-44s %8s %9s %7s %6s  %s\n' "判定" "サイト" "読込ms" "Content" "描画" "JS例外" "題名"
ok=0; warn=0; fail=0
while IFS= read -r url; do
  case "$url" in ''|'#'*) continue ;; esac
  name=$(printf '%s' "$url" | sed -E 's#https?://##; s#[^A-Za-z0-9]+#_#g' | cut -c1-40)
  log="$OUT/$name.log"
  KUMO_JS_LOG=1 timeout 90 "$BIN" --headless "$OUT/$name.png" --mem-report --width 1280 --height 900 "$url" >"$log" 2>&1
  code=$?
  # 別スレッドの出力が割り込むことがあるので、行頭に頼らず「N ms で読み込み」だけを読む
  load=$(grep -oE '[0-9]+ms で読み込み' "$log" | grep -oE '^[0-9]+' | tail -1)
  content=$(grep -E '^\[total\]' "$log" | grep -oE 'content [0-9.]+MB' | grep -oE '[0-9.]+' | head -1)
  items=$(grep -E '^\[content\]' "$log" | grep -oE '描画命令 [0-9]+' | grep -oE '[0-9]+' | head -1)
  errs=$(grep -E '^\[content\]' "$log" | grep -oE '例外 [0-9]+' | grep -oE '[0-9]+' | head -1)
  title=$(grep -E '^\[tab\]' "$log" | head -1 | sed -E 's/^\[tab\] //; s/ \| .*//' | cut -c1-40)
  verdict=OK
  if [ "$code" -ne 0 ] || [ -z "$load" ] || [ "$title" = "読み込めません" ]; then
    verdict=FAIL
  elif [ "${items:-0}" -lt 30 ] || [ "${errs:-0}" -gt 10 ] || [ "${load:-0}" -gt 15000 ]; then
    verdict=WARN
  fi
  case "$verdict" in OK) ok=$((ok+1));; WARN) warn=$((warn+1));; FAIL) fail=$((fail+1));; esac
  printf '%-6s %-44s %8s %8sMB %7s %6s  %s\n' "$verdict" "$(printf '%s' "$url" | cut -c1-44)" "${load:--}" "${content:--}" "${items:--}" "${errs:--}" "$title"
done < "$LIST"
echo
echo "OK $ok / WARN $warn / FAIL $fail"
