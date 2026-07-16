# 🦀 rust-http-server

[![CI](https://github.com/YOUR_USERNAME/rust-http-server/actions/workflows/ci.yml/badge.svg)](https://github.com/YOUR_USERNAME/rust-http-server/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-2021_edition-orange.svg)](https://www.rust-lang.org/)

A tiny multithreaded HTTP server written in **pure Rust** — zero dependencies, just the standard library.

## Features

- 🚀 **Zero dependencies** — built entirely on `std`
- 🧵 **Thread pool** — handles connections concurrently with a fixed pool of workers
- 📁 **Static file serving** — serves files from the `public/` directory
- 🔒 **Path traversal protection** — `../` requests are rejected
- 🎯 **Correct MIME types** — HTML, CSS, JS, JSON, images and more
- 🧹 **Graceful shutdown** — workers finish their jobs on drop

## Quick start

```bash
git clone https://github.com/YOUR_USERNAME/rust-http-server.git
cd rust-http-server
cargo run
```

Then open <http://127.0.0.1:8080> in your browser.

### Custom port

```bash
cargo run -- 3000
```

## How it works

```
                   ┌──────────────┐
 TCP connection ──▶│  TcpListener │
                   └──────┬───────┘
                          │ job
                   ┌──────▼───────┐
                   │  ThreadPool  │  mpsc channel + Arc<Mutex<Receiver>>
                   ├──────────────┤
                   │ worker-0     │
                   │ worker-1     │──▶ parse request ──▶ read file ──▶ respond
                   │ worker-2     │
                   │ worker-3     │
                   └──────────────┘
```

1. `TcpListener` accepts incoming connections on the main thread.
2. Each connection is sent as a job into an `mpsc` channel.
3. Worker threads pick up jobs, parse the request line, resolve the path
   inside `public/`, and write an HTTP/1.1 response.
4. Unknown paths get a custom `404.html`; non-GET methods get `405`.

## Project structure

```
├── src/
│   ├── main.rs         # server: listener, request parsing, responses
│   └── thread_pool.rs  # thread pool implementation
├── public/
│   ├── index.html      # demo page
│   └── 404.html        # not-found page
└── .github/workflows/
    └── ci.yml          # build + test + clippy + fmt on every push
```

## Running tests

```bash
cargo test
```

## Roadmap

- [ ] Keep-alive connections
- [ ] Request headers parsing
- [ ] Directory listings
- [ ] Config file support

## License

[MIT](LICENSE) — do whatever you want, just keep the copyright notice.
