use std::{sync::atomic::Ordering, time::Duration};
use clap::{arg, Command, crate_version};
use hyper::{Method, Uri, Version};

mod core;

#[tokio::main]
async fn main() {
    let (config, mut time) = parse_args();

    println!("Welcome to the Webbench.");
    println!("\n{:#?}", config.build_request());

    println!("\nRunning info: {} client(s), running {} sec.", config.clients, time);
    
    let mut benchmark = core::Webbench::new(config);

    benchmark.start().await;
    
    loop {
        if benchmark.status().failed.load(Ordering::Acquire) >= 100 || time == 0 {
            benchmark.stop().await;
            break;
        }

        tokio::time::sleep(Duration::from_secs(1)).await;
        time -= 1;
    }

    benchmark.wait().await;

    let status = benchmark.status();
    println!("Request: {} success, {} failed", status.success.load(Ordering::Relaxed), 
        status.failed.load(Ordering::Relaxed));
}

fn parse_args() -> (core::Config, usize) {
    let mut clients = usize::default();
    let mut time = usize::default();

    let mut method = Method::default();
    let mut uri = Uri::default();
    let mut version = Version::default();

    let args = Command::new("Webbench - Simple Web Benchmark")
        .author("Copyright (c) Ho 229")
        .version(crate_version!())
        
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

    (core::Config {
        request_data: (method, uri, version),
        is_keepalive: args.is_present("keep"),
        is_force: false,    // TODO
        clients
    }, time)
}
