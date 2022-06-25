# Webbench

![windows](https://github.com/ho-229/webbench-rs/workflows/build/badge.svg?style=flat-square)
![lines](https://tokei.rs/b1/github/ho-229/webbench-rs)

This is a Rust refactored version of [Radim Kolar - Web Bench](http://home.tiscali.cz/~cz210552/webbench.html), which uses tokio runtime to handle concurrent I/O tasks.

## Features

- HTTP
  - [x] HTTP 0.9/1.0/1.1
  - [ ] HTTP 2
- [x] Keep alive.
- [ ] Customizable header.
- [x] Use proxy server for request.

## Usage

- Build

  ```shell
  cargo build --release
  ```

- Command line usage

  ```txt
  USAGE:
      webbench [OPTIONS] <URL>

  ARGS:
      <URL>    URL address.

  OPTIONS:
      -c, --client <N>         Run <N> HTTP clients at once. [default: 1]
      -h, --http <http>        Use HTTP/[0.9, 1.0, 1.1] version. [default: 1.1]
          --help               Print help information
      -k, --keep               Keep-Alive.
      -m, --method <method>    Use [GET, HEAD, OPTIONS, TRACE] request method. [default: GET]
      -p, --proxy <server:port>    Use proxy server for request.
      -t, --time <sec>         Run benchmark for <sec> seconds. [default: 30]
      -V, --version            Print version information
  ```
