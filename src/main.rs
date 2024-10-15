use std::net::Ipv6Addr;

use nix::ifaddrs::InterfaceAddress;
use reqwest::Url;

fn main() -> anyhow::Result<()> {
    let zone = std::env::var("DYNV6_ZONE").map_err(|_| anyhow::anyhow!("DYNV6_ZONE env var not set"))?;
    let token = std::env::var("DYNV6_TOKEN").map_err(|_| anyhow::anyhow!("DYNV6_TOKEN env var not set"))?;
    let interface = std::env::var("DYNV6_INTERFACE").map_err(|_| anyhow::anyhow!("DYNV6_INTERFACE env var not set"))?;

    let addr = interface_ipv6(&interface)?;

    println!("identified ipv6 for interface \"{}\": {}", &interface, addr);

    update_ipv6(&addr, &zone, &token)?;

    Ok(())
}

fn interface_ipv6(ifname: &str) -> anyhow::Result<Ipv6Addr> {
    let addrs = nix::ifaddrs::getifaddrs().unwrap();
    let addr1 = addrs.filter(|ifaddr| ifaddr.interface_name == ifname).collect::<Vec<_>>();
    if addr1.is_empty() {
        return Err(anyhow::anyhow!("interface \"{}\" not found", ifname));
    }

    let addr = addr1.into_iter()
        .filter_map(|ifaddr| ipv6_from_interface_addr(&ifaddr))
        .find(|ip| !is_link_local(ip));

    addr.ok_or_else(|| anyhow::anyhow!("unable to find a suitable global ipv6 for interface \"{}\"", ifname))
}

fn is_link_local(ip: &Ipv6Addr) -> bool {
    (ip.segments()[0] & 0xffc0) == 0xfe80
}

fn ipv6_from_interface_addr(ifaddr: &InterfaceAddress) -> Option<Ipv6Addr> {
    match ifaddr.address {
        Some(address) => {
            let sock_addr6 = address.as_sockaddr_in6();
            sock_addr6.map(|sock_addr6| sock_addr6.ip())
        }
        None => None,
    }
}

fn update_ipv6(ip: &Ipv6Addr, zone: &str, token: &str) -> anyhow::Result<()> {
    let mut url = Url::parse("https://dynv6.com/api/update")?;
    let params = [
        ("zone", zone),
        ("token", token),
        ("ipv6", &ip.to_string()),
    ];

    url.query_pairs_mut().extend_pairs(params);

    println!("sending update request: {}", url);

    let response = reqwest::blocking::get(url)?;

    if !response.status().is_success() {
        return Err(anyhow::anyhow!("update request failed with status: {}", response.status()));
    }

    println!("update request successful");

    Ok(())
}
