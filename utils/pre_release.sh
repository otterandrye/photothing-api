#!/bin/sh

echo "env inside release script"
env

# cargo might not be on the path on the dyno, try and add it
if ! [[ $(type -P cargo) ]]; then
  echo "Trying to find Cargo. Path before: $PATH"
  PATH="$CACHE_DIR/cargo/bin:$PATH"
  echo "Path after: $PATH"
  echo "Cargo: $(which cargo)"
fi

cargo install diesel_cli --no-default-features --features postgres
diesel migration run
