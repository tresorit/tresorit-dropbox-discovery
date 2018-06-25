/*
   Copyright 2018 Tresorit Kft

   Licensed under the Apache License, Version 2.0 (the "License");
   you may not use this file except in compliance with the License.
   You may obtain a copy of the License at

       http://www.apache.org/licenses/LICENSE-2.0

   Unless required by applicable law or agreed to in writing, software
   distributed under the License is distributed on an "AS IS" BASIS,
   WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
   See the License for the specific language governing permissions and
   limitations under the License.
*/

#![cfg_attr(feature = "cargo-clippy", warn(clippy_pedantic))]

use std::collections::HashMap;
use std::io;
use std::net::SocketAddr;
use std::time::{Duration, Instant};

extern crate bytes;
use bytes::BytesMut;

extern crate dns_lookup;
use dns_lookup::lookup_addr;

extern crate net2;
use net2::UdpBuilder;

extern crate serde;
extern crate serde_json;
#[macro_use]
extern crate serde_derive;

extern crate tokio;
use tokio::net::{UdpFramed, UdpSocket};
use tokio::prelude::*;
use tokio::reactor::Handle;
use tokio::timer::Interval;
extern crate tokio_io;
use tokio_io::codec::Decoder;

/// Either is not implemented for Streams in futures 0.1.x, so we use our own.
#[derive(Debug)]
enum EitherStream<A, B> {
    A(A),
    B(B),
}

impl<A, B> Stream for EitherStream<A, B>
where
    A: Stream,
    B: Stream<Item = A::Item, Error = A::Error>,
{
    type Item = A::Item;
    type Error = A::Error;

    fn poll(&mut self) -> Poll<Option<A::Item>, A::Error> {
        match *self {
            EitherStream::A(ref mut a) => a.poll(),
            EitherStream::B(ref mut b) => b.poll(),
        }
    }
}

/// JSON representation of a Dropbox LAN Sync beacon packet.
#[derive(Debug, Deserialize)]
struct BeaconPacket {
    host_int: u128,
    version: Vec<usize>,
    displayname: String,
    port: u16,
    namespaces: Vec<u128>,
}

/// Simple Tokio codec to parse Dropbox LAN Sync beacon packets.
#[derive(Debug)]
struct BeaconCodec;

impl Decoder for BeaconCodec {
    // To keep decoded packets flowing, even malformed packets need to be handled, therefore we use
    // None to signal a datagram that possibly does not belong to the Dropbox LAN Sync protocol.
    type Item = Option<BeaconPacket>;
    type Error = io::Error;

    fn decode(&mut self, buf: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        if buf.is_empty() {
            // If an empty datagram were received, we signal that more data is needed.
            Ok(None)
        } else {
            // Here we assume that a single UDP datagram will be passed to decode() as a whole
            // If we cannot parse it into a valid JSON BeaconPacket, we assume that it is not a
            // half packet, but belongs to a different protocol.
            let packet = serde_json::from_slice(buf).map(Some).unwrap_or(None);
            // According to the docs, it is our responsibility to drop used bytes from the buffer
            buf.clear();
            // We never return an Err as it would close the socket, but we would like to listen
            // for more BeaconPackets even if a malformed one is received.
            Ok(Some(packet))
        }
    }
}

#[derive(Debug)]
struct HostInfo {
    id: u128,
    address: SocketAddr,
    host: String,
    namespaces: usize,
}

impl HostInfo {
    pub fn from_item(packet: &BeaconPacket, address: SocketAddr) -> Self {
        Self {
            id: packet.host_int,
            address,
            host: lookup_addr(&address.ip()).unwrap_or_else(|_| "(Unknown)".into()),
            namespaces: packet.namespaces.len(),
        }
    }
}

#[derive(Debug)]
enum Event {
    Countdown(u64),
    HostFound(HostInfo),
}

fn create_udp_stream(
    addr: &str,
    only_v6: bool,
) -> io::Result<impl Stream<Item = Event, Error = io::Error>> {
    // IPv6 socket bound to the ANY address behave differently on different OS-es.
    // On Windows it will listen on IPv6 only, but on Linux it listens on both IPv4 and IPv6,
    // unless the IPv6 only flag is set.
    let socket = match addr
        .parse()
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, e))?
    {
        SocketAddr::V4(a) => UdpBuilder::new_v4()?.bind(a),
        SocketAddr::V6(a) => UdpBuilder::new_v6()?.only_v6(only_v6)?.bind(a),
    }?;
    Ok(UdpFramed::new(
        UdpSocket::from_std(socket, &Handle::current())?,
        BeaconCodec,
    ).filter_map(|(item, addr)| {
        item.map(|packet| Event::HostFound(HostInfo::from_item(&packet, addr)))
    }))
}

fn create_countdown_stream(duration: Duration) -> impl Stream<Item = Event, Error = io::Error> {
    let start = Instant::now();
    Interval::new(start, Duration::from_secs(1))
        .map(move |now| {
            Event::Countdown(
                duration
                    .checked_sub(now.duration_since(start))
                    .map_or(0, |rem| rem.as_secs()),
            )
        })
        .map_err(|err| io::Error::new(io::ErrorKind::Other, err))
}

fn delete_current_line() {
    print!("\r{:78}\r", "");
}

fn die_if_error<T>(res: io::Result<T>) -> T {
    match res {
        Ok(t) => return t,
        Err(ref err) if err.kind() == io::ErrorKind::AddrInUse => {
            println!(
                "
Discovery error: the required network resource might already be in
use. Are you running Dropbox locally? If so, please exit Dropbox
and re-run the tool."
            );
        }
        Err(err) => {
            println!(
                "
The discovery process wasn't complete, because an error happened. Please
try again and if the problem persists, contact Tresorit.

Details:
{:?}",
                err
            );
        }
    };
    std::process::exit(1);
}

fn read_line() -> io::Result<String> {
    let mut line = String::new();
    io::stdin().read_line(&mut line)?;
    Ok(line)
}

fn print_welcome() -> io::Result<bool> {
    print!(
        "Dropbox Discovery Tool by Tresorit

It helps you discovering running Dropbox instances on your local network.
When the discovery process starts, it will open UDP port 17500 to watch
incoming Dropbox LAN Sync packets from within the local wired or wireless
network your computer is currently connected to.
In case your local network contains of multiple IP subnets (eg. a separate
wired and wireless network, or multiple Wi-Fi networks), you'll need to
re-run the tool to repeat the discovery for each network to get a complete
result.

Would you like to start the discovery process now? [Y/n] "
    );
    io::stdout().flush()?;

    let response = read_line()?.trim().to_ascii_lowercase();
    if response.is_empty() || response == "y" {
        println!();
        Ok(true)
    } else {
        Ok(false)
    }
}

fn print_progress(remaining: u64) -> io::Result<()> {
    delete_current_line();
    print!("Running discovery... ({} seconds remaining)", remaining);
    io::stdout().flush()?;
    Ok(())
}

fn print_result(hosts: &HashMap<u128, HostInfo>) {
    delete_current_line();
    if hosts.is_empty() {
        println!(
            "The Tresorit discovery tool couldn't find Dropbox running on any of the
analyzed networks."
        );
        return;
    }

    println!(
        "{} device{} running Dropbox:\n",
        hosts.len(),
        if hosts.len() > 1 { "s are" } else { " is" }
    );
    println!(
        "{:<18} {:<32} {:<15}",
        "IP address", "Computer name", "Dropbox folders"
    );
    println!("{:-<18}-{:-<32}-{:-<15}", "", "", "");
    for info in hosts.values() {
        println!(
            "{:<18} {:<32} {}",
            format!("{}", info.address.ip()),
            info.host,
            info.namespaces
        );
    }
}

fn try_main() -> io::Result<()> {
    if !print_welcome()? {
        return Ok(());
    }

    let stream = match create_udp_stream("0.0.0.0:17500", false) {
        Ok(ipv4) => match create_udp_stream("[::]:17500", true) {
            Ok(ipv6) => EitherStream::A(ipv4.select(ipv6)),
            Err(_) => EitherStream::B(ipv4),
        },
        Err(err) => match create_udp_stream("[::]:17500", false) {
            Ok(both) => EitherStream::B(both),
            Err(_) => return Err(err),
        },
    };

    tokio::run(
        stream
            .select(create_countdown_stream(Duration::from_secs(60)))
            .take_while(|event| {
                future::result(match *event {
                    Event::Countdown(remaining) => print_progress(remaining).and(Ok(remaining > 0)),
                    _ => Ok(true),
                })
            })
            .filter_map(|event| match event {
                Event::HostFound(info) => Some(info),
                _ => None,
            })
            .fold(HashMap::new(), |mut map, info| {
                future::ok::<HashMap<u128, HostInfo>, io::Error>({
                    map.entry(info.id).or_insert(info);
                    map
                })
            })
            .and_then(|map| future::ok(print_result(&map)))
            .or_else(|e| Ok(die_if_error(Err(e)))),
    );
    Ok(())
}

pub fn main() {
    die_if_error(try_main());

    println!();
    print!("Press Enter to exit.");
    io::stdout().flush().unwrap_or(());

    read_line().unwrap_or_default();
}
