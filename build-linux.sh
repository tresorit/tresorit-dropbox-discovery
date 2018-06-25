#!/bin/sh
set -e

cargo +stable build --release --target i686-unknown-linux-gnu
cp -f target/i686-unknown-linux-gnu/release/tresorit-dropbox-discovery tresorit-dropbox-discovery-linux-x86
strip tresorit-dropbox-discovery-linux-x86
sha512sum tresorit-dropbox-discovery-linux-x86 > tresorit-dropbox-discovery-linux-x86.sha512
sha512sum -c tresorit-dropbox-discovery-linux-x86.sha512 >/dev/null

cargo +stable build --release --target x86_64-unknown-linux-gnu
cp -f target/x86_64-unknown-linux-gnu/release/tresorit-dropbox-discovery tresorit-dropbox-discovery-linux-x64
strip tresorit-dropbox-discovery-linux-x64
sha512sum tresorit-dropbox-discovery-linux-x64 > tresorit-dropbox-discovery-linux-x64.sha512
sha512sum -c tresorit-dropbox-discovery-linux-x64.sha512 >/dev/null
