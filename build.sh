#
# Copyright 2022 SphereEx Authors
#
# Licensed under the Apache License, Version 2.0 (the "License");
# you may not use this file except in compliance with the License.
# You may obtain a copy of the License at
#
# http://www.apache.org/licenses/LICENSE-2.0
#
# Unless required by applicable law or agreed to in writing, software
# distributed under the License is distributed on an "AS IS" BASIS,
# WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
# See the License for the specific language governing permissions and
# limitations under the License.
#
#!/bin/bash

PROJECT_NAME=""
JOB_NAME=""
ARGS=""

help() {
  echo $'Usage:
  This script is used to execute Makefile of the sub project
  build.sh [PROJECT_NAME] [JOB_NAME] [ARGS]

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
  echo 'PROJECT_NAME is required, PROJECT_NAME is first arg'
  help
  exit 1
fi

if [ -z "${JOB_NAME}" ]; then
  echo 'JOB_NAME is required, JOB_NAME is second arg'
  help
  exit 1
fi

make -C ${PROJECT_NAME} ${JOB_NAME} ${ARGS}
