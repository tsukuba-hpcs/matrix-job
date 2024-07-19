#!/bin/sh

#PBS -b {{ process_count }}
#PBS -T openmpi
#PBS -v NQSV_MPI_VER=4.1.6/gcc11.4.0-cuda11.8.0
#PBS -q gpu
#PBS -A NBB

module load openmpi/$NQSV_MPI_VER
mpirun ${NQSV_MPIOPTS} -np {{ process_count }} -npernode 1 ./n{{ process_count }}-{{ size }}{{ unit }}.sh
