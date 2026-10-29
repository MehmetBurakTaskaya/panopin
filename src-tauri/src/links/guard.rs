//! The network guard (D-010): every request a link preview makes goes through these rules.
//!
//! - HTTPS only, no user name or password in the URL, no query parameter whose name contains
//!   token, key, secret, sig, password, auth, session or code.
//! - Only public unicast addresses: a resolver of ours drops loopback, private, link-local,
//!   shared, reserved, documentation and multicast addresses after DNS, so connections only ever
//!   go to vetted addresses (DNS rebinding included). IP literals are checked the same way.
//! - Every redirect is checked again, at most five of them; no proxy is ever used.
//! - 5 seconds and 2 MB per response.

use std::{
    error::Error,
    fmt,
    net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr},
    sync::Arc,
    time::Duration,
};

use reqwest::{
    dns::{Addrs, Name, Resolve, Resolving},
    redirect, Client, Response, Url,
};
use url::Host;

pub const MAX_BYTES: usize = 2 * 1024 * 1024;
pub const TIMEOUT: Duration = Duration::from_secs(5);
const MAX_REDIRECTS: usize = 5;

/// Query parameter names containing any of these keep a URL off the network.
const SENSITIVE: [&str; 11] = [
    "token",
    "key",
    "secret",
    "sig",
    "password",
    "passwd",
    "pwd",
    "auth",
    "session",
    "code",
    "credential",
];

/// Why a request was not made.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Refusal {
    NotHttps,
    Credentials,
    SensitiveQuery,
    PrivateAddress,
    NoHost,
    TooManyRedirects,
    TooLarge,
}

impl fmt::Display for Refusal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let reason = match self {
            Refusal::NotHttps => "only https is allowed",
            Refusal::Credentials => "the URL carries credentials",
            Refusal::SensitiveQuery => "the URL carries a sensitive parameter",
            Refusal::PrivateAddress => "the address is not public",
            Refusal::NoHost => "the URL has no host",
            Refusal::TooManyRedirects => "too many redirects",
            Refusal::TooLarge => "the response is too large",
        };
        f.write_str(reason)
    }
}

impl Error for Refusal {}

fn public_v4(ip: Ipv4Addr) -> bool {
    let [a, b, c, _] = ip.octets();
    !(a == 0                                   // "this network"
        || a == 10                             // private
        || a == 127                            // loopback
        || (a == 100 && (64..=127).contains(&b)) // shared (CGNAT)
        || (a == 169 && b == 254)              // link-local, cloud metadata
        || (a == 172 && (16..=31).contains(&b)) // private
        || (a == 192 && b == 0 && c == 0)      // IETF protocol assignments
        || (a == 192 && b == 0 && c == 2)      // documentation
        || (a == 192 && b == 88 && c == 99)    // 6to4 relay
        || (a == 192 && b == 168)              // private
        || (a == 198 && (b == 18 || b == 19))  // benchmarking
        || (a == 198 && b == 51 && c == 100)   // documentation
        || (a == 203 && b == 0 && c == 113)    // documentation
        || a >= 224) // multicast, reserved, broadcast
}

fn public_v6(ip: Ipv6Addr) -> bool {
    if let Some(v4) = ip.to_ipv4_mapped() {
        return public_v4(v4);
    }
    let s = ip.segments();
    // NAT64 carries an IPv4 address in its last 32 bits.
    if s[0] == 0x64 && s[1] == 0xff9b && s[2..6].iter().all(|&x| x == 0) {
        return public_v4(Ipv4Addr::new(
            (s[6] >> 8) as u8,
            s[6] as u8,
            (s[7] >> 8) as u8,
            s[7] as u8,
        ));
    }
    // Only global unicast (2000::/3), minus Teredo and protocol blocks (2001::/23),
    // documentation (2001:db8::/32) and 6to4 (2002::/16).
    (s[0] & 0xe000) == 0x2000
        && !(s[0] == 0x2001 && s[1] < 0x0200)
        && !(s[0] == 0x2001 && s[1] == 0x0db8)
        && s[0] != 0x2002
}

pub fn is_public(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(v4) => public_v4(v4),
        IpAddr::V6(v6) => public_v6(v6),
    }
}

/// The rules a URL must pass before any request, and again at every redirect.
pub fn check_url(url: &Url) -> Result<(), Refusal> {
    if url.scheme() != "https" {
        return Err(Refusal::NotHttps);
    }
    if !url.username().is_empty() || url.password().is_some() {
        return Err(Refusal::Credentials);
    }
    match url.host() {
        None => return Err(Refusal::NoHost),
        Some(Host::Ipv4(ip)) if !public_v4(ip) => return Err(Refusal::PrivateAddress),
        Some(Host::Ipv6(ip)) if !public_v6(ip) => return Err(Refusal::PrivateAddress),
        Some(Host::Domain(domain)) => {
            let domain = domain.trim_end_matches('.').to_ascii_lowercase();
            // Names that never leave the machine or the local network.
            if domain == "localhost"
                || domain.ends_with(".localhost")
                || domain.ends_with(".local")
                || domain.ends_with(".internal")
                || domain.ends_with(".home.arpa")
                || !domain.contains('.')
            {
                return Err(Refusal::PrivateAddress);
            }
        }
        Some(_) => {}
    }
    let sensitive = url.query_pairs().any(|(name, _)| {
        let name = name.to_ascii_lowercase();
        SENSITIVE.iter().any(|word| name.contains(word))
    });
    if sensitive {
        return Err(Refusal::SensitiveQuery);
    }
    Ok(())
}

/// Resolves with the system resolver, then keeps only public addresses.
pub async fn resolve_public(host: &str) -> Result<Vec<SocketAddr>, Box<dyn Error + Send + Sync>> {
    let found = tokio::net::lookup_host((host, 0)).await?;
    let public: Vec<SocketAddr> = found.filter(|addr| is_public(addr.ip())).collect();
    if public.is_empty() {
        return Err(Box::new(Refusal::PrivateAddress));
    }
    Ok(public)
}

struct PublicResolver;

impl Resolve for PublicResolver {
    fn resolve(&self, name: Name) -> Resolving {
        let host = name.as_str().to_owned();
        Box::pin(async move {
            let addrs = resolve_public(&host).await?;
            Ok(Box::new(addrs.into_iter()) as Addrs)
        })
    }
}

/// The only HTTP client link previews use.
pub fn client() -> reqwest::Result<Client> {
    Client::builder()
        .https_only(true)
        .no_proxy()
        .dns_resolver(Arc::new(PublicResolver))
        .redirect(redirect::Policy::custom(|attempt| {
            if attempt.previous().len() >= MAX_REDIRECTS {
                return attempt.error(Refusal::TooManyRedirects);
            }
            match check_url(attempt.url()) {
                Ok(()) => attempt.follow(),
                Err(refusal) => attempt.error(refusal),
            }
        }))
        .timeout(TIMEOUT)
        .connect_timeout(Duration::from_secs(4))
        .user_agent(concat!(
            "Panopin/",
            env!("CARGO_PKG_VERSION"),
            " (link preview)"
        ))
        .build()
}

/// Reads at most `MAX_BYTES`. `truncate`: keep what fits (a page's head comes first) instead of
/// giving up.
pub async fn read_body(
    mut response: Response,
    truncate: bool,
) -> Result<Vec<u8>, Box<dyn Error + Send + Sync>> {
    if !truncate
        && response
            .content_length()
            .is_some_and(|len| len > MAX_BYTES as u64)
    {
        return Err(Box::new(Refusal::TooLarge));
    }
    let mut body = Vec::new();
    while let Some(chunk) = response.chunk().await? {
        if body.len() + chunk.len() > MAX_BYTES {
            if truncate {
                body.extend_from_slice(&chunk[..MAX_BYTES - body.len()]);
                break;
            }
            return Err(Box::new(Refusal::TooLarge));
        }
        body.extend_from_slice(&chunk);
    }
    Ok(body)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn check(url: &str) -> Result<(), Refusal> {
        check_url(&Url::parse(url).unwrap())
    }

    #[test]
    fn only_plain_https_urls_to_public_hosts_pass() {
        assert_eq!(check("https://example.com/page?q=kitap&page=2"), Ok(()));
        assert_eq!(check("https://93.184.216.34/"), Ok(()));
        assert_eq!(check("https://[2606:2800:220:1::1]/"), Ok(()));
        assert_eq!(check("http://example.com/"), Err(Refusal::NotHttps));
        assert_eq!(check("ftp://example.com/"), Err(Refusal::NotHttps));
        assert_eq!(check("file:///C:/Windows/win.ini"), Err(Refusal::NotHttps));
        assert_eq!(
            check("https://user:pass@example.com/"),
            Err(Refusal::Credentials)
        );
        assert_eq!(
            check("https://user@example.com/"),
            Err(Refusal::Credentials)
        );
    }

    #[test]
    fn sensitive_query_parameters_are_refused() {
        for url in [
            "https://example.com/?token=abc",
            "https://example.com/?access_token=abc",
            "https://example.com/?API_KEY=abc",
            "https://example.com/?X-Amz-Signature=abc",
            "https://example.com/?password=x",
            "https://example.com/?auth=x",
            "https://example.com/?sessionid=x",
            "https://example.com/callback?code=x&state=y",
            "https://example.com/?client_secret=x",
        ] {
            assert_eq!(check(url), Err(Refusal::SensitiveQuery), "{url}");
        }
        assert_eq!(
            check("https://example.com/search?q=token"),
            Ok(()),
            "values do not count"
        );
    }

    #[test]
    fn local_and_private_targets_are_refused() {
        for url in [
            "https://localhost/",
            "https://LOCALHOST./",
            "https://printer.local/",
            "https://intranet/",
            "https://metadata.google.internal/",
            "https://router.home.arpa/",
            "https://127.0.0.1/",
            "https://127.1.2.3:8443/",
            "https://0.0.0.0/",
            "https://10.0.0.5/",
            "https://172.16.0.1/",
            "https://172.31.255.255/",
            "https://192.168.1.1/",
            "https://169.254.169.254/latest/meta-data/",
            "https://100.64.0.1/",
            "https://224.0.0.1/",
            "https://255.255.255.255/",
            "https://[::1]/",
            "https://[::]/",
            "https://[fe80::1]/",
            "https://[fc00::1]/",
            "https://[fd12:3456::1]/",
            "https://[ff02::1]/",
            "https://[::ffff:127.0.0.1]/",
            "https://[::ffff:192.168.0.1]/",
            "https://[64:ff9b::a00:1]/",
            "https://[2001:db8::1]/",
            "https://[2002:7f00:1::]/",
        ] {
            assert_eq!(check(url), Err(Refusal::PrivateAddress), "{url}");
        }
        // Decimal and hex forms are normalised by the URL parser before the check.
        assert_eq!(check("https://2130706433/"), Err(Refusal::PrivateAddress));
        assert_eq!(check("https://0x7f.1/"), Err(Refusal::PrivateAddress));
    }

    #[test]
    fn address_ranges() {
        let ip = |s: &str| s.parse::<IpAddr>().unwrap();
        for public in [
            "8.8.8.8",
            "1.1.1.1",
            "172.32.0.1",
            "100.128.0.1",
            "2a00:1450:4001::200e",
        ] {
            assert!(is_public(ip(public)), "{public}");
        }
        for private in [
            "172.20.1.1",
            "198.18.0.1",
            "203.0.113.9",
            "240.0.0.1",
            "2001::1",
            "::ffff:10.1.1.1",
        ] {
            assert!(!is_public(ip(private)), "{private}");
        }
    }

    #[test]
    fn names_that_resolve_to_this_machine_are_refused() {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        let refused = runtime.block_on(resolve_public("localhost")).unwrap_err();
        assert_eq!(
            refused.downcast_ref::<Refusal>(),
            Some(&Refusal::PrivateAddress)
        );
    }

    #[test]
    fn the_client_never_sends_a_refused_request() {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        let client = client().unwrap();
        runtime.block_on(async {
            // http is refused by the client itself; a private https host by the resolver.
            assert!(client.get("http://example.com/").send().await.is_err());
            let refused = client.get("https://localhost:9/").send().await.unwrap_err();
            assert!(refused.is_connect() || refused.is_request(), "{refused:?}");
        });
    }
}
