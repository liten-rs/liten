# Using loom in testing

Date: 2025-03-25

Status: accepted

## Context

Loom is a library built my the tokio-team to harden multithreaded apps.
It hooks into the ordering of many multithreaded primitives in rust, which enables the library to replay all possible order-of-execution scenarios.

## Decision

This is an exceptional library which has already catched two bugs in the oneshot implementation before even fully commiting the loom setup.

## Consequences
Better reproducability.
