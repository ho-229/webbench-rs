use clap::{arg, App};
use hyper::{Method, Uri, Version, Request};

mod core;

fn main() {
    let (clients, request) = parse_args();

    println!("clients: {} \nrequest: {:?}", clients, request);
    core::Webbench::new(request, clients, false, false);
    
    //let mut bench = core::Webbench::new();
    //let mut _benchmark = Webbench::new(build_header(&args), client, parse_flag(&args));
}

fn parse_args() -> (usize, Request<()>) {
    let mut clients = usize::default();

    let mut method = Method::default();
    let mut uri = Uri::default();
    let mut version = Version::default();

    let args = App::new("Webbench - Simple Web Benchmark")
        .author("Copyright (c) Ho 229")
        
        .arg(arg!(-t --time "Run benchmark for <sec> seconds.")
            .value_name("sec").default_value("30"))

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
        .arg(arg!(-f --force "Don't wait for reply from server."))

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

    let connection = if args.is_present("keep") { "keep-alive" } else { "close" };

    let request = Request::builder()
        .method(method)
        .uri(uri)
        .version(version)
        .header("Connection", connection)
        .body(())
        .unwrap();

    (clients, request)
}
