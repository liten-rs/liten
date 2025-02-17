# Liten

An async runtime for Rust, designed to be performant, minimal and simple.

# Motivation

I wanted to see how an async runtime worked in rust, so i created my own. I wanted to build this with these criteria:

* Minimal Overhead – Focused on performance and low-latency execution.
* No Unnecessary Dependencies – Lightweight and focused design.
* Efficient - Making the most use of the users/servers CPUs.

# Technicals

* Mutlithreaded: Built on the [N:M](https://en.wikipedia.org/wiki/Thread_(computing)#M:N_(hybrid_threading)) threading model.
* Work stealing scheduler: Making the most use of the CPUs.
* Networking ready: [TcpListener](https://docs.rs/liten/latest/liten/net/struct.TcpListener.html)/[UdpSocket](https://docs.rs/liten/latest/liten/net/struct.UdpSocket.html) builtin.
* Efficient IO handling: liten is using the widely used [mio](https://docs.rs/mio) library for its IO event loop.

# LICENSE

The project is licensed under the [MIT License](https://opensource.org/license/mit).
