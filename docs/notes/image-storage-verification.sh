#!/usr/bin/env bash
# Post-fix verification. Uses the new content-addressed layout:
#   images/{hash[..2]}/{hash}.{ext}
# and the new DB column `image_extension` with `content` = bare hash.
set -e
cd /tmp/ditox-bughunt

SCRATCH=$(mktemp -d)
export XDG_DATA_HOME=$SCRATCH
export XDG_CONFIG_HOME=$SCRATCH/config
mkdir -p "$XDG_CONFIG_HOME"
BIN=/home/friend/dev/personal/ditox/target/release/ditox
DB=$XDG_DATA_HOME/ditox/ditox.db
IMG=$XDG_DATA_HOME/ditox/images

cleanup() {
    pkill -f "release/ditox" 2>/dev/null || true
    rm -rf "$SCRATCH"
}
trap cleanup EXIT

count_files() {
    # Count non-quarantine, non-tmp files under IMG
    find "$IMG" -type f ! -path "*/.quarantine/*" ! -name "*.tmp" 2>/dev/null | wc -l
}

assert_eq() {
    # $1=label $2=actual $3=expected
    if [ "$2" = "$3" ]; then
        echo "  OK  $1: $2"
    else
        echo "  FAIL $1: got $2, expected $3"
        EXIT_CODE=1
    fi
}

EXIT_CODE=0

echo "=== STAGE A: Bug #1 (write-before-dedup) ==="
"$BIN" watch >/tmp/ditox-bughunt/watch-fixed-a.log 2>&1 &
WPID=$!
sleep 0.6

wl-copy --type image/png < px0.png
sleep 0.7
wl-copy "text1"
sleep 0.7
wl-copy --type image/png < px0.png
sleep 0.7
wl-copy "text2"
sleep 0.7
wl-copy --type image/png < px0.png
sleep 0.7

kill $WPID 2>/dev/null || true
wait $WPID 2>/dev/null || true

img_rows=$(sqlite3 "$DB" "SELECT COUNT(*) FROM entries WHERE entry_type='image';")
files=$(count_files)
echo "image rows: $img_rows, files: $files"
assert_eq "one row per content-hash" "$img_rows" "1"
assert_eq "rows == files (no orphans)" "$img_rows" "$files"

echo ""
echo "=== STAGE B: delete removes blob ==="
ID=$(sqlite3 "$DB" "SELECT id FROM entries WHERE entry_type='image' LIMIT 1;")
"$BIN" delete "$ID" >/dev/null
img_rows=$(sqlite3 "$DB" "SELECT COUNT(*) FROM entries WHERE entry_type='image';")
files=$(count_files)
assert_eq "rows==0 after delete" "$img_rows" "0"
assert_eq "files==0 after delete" "$files" "0"

echo ""
echo "=== STAGE C: cleanup_old prunes blobs (max=3) ==="
mkdir -p "$XDG_CONFIG_HOME/ditox"
cat > "$XDG_CONFIG_HOME/ditox/config.toml" <<EOF
[general]
max_entries = 3
poll_interval_ms = 300
EOF
sqlite3 "$DB" "DELETE FROM entries;"
rm -rf "$IMG"
mkdir -p "$IMG"

"$BIN" watch >/tmp/ditox-bughunt/watch-fixed-c.log 2>&1 &
WPID=$!
sleep 0.6
for img in px0 px1 px2 px3 px4 px5; do
  wl-copy --type image/png < $img.png
  sleep 0.7
  wl-copy "filler-$img"
  sleep 0.4
done
kill $WPID 2>/dev/null || true
wait $WPID 2>/dev/null || true

img_rows=$(sqlite3 "$DB" "SELECT COUNT(*) FROM entries WHERE entry_type='image';")
files=$(count_files)
echo "image rows: $img_rows, files: $files"
# Note: cleanup_old caps TOTAL entries, not just images. Text filler counts too.
# So image rows may be <=3 depending on ordering. Key invariant is rows==files.
assert_eq "rows == files (no image leaks after eviction)" "$img_rows" "$files"

echo ""
echo "=== STAGE D: clear_all wipes blobs ==="
"$BIN" clear --confirm >/dev/null
img_rows=$(sqlite3 "$DB" "SELECT COUNT(*) FROM entries WHERE entry_type='image';")
files=$(count_files)
assert_eq "rows==0 after clear" "$img_rows" "0"
assert_eq "files==0 after clear" "$files" "0"

echo ""
echo "=== STAGE E: watcher restart idempotent ==="
# Scenario: user copies image, watcher captures it, watcher restarts while
# image is still on clipboard. Expected: no duplicate capture.
#
# Start a watcher, copy the image WHILE it's running (so it captures), then
# restart the watcher with the same image still on the clipboard and assert
# no new rows/files appear.
"$BIN" watch >/tmp/ditox-bughunt/watch-fixed-e1.log 2>&1 &
WPID=$!
sleep 0.6
wl-copy --type image/png < px1.png
sleep 1.2
rows1=$(sqlite3 "$DB" "SELECT COUNT(*) FROM entries WHERE entry_type='image';")
files1=$(count_files)
kill $WPID; wait $WPID 2>/dev/null || true

# Image still on clipboard. Restart.
"$BIN" watch >/tmp/ditox-bughunt/watch-fixed-e2.log 2>&1 &
WPID=$!
sleep 1.5
rows2=$(sqlite3 "$DB" "SELECT COUNT(*) FROM entries WHERE entry_type='image';")
files2=$(count_files)
kill $WPID; wait $WPID 2>/dev/null || true

echo "after first watcher:  rows=$rows1 files=$files1"
echo "after restart:        rows=$rows2 files=$files2"
assert_eq "first watcher captured the image" "$rows1" "1"
assert_eq "restart doesn't add rows" "$rows2" "$rows1"
assert_eq "restart doesn't add files" "$files2" "$files1"

echo ""
echo "=== STAGE F: hash integrity of referenced blobs ==="
rows_raw=$(sqlite3 "$DB" "SELECT hash || '|' || COALESCE(image_extension,'png') FROM entries WHERE entry_type='image'")
if [ -z "$rows_raw" ]; then
  echo "(no image rows — stage E reset left nothing)"
else
  while IFS='|' read hash ext; do
      path="$IMG/${hash:0:2}/${hash}.${ext}"
      actual=$(sha256sum "$path" 2>/dev/null | awk '{print $1}')
      if [ -z "$actual" ]; then
          echo "  FAIL dangling: $path missing"
          EXIT_CODE=1
      elif [ "$actual" != "$hash" ]; then
          echo "  FAIL mismatch: db=$hash actual=$actual"
          EXIT_CODE=1
      else
          echo "  OK    $hash"
      fi
  done <<< "$rows_raw"
fi

echo ""
echo "=== STAGE G: ditox repair (dry-run) ==="
# Inject one orphan file and one dangling row manually
mkdir -p "$IMG/ab"
echo "fake" > "$IMG/ab/abababababababababababababababababababababababababababababababab.png"
sqlite3 "$DB" "INSERT INTO entries (id, entry_type, content, hash, byte_size, created_at, last_used, pinned, image_extension) VALUES ('dangle-1', 'image', 'deaddeaddeaddeaddeaddeaddeaddeaddeaddeaddeaddeaddeaddeaddeaddead', 'deaddeaddeaddeaddeaddeaddeaddeaddeaddeaddeaddeaddeaddeaddeaddead', 1, datetime('now'), datetime('now'), 0, 'png');"

"$BIN" repair --dry-run
img_rows=$(sqlite3 "$DB" "SELECT COUNT(*) FROM entries WHERE entry_type='image';")
files=$(count_files)
echo "post-dry-run: rows=$img_rows files=$files (should be unchanged)"
# Dry run should change nothing
dangle_still_there=$(sqlite3 "$DB" "SELECT COUNT(*) FROM entries WHERE id='dangle-1';")
orphan_still_there=$([ -f "$IMG/ab/abababababababababababababababababababababababababababababababab.png" ] && echo 1 || echo 0)
assert_eq "dry-run: dangling row still present" "$dangle_still_there" "1"
assert_eq "dry-run: orphan file still present" "$orphan_still_there" "1"

echo ""
echo "=== STAGE H: ditox repair (apply) ==="
"$BIN" repair
dangle_gone=$(sqlite3 "$DB" "SELECT COUNT(*) FROM entries WHERE id='dangle-1';")
orphan_gone=$([ ! -f "$IMG/ab/abababababababababababababababababababababababababababababababab.png" ] && echo 1 || echo 0)
assert_eq "repair: dangling row removed" "$dangle_gone" "0"
assert_eq "repair: orphan file removed" "$orphan_gone" "1"

echo ""
if [ $EXIT_CODE -eq 0 ]; then
    echo "ALL STAGES PASSED"
else
    echo "SOME STAGES FAILED (see above)"
fi
exit $EXIT_CODE
