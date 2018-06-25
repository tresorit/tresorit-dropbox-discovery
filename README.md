# Dropbox Discovery Tool by Tresorit
 
It helps you discovering running Dropbox instances on your local network.
When the discovery process starts, it will open UDP port 17500 to watch
incoming Dropbox LAN Sync packets from within the local wired or wireless
network your computer is currently connected to.  
In case your local network contains of multiple IP subnets (eg. a separate
wired and wireless network, or multiple Wi-Fi networks), you'll need to
re-run the tool to repeat the discovery for each network to get a complete
result.

## Build instructions


You can download the latest prebuilt binaries for Windows, macOS and Linux from the Releases page.

You can also build it manually, by installing Rust 1.26.0 or newer and then running the following commands:
```sh
git clone https://github.com/tresorit/tresorit-dropbox-discovery
cd tresorit-dropbox-discovery
cargo build
```

Running `cargo run` will start the compiled binary.

We have also provided helper scripts that we use to compile the tool for official releases (`build-win.cmd`, `build-macos.sh`, `build-linux.sh`).
