#!/usr/bin/env bash
#-*-mode:sh;indent-tabs-mode:nil;tab-width:2;coding:utf-8-*-
# vi: tabstop=2 shiftwidth=2 softtabstop=2 expandtab:
set -euxo pipefail
WD="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../../" && pwd)"
echo "${WD}"
ESC_WD="$(echo "$WD" | sed 's/\//\\\//g')"
DOCKER_FILE="$(readlink -f $(dirname "${BASH_SOURCE[0]}")/Dockerfile)"
DOCKER_FILE=$(echo "${DOCKER_FILE}" | sed -e "s/$ESC_WD\///g")
pushd "$WD" >/dev/null 2>&1
if [ -z ${IMAGE_NAME+x} ] || [ -z ${IMAGE_NAME+x} ]; then
  IMAGE_NAME="convco/convco"
fi
IMAGE_NAME="${IMAGE_NAME}-gitpod"
CACHE_NAME="${IMAGE_NAME}:cache"
export DOCKER_BUILDKIT=1
BUILD="docker"
if [[ $(docker buildx version 2>/dev/null) ]] \
  && ! sudo grep -sq 'docker\|lxc' /proc/1/environ ; then
  builder="$(echo "$IMAGE_NAME" | cut -d/ -f2)"
  BUILD+=" buildx build"
  BUILD+=" --cache-to type=registry,mode=max,ref=${CACHE_NAME}"
  BUILD+=" --push"
  docker buildx use "${builder}" || docker buildx create --use --name "${builder}"
else
  BUILD+=" build"
  BUILD+=" --pull"
fi
  BUILD+=" --progress=auto"
  BUILD+=" --file ${DOCKER_FILE}"
  BUILD+=" --tag ${IMAGE_NAME}:latest"
  BUILD+=" --cache-from type=registry,ref=${CACHE_NAME}"
$BUILD $WD
if [[ $(docker buildx version 2>/dev/null) ]] \
  && ! sudo grep -sq 'docker\|lxc' /proc/1/environ; then
  docker buildx use default
else
  PUSH="docker push"
  PUSH+=" ${IMAGE_NAME}:latest"
  $PUSH
fi
popd >/dev/null 2>&1
