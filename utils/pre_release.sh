#!/bin/sh

# cargo might not be on the path on the dyno, try and add it
[[ !$(type -P cargo) ]] || PATH="$CACHE_DIR/cargo/bin:$PATH"

cargo install diesel_cli --no-default-features --features postgres
diesel migration run
