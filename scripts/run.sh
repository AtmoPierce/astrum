#!/usr/bin/env bash
set -euo pipefail

JOBNAME=${1:-astrum}
EMAIL=${EMAIL:-mga0002@uah.edu}
QUEUE=${QUEUE:-express}
NCPUS=${NCPUS:-1}
MEM=${MEM:-1000mb}
CHIP=${CHIP:-Milan}
WALL=${WALL:-01:00:00}

# Required start time in YYYYMMDDhhmm.ss format
START=$(date +%Y%m%d%H%M.%S)

qsub -q "$QUEUE" -j oe \
     -N "$JOBNAME" \
     -a "$START" \
     -r n \
     -M "$EMAIL" \
     -l walltime="$WALL" \
     -l select=1:ncpus=$NCPUS:mem=$MEM:cpuchip=$CHIP \
     <<'EOS'

#!/bin/bash
set -euo pipefail
echo "Node: $(hostname)"
echo "PBS_O_WORKDIR: ${PBS_O_WORKDIR}"
cd "${PBS_O_WORKDIR}"

# --- choose scratch (node-local if available) ---
if [[ -d /scratch-local && -w /scratch-local ]]; then
  WORKDIR="/scratch-local/${LOGNAME}.${PBS_JOBNAME}.${PBS_JOBID}"
  echo "Using local scratch: $WORKDIR"
else
  WORKDIR="${TMPDIR:-/tmp}/${LOGNAME}.${PBS_JOBNAME}.${PBS_JOBID}"
  echo "Using tmp fallback: $WORKDIR"
fi
umask 077
mkdir -p "$WORKDIR"
ls -ld "$WORKDIR" || true

# Make results go to scratch
export TMPDIR="$WORKDIR"
export CSV_DIR="$WORKDIR"

# --- always stage-out on exit, even if the job fails ---
OUTDIR="${PBS_O_WORKDIR}/output/${PBS_JOBNAME}.o${PBS_JOBID}"
cleanup() {
  rc=$?
  echo "Staging out -> $OUTDIR"
  mkdir -p "$OUTDIR"
  # rsync if present, otherwise cp -a
  if command -v rsync >/dev/null 2>&1; then
    rsync -a "$WORKDIR/." "$OUTDIR/"
  else
    cp -a "$WORKDIR/." "$OUTDIR/" || true
  fi
  echo "Final contents:"
  ls -lh "$OUTDIR" || true
  # (optional) leave WORKDIR; remove if you prefer cleanup:
  # rm -rf "$WORKDIR"
  exit $rc
}
trap cleanup EXIT

# Confirm vendor
grep -m1 'vendor_id' /proc/cpuinfo

# Show which powercap is available
ls -1 /sys/class/powercap

# Read energy once (gives microjoules)
cat /sys/class/powercap/intel-rapl:0/energy_uj

# --- run your program 
echo "Running job $PBS_JOBNAME"
./target/release/astrum
EOS