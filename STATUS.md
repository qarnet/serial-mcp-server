# Work in progress

## Goal
Make the server work correctly with CDC-ACM USB serial devices (e.g. WCH-Link on /dev/ttyACM0).

Hardware test command:
```
sudo systemctl stop ModemManager
SERIAL_MCP_TEST_PORT=/dev/ttyACM0 cargo test --test hardware_loopback -- --ignored --test-threads=1
```

---

## Problem 1: `read` tool truncates on CDC-ACM — FIXED

CDC-ACM delivers data in USB packets. A single write may arrive in multiple packets.
`read_bytes` (handler.rs) originally did one `connection.read()` and returned only the
first packet (e.g. got "loopback-tes" instead of "loopback-test\r\n").

**Fix:** `read_bytes` now waits for first bytes then drains for `SETTLE_MS=50ms` to
collect remaining packets before returning. See `handler.rs::read_bytes`.

---

## Problem 2: `wait_for` + concurrent `write` deadlock — IN PROGRESS / BROKEN

### Root cause
`SerialConnection::read()` (serial.rs) held the `io: Mutex<Box<dyn SerialIo>>` for the
entire timeout. A concurrent `connection.write()` call blocked on the same mutex. Data
never looped back so `wait_for` always timed out.

This is triggered by the request/response pattern: issue `wait_for`, then `write` on the
same connection. Exactly what `hw_loopback_wait_for_matches_echo` does.

### Fix attempted
Changed `read()` to poll every 5ms, releasing the lock between polls via `yield_now()`.
This lets `write()` grab the lock between polls.

### Side effect — broke PTY test
`pty_wait_for_matches_real_serial_pattern` now consistently fails (hits 3000ms timeout).
That test writes to the PTY master directly — no mutex contention — so the fix shouldn't
affect it. Something about repeatedly creating/dropping `io.read()` futures on a
tokio-serial `SerialStream` backed by a PTY fd seems to interfere with tokio's
epoll/waker registration.

### Approaches to investigate
A) Replace `yield_now()` with `tokio::time::sleep(Duration::from_millis(1))` to give
   the I/O driver time to process epoll events between polls.

B) Split `SerialConnection` into separate reader and writer mutexes so concurrent
   read+write is possible without polling. Requires refactoring the `SerialIo` trait.

C) Background drain task: a spawned task continuously reads from the serial port into
   a channel buffer; `read()` and `read_until_pattern()` consume from the buffer;
   `write()` goes directly. Architecturally cleanest but largest change.

### Current state of serial.rs `read()`
```rust
pub async fn read(&self, dst: &mut [u8], timeout_ms: Option<u64>) -> Result<usize> {
    const POLL_MS: u64 = 5;
    match timeout_ms {
        None => Ok(self.io.lock().await.read(dst).await?),
        Some(ms) => {
            let deadline = Instant::now() + Duration::from_millis(ms);
            loop {
                let remaining = deadline.saturating_duration_since(Instant::now());
                if remaining.is_zero() { return Err(SerialError::ReadTimeout); }
                let poll_dur = remaining.min(Duration::from_millis(POLL_MS));
                {
                    let mut io = self.io.lock().await;
                    match timeout(poll_dur, io.read(dst)).await {
                        Ok(Ok(n)) if n > 0 => return Ok(n),
                        Ok(Ok(_)) => {}
                        Ok(Err(e)) => return Err(SerialError::from(e)),
                        Err(_elapsed) => {}
                    }
                }
                tokio::task::yield_now().await;
            }
        }
    }
}
```
