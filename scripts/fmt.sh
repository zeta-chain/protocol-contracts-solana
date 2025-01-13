#!/bin/bash

# Exit on any error
set -e

if ! command -v rustup &> /dev/null
then
    echo "rustup is required to run the script."
    exit 1
fi

if ! command -v rustfmt &> /dev/null
then
    echo "rustfmt could not be found, installing..."
    rustup component add rustfmt
fi

cargo fmt
if [[ $? == 0 ]] ; then
    echo "Code is formatted!"
else
    echo "An error occurred during formatting."
fi
