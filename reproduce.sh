#!/usr/bin/env bash

# illustrate https://github.com/vv9k/docker-api-rs/issues/39

set -euo pipefail

cargo build --example image &> /dev/null

[ -f many-builds-2G/file0 ] || for i in $(seq 0 20000); do fallocate -l 100K many-builds-2G/file$i; done
[ -f build-2G/file ] || fallocate -l 2G build-2G/file

profile()
{
  title=$1; shift
  dockerfileDir=$1; shift

  echo "--- $title ---"
  echo "---> docker"
  time docker build $dockerfileDir > /dev/null

  echo ""
  echo "---> docker-api"
  time target/debug/examples/image build $dockerfileDir > /dev/null
  echo ""
}

profile "NO BUILD CONTEXT" no-build-ctx
profile "20000 FILES; 100Kb EACH" many-builds-2G
profile "1 FILE; 2Gb" build-2G
profile "THIS REPO AS BUILD CONTEXT" . 
