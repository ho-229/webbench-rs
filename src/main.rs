use std::{time::Duration, sync::atomic::Ordering};
use clap::{arg, Command};
use http::{request, Method, Uri, Version};

mod core;

fn main() -> core::Result<()> {
    let (config, mut time) = parse_args()?;

    println!("Welcome to the Webbench.");

    print!("Request:\n{}", std::str::from_utf8(&config.request).unwrap());
    println!("\nRunning info: {} client(s), running {} sec.", config.clients, time);
    
    let benchmark = core::Webbench::new(config)?;

    benchmark.start();

    loop {
        let success = benchmark.status().success.load(Ordering::Acquire);
        let failed = benchmark.status().failed.load(Ordering::Acquire);

        let print_result = || {
            println!("Recvied: {} bytes.", benchmark.status().recived.load(Ordering::Acquire));
            println!("Requests: {} success, {} failed.", success, failed);
        };

        if failed as f64 / success as f64 > 0.5 {
            println!("Too many failures.");
            print_result();
            break;
        }

        if time == 0 {
            print_result();
            break;
        }

        std::thread::sleep(Duration::from_secs(1));
        time -= 1;
    }

    benchmark.stop(Duration::from_secs(1));
    
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
                        else { Err("<sec> must be greater than 0".to_string()) }
                    },
                    Err(err) => Err(err.to_string())
                }
            }))

        .arg(arg!(-c --client "Run <N> HTTP clients at once.")
            .value_name("N").default_value("1").validator(|num| {
                match num.parse::<usize>() {
                    Ok(n) => {
                        if n > 0 {  clients = n; Ok(()) }
                        else { Err("N must be greater than 0".to_string()) }
                    },
                    Err(err) => Err(err.to_string())
                }
            }))

        .arg(arg!(-k --keep "Keep-Alive"))
        //.arg(arg!(-f --force "Don't wait for reply from server."))

        .arg(arg!(-m --method "Use [GET(default), HEAD, OPTIONS, TRACE] request method")
            .default_value("GET").validator(|str| {
                match str.parse::<Method>() {
                    Ok(res) => {
                        match res {
                            Method::GET | Method::HEAD | Method::OPTIONS | Method::TRACE => {
                                method = res; Ok(())
                            },
                            _ => Err("Only [GET(default), HEAD, OPTIONS, TRACE] are supported by --method".to_string())
                        }
                    },
                    Err(e) => Err(e.to_string())
                }
            }))

        .arg(arg!(-h --http "Use HTTP/[1.1, 2] version")
            .default_value("1.1").validator(|str| {
                match str {
                    "1.1" => { version = Version::HTTP_11; Ok(()) },
                    "2" => { version = Version::HTTP_2; Ok(()) },
                    _ => Err("Only [1.1(default), 2] are supported by --http".to_string())
                }
            }))

        .arg(arg!([URL] "URL address").required(true).validator(|str| {
            match str.parse::<Uri>() {
                Ok(u) =>{ uri = u; Ok(()) },
                Err(e) => Err(e.to_string())
            }
        }))
        
        .get_matches();

    let is_keepalive = args.is_present("keep");

    let connection = if is_keepalive { "keep-alive" } else { "close" };

    let request = request::Builder::new()
        .method(method)
        .uri(uri.clone())
        .version(version)
        .header("User-Agent", "webbench-rs")
        .header("Host", uri.clone().host().unwrap())
        .header("Connection", connection)
        .body(())?;

    Ok((core::Config {
        addr: (request.uri().host().unwrap().to_string(), 
            request.uri().port_u16().unwrap_or(match uri.scheme_str().unwrap() {
                "https" => 443,
                _ => 80,
        })),
        request: core::protocol::raw_request(request)?,
        is_keepalive,
        clients,
    }, time))
}
