#!/usr/bin/env bash
# qme-watch.sh — report PBS job status every N seconds until none remain
# Usage: qme-watch.sh [-i SECONDS] [-v]
#  -i, --interval  polling interval (default 1s)
#  -v, --verbose   also print qstat/qme table each tick

set -euo pipefail

INTERVAL=10
VERBOSE=0
while [[ $# -gt 0 ]]; do
  case "$1" in
    (-i|--interval) INTERVAL="${2:-1}"; shift 2;;
    (-v|--verbose)  VERBOSE=1; shift;;
    (-h|--help)
      echo "Usage: $0 [-i SECONDS] [-v]"; exit 0;;
    (*) echo "Unknown option: $1"; exit 2;;
  esac
done

have() { command -v "$1" >/dev/null 2>&1; }

# Choose how to list active jobs
if have qselect && have qstat; then
  LIST_IDS='qselect -u "$USER"'
  SUMMARIZE='qstat -fx'
  TABLE='qstat -u "$USER" -t'
else
  echo "Error: need qselect+qstat or qme in PATH." >&2
  exit 1
fi

trap 'echo; echo "Interrupted."; exit 130' INT

echo "Watching jobs for user: $USER  (interval: ${INTERVAL}s)"
while :; do
  ts="$(date +%H:%M:%S)"

  if [[ -n "$LIST_IDS" ]]; then
    # Robust path: qselect tells us if *any* jobs are still present
    mapfile -t IDS < <(bash -c "$LIST_IDS" 2>/dev/null || true)
    if (( ${#IDS[@]} == 0 )); then
      echo "[$ts] All jobs done (none remain)."
      exit 0
    fi

    # Summarize by PBS job_state
    # Q=queued, R=running, H=hold, E=exiting, B=begun (array), S=suspended, W=waiting, T=transit, F=finished
    summary="$($SUMMARIZE "${IDS[@]}" 2>/dev/null \
      | awk '
          /job_state =/ {c[$3]++}
          END {
            total=0; for (k in c) total+=c[k];
            printf "jobs=%d", total;
            # Print common states in a stable order, then any others
            order[1]="R"; order[2]="Q"; order[3]="B"; order[4]="H";
            order[5]="S"; order[6]="E"; order[7]="W"; order[8]="T"; order[9]="F";
            for (i=1;i<=9;i++) if (order[i] in c) printf " %s=%d", order[i], c[order[i]];
            for (k in c) { unk=1;
              for (i=1;i<=9;i++) if (k==order[i]) {unk=0;break}
              if (unk) printf " %s=%d", k, c[k];
            }
            print ""
          }'
      )"
    echo "[$ts] $summary"

    if (( VERBOSE )); then
      echo "----"
      bash -c "$TABLE" || true
      echo "----"
    fi
  else
    # qme-only fallback: treat any output lines as "jobs exist"
    out="$(bash -c "$TABLE" 2>/dev/null || true)"
    # Count non-header lines heuristically
    active_lines="$(printf "%s\n" "$out" | awk 'NR>1 && NF>0 {n++} END{print n+0}')"
    if (( active_lines == 0 )); then
      echo "[$ts] All jobs done (none remain)."
      exit 0
    fi
    echo "[$ts] jobs~$active_lines (via qme)"
    if (( VERBOSE )); then
      echo "----"; printf "%s\n" "$out"; echo "----"
    fi
  fi

  sleep "$INTERVAL"
done
