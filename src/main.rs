use std::{time::Duration, sync::atomic::Ordering, net::ToSocketAddrs};
use clap::{arg, Command};
use http::{request, Method, Uri, Version};
use byte_unit::Byte;

mod core;

fn main() -> core::Result<()> {
    let (config, time) = parse_args()?;

    println!("Welcome to the Webbench.\n");

    print!("Request:\n{}", std::str::from_utf8(&config.request).unwrap());
    println!("\nRunning info: {} client(s), running {} sec.\n", config.clients, time);
    
    let benchmark = core::Webbench::new(config)?;

    benchmark.start();

    let mut count = time;
    let (success, failed, received) = loop {
        let status = benchmark.status();
        let success = status.success.load(Ordering::Acquire);
        let failed = status.failed.load(Ordering::Acquire);
        let received = status.received.load(Ordering::Acquire);

        if failed as f64 / success as f64 > 0.5 {
            println!("Too many failures.");
            break (success, failed, received);
        }

        if count == 0 {
            break (success, failed, received);
        }

        std::thread::sleep(Duration::from_secs(1));
        count -= 1;
    };

    println!("Received: total {}, {}/s.", Byte::from(received).get_appropriate_unit(true),
        Byte::from((received as f64 / time as f64) as u128).get_appropriate_unit(true));
    println!("Requests: {} req/min. {} success, {} failed.",
        (60.00 / time as f64 * success as f64) as u32, success, failed);

    benchmark.stop();
    
    Ok(())
}

fn parse_args() -> core::Result<(core::Config, usize)> {
    let mut clients = usize::default();
    let mut time = usize::default();

    let mut method = Method::default();
    let mut uri = Uri::default();
    let mut version = Version::default();

    let args = Command::new("Webbench - Simple Web Benchmark")
        .author("Copyright (c) Ho 229")
        .version(clap::crate_version!())
        
        .arg(arg!(-t --time "Run benchmark for <sec> seconds.")
            .value_name("sec").default_value("30").validator(|num| {
                match num.parse::<usize>() {
                    Ok(n) => {
                        if n > 0 { time = n; Ok(()) }
                        else { Err("<sec> must be greater than 0.".to_string()) }
                    },
                    Err(err) => Err(err.to_string())
                }
            }))

        .arg(arg!(-c --client "Run <N> HTTP clients at once.")
            .value_name("N").default_value("1").validator(|num| {
                match num.parse::<usize>() {
                    Ok(n) => {
                        if n > 0 {  clients = n; Ok(()) }
                        else { Err("N must be greater than 0.".to_string()) }
                    },
                    Err(err) => Err(err.to_string())
                }
            }))

        .arg(arg!(-k --keep "Keep-Alive."))
        //.arg(arg!(-f --force "Don't wait for reply from server."))

        .arg(arg!(-m --method "Use [GET, HEAD, OPTIONS, TRACE] request method.")
            .default_value("GET").validator(|str| {
                match str.parse::<Method>() {
                    Ok(res) => {
                        match res {
                            Method::GET | Method::HEAD | Method::OPTIONS | Method::TRACE => {
                                method = res; Ok(())
                            },
                            _ => Err("Only [GET, HEAD, OPTIONS, TRACE] are supported by --method.".to_string())
                        }
                    },
                    Err(e) => Err(e.to_string())
                }
            }))

        .arg(arg!(-h --http "Use HTTP/[0.9, 1.1, 2] version.")
            .default_value("1.1").validator(|str| {
                match str {
                    "0.9" => { version = Version::HTTP_09; Ok(()) },
                    "1.0" => { version = Version::HTTP_10; Ok(()) },
                    "1.1" => { version = Version::HTTP_11; Ok(()) },
                    //"2" => { version = Version::HTTP_2; Ok(()) },
                    _ => Err("Only [0.9, 1.0, 1.1] are supported by --http.".to_string())
                }
            }))

        .arg(arg!([URL] "URL address.").required(true).validator(|str| {
            match str.parse::<Uri>() {
                Ok(u) if u.host().is_some() && u.scheme().is_some() => { uri = u; Ok(()) },
                Ok(_) => Err("URI must contain host and scheme.".to_string()),
                Err(e) => Err(e.to_string())
            }
        }))
        
        .get_matches();

    let is_keepalive = args.is_present("keep") && version == Version::HTTP_11;

    let connection = if is_keepalive { "keep-alive" } else { "close" };

    let request = request::Builder::new()
        .method(method)
        .uri(uri.clone())
        .version(version)
        .header("User-Agent", "webbench-rs")
        .header("Host", uri.clone().host().unwrap())
        .header("Connection", connection)
        .body(())?;

    let addr = format!("{}:{}", uri.host().unwrap(), uri.port_u16().unwrap_or(
        match uri.scheme_str().unwrap() {
            "https" => 443,
            _ => 80,
        }
    )).to_socket_addrs()?.as_slice().to_vec();

    Ok((core::Config {
        addr,
        request: core::protocol::raw_request(request)?,
        is_keepalive,
        clients,
    }, time))
}
