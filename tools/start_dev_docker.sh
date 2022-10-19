#!/bin/bash
set -u
export BOLD='\033[1m'
export RED='\033[0;31m'
export BLUE='\033[0;34m'
export GREEN='\033[32m'
export WHITE='\033[34m'
export YELLOW='\033[33m'
export NO_COLOR='\033[0m'

DEV_CONTAINER="sk_dev"
DOCKER_IMG="skdev"

ok() {
  (echo >&2 -e "[${GREEN}${BOLD} OK ${NO_COLOR}] $*")
}

main() {
  docker run  -idt --privileged --name=sk_dev -v `pwd`:/skwork skdev

  ok "Successfully started Docker container [${DEV_CONTAINER}] based on image: [${DOCKER_IMG}]"

  ok "-_-!"
}

main "$@"
