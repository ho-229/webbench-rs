mod core;

use byte_unit::Byte;
use clap::{arg, command, ArgAction, Parser};
use http::{header::HeaderName, request, HeaderValue, Uri};
use std::{
    net::{SocketAddr, ToSocketAddrs},
    str::FromStr,
    sync::atomic::Ordering,
    time::Duration,
};

use crate::core::{Method, Version};

#[derive(Debug, Parser)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(value_parser = Uri::from_str)]
    url: Uri,

    /// Run benchmark for <TIME> seconds.
    #[arg(short, long, default_value_t = 30)]
    #[arg(value_parser = clap::value_parser!(u32).range(1..))]
    time: u32,

    /// Run <CLIENT> HTTP clients at once.
    #[arg(short, long, default_value_t = 1)]
    #[arg(value_parser = clap::value_parser!(u32).range(1..))]
    client: u32,

    /// Use proxy server for request.
    #[arg(short, long)]
    #[arg(value_parser = SocketAddr::from_str)]
    proxy: Option<SocketAddr>,

    /// Keep-Alive
    #[arg(short, long, default_value_t = false)]
    keep: bool,

    /// Use <METHOD> request method.
    #[arg(short, long, value_enum, default_value_t = Method::GET)]
    method: Method,

    /// Use <HTTP> version for request.
    #[arg(long, value_enum, default_value_t = Version::H11)]
    http: Version,

    /// Send requests using customized header.
    #[arg(long, value_parser = parse_header, action = ArgAction::Append)]
    header: Vec<(HeaderName, HeaderValue)>,
}

fn parse_header(header: &str) -> Result<(HeaderName, HeaderValue), String> {
    header
        .split_once(":")
        .ok_or("Invalid HTTP header".to_string())
        .and_then(|h| {
            Ok((
                HeaderName::from_str(h.0)
                    .map_err(|e| format!("Invalid header name: {}", e.to_string()))?,
                HeaderValue::from_str(h.1)
                    .map_err(|e| format!("Invalid header value: {}", e.to_string()))?,
            ))
        })
}

fn parse_args(args: &Args) -> core::Result<core::Config> {
    let is_keepalive = args.keep && args.http == Version::H11;

    let connection = if is_keepalive { "keep-alive" } else { "close" };
    let host = args.url.host().ok_or("Invalid host")?;

    let mut request = request::Builder::new()
        .method(Into::<http::Method>::into(args.method))
        .uri(args.url.clone())
        .version(args.http.into())
        .header("User-Agent", "webbench-rs")
        .header("Host", host)
        .header("Connection", connection);

    request
        .headers_mut()
        .unwrap()
        .extend(args.header.to_owned().into_iter());

    let addrs = args.proxy.map(|p| vec![p]).or_else(|| {
        format!(
            "{}:{}",
            host,
            args.url.port_u16().unwrap_or(80) // Default port of HTTP
        )
        .to_socket_addrs()
        .ok()
        .map(|addrs| addrs.as_slice().to_vec())
    })
    .ok_or("Invalid addrs")?;

    Ok(core::Config {
        addrs,
        request: core::protocol::raw_request(request.body(())?)?,
        is_keepalive,
        clients: args.client,
    })
}

fn main() -> core::Result<()> {
    let args = Args::parse();
    let config = parse_args(&args)?;

    println!("Welcome to the Webbench.\n");

    print!("Request:\n{}", std::str::from_utf8(&config.request)?);
    print!(
        "\nRunning info: {} client(s), running {} sec",
        config.clients, args.time
    );

    if let Some(p) = args.proxy {
        print!(", via proxy server: {}", p);
    }

    println!(".\n");

    let benchmark = core::Webbench::new(config)?;

    benchmark.start()?;

    let mut count = args.time;
    let (success, failed, received) = loop {
        let status = benchmark.status();
        let failed = status.failed.load(Ordering::Acquire);
        let success = status.success.load(Ordering::Acquire);
        let received = status.received.load(Ordering::Acquire);
        let interrupted = status.interrupted.load(Ordering::Acquire);

        if failed as f64 / success as f64 > 0.5 {
            println!("Too many failures.");
            break (success, failed, received);
        } else if interrupted {
            println!("\nInterrupted by user.");
            break (success, failed, received);
        } else if count == 0 {
            break (success, failed, received);
        }

        std::thread::sleep(Duration::from_secs(1));
        count -= 1;
    };

    benchmark.stop();

    println!(
        "Received: total {}, {}/s.",
        Byte::from(received).get_appropriate_unit(true),
        Byte::from((received as f64 / args.time as f64) as u128).get_appropriate_unit(true)
    );
    println!(
        "Requests: {} req/min, {} req/s. {success} success, {failed} failed.",
        (60.00 / args.time as f64 * success as f64) as u32,
        (success as f64 / args.time as f64) as u32
    );

    Ok(())
}
