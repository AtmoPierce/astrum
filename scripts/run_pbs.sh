# Any AMD Milan node with AVX2 explicitly (redundant, but shows pattern)
qsub -l select=1:ncpus=1 -l walltime=00:30:00 scripts/run_pbs.sh

# Pin to Ice Lake (Intel, AVX-512 true on your dump)
# qsub -l select=1:ncpus=32:cpuchip=Icelake:chipman=intel:avx512=True -l walltime=02:00:00 -- scripts/run_pbs.sh

# GPU class (Ampere A100-style on your dump)
# qsub -q gpuq -l select=1:ncpus=8:ngpus=1:gpuname=ampere -l walltime=01:00:00 -- scripts/run_pbs.sh