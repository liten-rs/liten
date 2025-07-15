# Liten

`liten` is a asynchronous runtime library that runs futures. It does this by incorporating a event-loop (single threaded by default).
It's designed for applications/libraries that are io heavy, meaning that most of the execution-time of the binary is in io (networking, file operations etc).

## Goal
`liten` aims to be as predictable as possible and to be as efficient as possible. It does this by providing a set of API's that turns blocking operations into asynchronous operations, allowing other things to get done inbetween.

## How?
It does this by incorporating a single-threaded event-loop that makes sure everything keeps moving forward.
