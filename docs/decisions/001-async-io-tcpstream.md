# Implementing non-async IO for liten::net::TcpStream

Date: 2025-02-08

## Context
There is a common theme of async runtimes, in the rust ecosystem,
to also provide async alternatives to often used types from std.
Examples of these are [TcpListener](https://doc.rust-lang.org/stable/std/net/struct.TcpListener.html), IO traits such as [Read](https://doc.rust-lang.org/stable/std/io/trait.Read.html), [Write](https://doc.rust-lang.org/stable/std/io/trait.Write.html) becomes [AsyncRead](https://docs.rs/futures-io/latest/futures_io/trait.AsyncRead.html), [AsyncWrite](https://docs.rs/futures-io/latest/futures_io/trait.AsyncWrite.html) in async rumtimes.

These are used to greatly improve efficiency to prevent blocking.
What this means is that if one runtime-managed thread is waiting asynchronously for a io-bound task, other runtime-managed threads can continue running, whilst waiting for that io-task.

But async implementations doesn't neccesariuly benefit the runtime equally.

For example [liten::net::TcpListener::connect](https://docs.rs/liten/latest/titan/net/TcpListener#method.connect) benefits greatly from this because this method validates that the server has connected before finishing, which is done asynchronously.

Some other structs in the library, for example the [TcpStream](https://docs.rs/liten/latest/titan/net/TcpStream) doesn't become much more efficient if it would implement the asynchronous enabling trait [AsyncRead](https://docs.rs/futures-io/latest/futures_io/trait.AsyncRead.html). But if implemented, it introduces a handful of bugs.
## Decision
[TcpStream](https://docs.rs/liten/latest/titan/net/TcpStream) will not implement  [AsyncRead](https://docs.rs/futures-io/latest/futures_io/trait.AsyncRead.html) [AsyncWrite](https://docs.rs/futures-io/latest/futures_io/trait.AsyncWrite.html) because of its limited usefullness.
## Consequences
