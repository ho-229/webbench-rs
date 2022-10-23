# Webbench

![windows](https://github.com/ho-229/webbench-rs/workflows/build/badge.svg?style=flat-square)
![lines](https://tokei.rs/b1/github/ho-229/webbench-rs)

This is a Rust refactored version of [Radim Kolar - Web Bench](http://home.tiscali.cz/~cz210552/webbench.html), which uses tokio runtime to handle concurrent I/O tasks.

## Features

- HTTP
  - [x] HTTP 0.9/1.0/1.1
  - [ ] HTTP 2
- [x] Keep alive.
- [x] Customizable header.
- [x] Use proxy server for request.
- [ ] TLS connections support.

## Usage

- Build

  ```shell
  cargo build --release
  ```

- Command line usage

  ```txt
  Usage: webbench [OPTIONS] <URL>

  Arguments:
    <URL>
  
  Options:
    -t, --time \<TIME>
            Run benchmark for \<TIME> seconds

            [default: 30]

    -c, --client <CLIENT>
            Run <CLIENT> HTTP clients at once

            [default: 1]

    -p, --proxy <PROXY>
            Use proxy server for request

    -k, --keep
            Keep-Alive

    -m, --method <METHOD>
            Use <METHOD> request method

            [default: get]
            [possible values: options, get, head, trace]

        --http <HTTP>
            Use <HTTP> version for request
          
            [default: h11]

            Possible values:
            - h09: HTTP 0.9
            - h10: HTTP 1.0
            - h11: HTTP 1.1

        --header <HEADER>
            Send requests using customized header

    -h, --help
            Print help information (use `-h` for a summary)

    -V, --version
            Print version information
  ```
