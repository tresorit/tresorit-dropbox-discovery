set RUSTFLAGS=-C target-feature=+crt-static
cargo +stable build --release --target i686-pc-windows-msvc
copy /y target\i686-pc-windows-msvc\release\tresorit-dropbox-discovery.exe tresorit-dropbox-discovery-win.exe

echo "The compiled exe does not have resources (company info, version, icon) and signature yet!"
