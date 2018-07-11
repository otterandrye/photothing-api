#!/bin/sh
set -e

cargo install diesel_cli --no-default-features --features postgres
diesel migration run

