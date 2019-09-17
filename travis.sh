#!/bin/bash

# use -j2 as travis VM's have 2 cores https://docs.travis-ci.com/user/reference/overview/

set -ev

# prevent timeouts
# this is not ideal... but we have looooong compile times.
# and travis_wait doesnt work in bash
(
    while :
    do
        sleep 5m
        echo "☃"
    done
) &

# test and build
rustup update
cargo test --release -v --all -j 2
cd pf_sandbox
cargo build --release --no-default-features
cd ..
cargo build --release --all -j 2

# commented out as website is not running for now
#if [ "${TRAVIS_PULL_REQUEST}" = "false" ]; then
#    # package
#    mkdir pf
#    mv target/release/pf_sandbox pf/
#    mv target/release/pf_tas pf/
#    mv target/release/pf_cli pf/
#    mv target/release/pf_map_controllers pf/
#    tar -cvzf pfsandbox-${TRAVIS_COMMIT:0:15}-linux.tar.gz pf
#
#    # upload
#    echo "put pfsandbox-${TRAVIS_COMMIT:0:15}-linux.tar.gz /home/rubic/PF_Sandbox_Website/builds/" | sftp rubic@pfsandbox.net
#fi
