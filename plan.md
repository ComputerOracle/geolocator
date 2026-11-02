# Wave Program: Issues I'd Post as a Maintainer

Geolocator is a Rust/Actix Web backend combining GPS distance
calculations with live Bluetooth proximity scanning. Below are the
categories of scoped issues I'd open for contributors, each sized for
a single sprint.

## Bug Fixes
- **Validate GPS input**: `/distance` and `/proximity` currently
  accept any number for lat/lon, including out-of-range values (e.g.
  latitude > 90) or NaN. Add validation that returns a clean 400 error
  instead of silently producing garbage math.
- **Handle a poisoned cache lock gracefully**: the Bluetooth device
  cache uses `Mutex::lock().unwrap()` in a few places - if a thread
  ever panics while holding the lock, every future request panics
  too. Replace with graceful error handling.
- **Quiet the retry log spam**: when no Bluetooth adapter exists (e.g.
  in a cloud deployment), the scanner logs an error every 3 seconds
  forever. Add exponential backoff and cap the log frequency.

## New Features
- **Distance units**: accept an optional `unit` field (`meters` |
  `miles` | `feet`) on `/distance` and `/proximity`.
- **Configurable step threshold**: make the step threshold a query
  parameter instead of a hardcoded constant, so callers can set their
  own "close enough" limit.
- **Minimal browser tester page**: a static HTML page (no framework)
  with a form that calls `/distance` and displays the result, so
  non-technical reviewers can try the API without curl/Postman.
- **Device history**: keep the last N readings per Bluetooth device
  (not just the latest) so a caller can see whether a device is
  trending closer or further away over time.
- **Rate limiting**: basic per-IP rate limiting on all routes to
  protect the free-tier deployment from abuse.

## Documentation
- **OpenAPI spec**: write an `openapi.yaml` describing all routes with
  example requests/responses, so the API is discoverable without
  reading source code.
- **Architecture doc**: a short write-up explaining how GPS math, the
  background Bluetooth scanner, and Actix routing fit together -
  useful onboarding for contributors unfamiliar with async Rust.
- **Contributor setup guide**: document the exact system dependencies
  (`libdbus-1-dev`, `pkg-config`) and common build errors, since
  these aren't obvious to newcomers.

## Testing
- **Integration tests**: use `actix_web::test` to spin up the app and
  hit `/distance`, `/proximity`, and `/scan` with real HTTP requests,
  instead of only unit-testing the math functions in isolation.
- **Edge case coverage**: tests for bearing wraparound (359° → 0°),
  identical coordinates, and extreme RSSI values (`0`, `-127`).
- **Concurrency test**: verify the shared device cache behaves
  correctly under many simultaneous `/scan` requests while the
  background scanner is also writing to it.

## Tooling / CI
- **GitHub Actions workflow**: run `cargo test`, `cargo clippy`, and
  `cargo fmt --check` on every pull request.
- **Docker build check**: add a CI step that builds the `Dockerfile`
  on every PR, so a broken deploy is caught before merge, not after.

## Refactoring
- **Extract shared GPS logic**: `/distance` and `/proximity` both
  repeat the same "calculate distance, steps, bearing" block - pull
  it into one helper function both routes call, to avoid the two
  drifting apart as the project grows.
