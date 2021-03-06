use std::{time::Duration, sync::atomic::Ordering, net::{ToSocketAddrs, SocketAddr}, str::FromStr};
use clap::{arg, Command, ArgAction};
use http::{request, Method, Uri, Version, HeaderValue, header::HeaderName};
use byte_unit::Byte;

mod core;

fn main() -> core::Result<()> {
    let (config, time, proxy) = parse_args()?;

    println!("Welcome to the Webbench.\n");

    print!("Request:\n{}", std::str::from_utf8(&config.request).unwrap());
    print!("\nRunning info: {} client(s), running {} sec", config.clients, time);

    if let Some(p) = proxy {
        print!(", via proxy server: {}", p);
    }

    println!(".\n");

    let benchmark = core::Webbench::new(config)?;

    benchmark.start()?;

    let mut count = time;
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

    println!("Received: total {}, {}/s.", Byte::from(received).get_appropriate_unit(true),
        Byte::from((received as f64 / time as f64) as u128).get_appropriate_unit(true));
    println!("Requests: {} req/min, {} req/s. {success} success, {failed} failed.",
        (60.00 / time as f64 * success as f64) as u32, (success as f64 / time as f64) as u32);

    Ok(())
}

fn parse_args() -> core::Result<(core::Config, usize, Option<SocketAddr>)> {
    let mut clients = usize::default();
    let mut time = usize::default();

    let mut method = Method::default();
    let mut uri = Uri::default();
    let mut version = Version::default();

    let mut proxy = None;
    let mut headers = Vec::new();

    let args = Command::new("Webbench")
        .about("Simple Web Benchmark written by Rust.")
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

        .arg(arg!(-h --http "Use HTTP/[0.9, 1.0, 1.1] version.")
            .value_name("version").default_value("1.1").validator(|str| {
                match str {
                    "0.9" => { version = Version::HTTP_09; Ok(()) },
                    "1.0" => { version = Version::HTTP_10; Ok(()) },
                    "1.1" => { version = Version::HTTP_11; Ok(()) },
                    //"2" => { version = Version::HTTP_2; Ok(()) },
                    _ => Err("Only [0.9, 1.0, 1.1] are supported by --http.".to_string())
                }
            }))

        .arg(arg!(-H --header "Send requests using customized header.")
            .value_name("key:value").action(ArgAction::Append).validator(|h| {
                match h.split_once(":") {
                    Some((key, value)) => {
                        headers.push((String::from(key), String::from(value)));
                        Ok(())
                    },
                    None => Err("Parsing header has failed.".to_string())
                }
            }))

        .arg(arg!(-p --proxy "Use proxy server for request.")
            .value_name("server:port").validator(|p| {
                match p.to_socket_addrs() {
                    Ok(mut res) => { proxy = Some(res.next().unwrap()); Ok(()) },
                    Err(e) => Err(e.to_string()),
                }
            }))

        .arg(arg!(<URL> "URL address.").validator(|str| {
            match str.parse::<Uri>() {
                Ok(u) if u.host().is_some() && u.scheme().is_some() => {
                    match u.scheme_str().unwrap() {
                        /*"https" |*/ "http" => { uri = u; Ok(()) }
                        _ => { Err("Scheme unsupported.".to_string()) }
                    }
                },
                Ok(_) => Err("The URL must contain host and scheme.".to_string()),
                Err(e) => Err(e.to_string())
            }
        }))
        
        .get_matches();

    let is_keepalive = args.is_present("keep") && version == Version::HTTP_11;

    let connection = if is_keepalive { "keep-alive" } else { "close" };

    let mut request = request::Builder::new()
        .method(method)
        .uri(uri.clone())
        .version(version)
        .header("User-Agent", "webbench-rs")
        .header("Host", uri.clone().host().unwrap())
        .header("Connection", connection);

    for (key, value) in headers {
        request.headers_mut().unwrap().insert(
            HeaderName::from_str(&key)?, HeaderValue::from_str(&value)?);
    }

    let addrs = if let Some(p) = proxy {
        vec![p]
    } else {
        format!("{}:{}", uri.host().unwrap(), uri.port_u16().unwrap_or(
            match uri.scheme_str().unwrap() {
                //"https" => 443,
                _ => 80,
            }
        )).to_socket_addrs()?.as_slice().to_vec()
    };

    Ok((core::Config {
        addrs,
        request: core::protocol::raw_request(request.body(())?)?,
        is_keepalive,
        clients,
    }, time, proxy))
}
