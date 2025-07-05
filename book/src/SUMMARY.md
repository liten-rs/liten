Liten is an ecosystem of a async-style programming. At the heart of this ecosystem is it's async runtime.
liten runtime provides:
- Millisecond presicion time capabilities
- Threadpool
- networking
- other data pattern
- Green threads
- fs operations

All of these completely integrated into async rust. Rusts goal is to make programming as efficient as possible.
It does this by leveraging async to be able to not do work when not neccessary.


<!-- For example in a mpsc channel, when the sender hasn't sent anything, the receiver shouldn't spinlock which would waist CPU-cycles. -->


<!-- ## What is an async runtime? -->
<!-- The api of an async runtime is very simple, but a difficult problem to solve. The only thing an async runtime -->
<!-- is responsible of is calling `poll` on [`Future`](`std::future::Future`)'s provided by the library user. -->
<!-- `Future` is just a trait that allows concurrency to be defined in rusts type system. -->
<!-- By calling the poll method, the Future can return if it is ready to return or if it needs to wait for whatever reason, -->
<!-- calling a "waker" provided by the runtime when it can make progress. -->
