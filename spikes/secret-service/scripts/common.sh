# Shared helpers for the spike scenarios. Sourced, not executed.
# run <cmd...> prints the exact command, stdout, stderr, exit code and duration (ms).
step() { echo; echo "### $*"; }
run() {
  local s e rc out err_file
  err_file="$(mktemp)"
  s=$(date +%s%N)
  out=$(timeout 15 "$@" 2>"$err_file"); rc=$?
  e=$(date +%s%N)
  echo "\$ $*"
  echo "stdout: $out"
  echo "stderr: $(cat "$err_file")"
  echo "exit=$rc duration_ms=$(( (e - s) / 1000000 ))"
  rm -f "$err_file"
}
