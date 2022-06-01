#!/bin/bash

PROJECT_NAME=""
JOB_NAME=""
ARGS=""

help() {
  echo $'Usage:
  This script is used to execute Makefile of the sub project
  build.sh [PRJECT_NAME] [JOB_NAME] [ARGS]

PROJECT_NAME: Project name is required, value is "pisa-proxy" or "pisa-controller"
JOB_NAME: Job name is required, the target of Makefile of PROJECT_NAME
ARGS: Args of make
  '
}

idx=1
for arg in "$@"; do
  case ${idx} in
    1)
      PROJECT_NAME=${arg}
      ;;
    2) 
      JOB_NAME=${arg}
      ;;
    *)
      ARGS="${ARGS} ${arg}"
  esac
  (( idx += 1 ))
done

if [ -z "${PROJECT_NAME}" ]; then
  echo 'PRJECT_NAME is required, PROJECT_NAME is first arg'
  help
  exit 1
fi

if [ -z "${JOB_NAME}" ]; then
  echo 'JOB_NAME is required, JOB_NAME is second arg'
  help
  exit 1
fi

make -C ${PROJECT_NAME} ${JOB_NAME} ${ARGS}
