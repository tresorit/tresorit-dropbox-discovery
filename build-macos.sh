#!/bin/sh
set -e

cargo +stable build --release --target x86_64-apple-darwin
cp -f target/x86_64-apple-darwin/release/tresorit-dropbox-discovery tresorit-dropbox-discovery-macos
strip tresorit-dropbox-discovery-macos
sha512sum tresorit-dropbox-discovery-macos > tresorit-dropbox-discovery-macos.sha512
sha512sum -c tresorit-dropbox-discovery-macos.sha512 >/dev/null
