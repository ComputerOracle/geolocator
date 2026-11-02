# Geolocator

A Rust + Actix Web backend for proximity-aware navigation: it turns GPS
coordinates into real-world distance, walking steps, and direction, and
separately does **real, live Bluetooth signal scanning** on the machine
it runs on to estimate how close nearby devices actually are.

GPS gives you *where* two points are on Earth. Bluetooth gives you *how
close* a device physically is right now, based on real signal strength
(RSSI) - not a simulation. This project combines both: deterministic
GPS math for exact coordinates, and live hardware scanning for
ambient nearby devices.

## What it does

- 📍 Accepts two GPS points and calculates the distance between them
  (Haversine formula, accounts for Earth's curvature)
- 🚶 Converts that distance into an estimated number of walking steps
- 📏 Checks whether that's within a step threshold (10,000 by default)
- 🧭 Calculates the compass bearing/direction from point A to point B
- 📡 Scans for real nearby Bluetooth devices on the host machine and
  reports their live signal strength (RSSI), converted into an
  estimated distance and step count
- 🔗 Combines GPS + a phone-reported Bluetooth status into one verdict
  (`/proximity`)

## Setup

Needs a system Bluetooth library to build (for real device scanning):

```bash
sudo apt install -y libdbus-1-dev pkg-config
```

Then:

```bash
cargo run
```

Server starts on `http://0.0.0.0:8080` (reachable from other devices on
the same WiFi, using this machine's local IP instead of 0.0.0.0).

## Endpoints

### `GET /`
Health check.
```bash
curl http://127.0.0.1:8080/
```

### `POST /distance`
GPS only - distance, steps, threshold check, bearing. Fully
deterministic, same result every time, no hardware involved.
```bash
curl -X POST http://127.0.0.1:8080/distance \
  -H "Content-Type: application/json" \
  -d '{"lat1": 6.5244, "lon1": 3.3792, "lat2": 6.6018, "lon2": 3.3515}'
```

### `POST /proximity`
GPS + a phone-reported Bluetooth status, combined into one response.
```bash
curl -X POST http://127.0.0.1:8080/proximity \
  -H "Content-Type: application/json" \
  -d '{
    "lat1": 6.5244, "lon1": 3.3792,
    "lat2": 6.5290, "lon2": 3.3810,
    "bluetooth": {
      "device_id": "phone_a",
      "bluetooth_enabled": true,
      "target_detected": true,
      "rssi": -55
    }
  }'
```
`bluetooth_enabled` and `target_detected` are things a real phone's own
Bluetooth scan would report to us - this endpoint doesn't scan
Bluetooth itself, it just accepts that info as part of the request
(same way it accepts GPS coordinates).

### `GET /scan`
Real, live Bluetooth scan of the host machine - no request body needed.
Runs continuously in the background for as long as the server is up, so
this answers instantly with whatever's been heard recently, closest
device first.
```bash
curl -s http://127.0.0.1:8080/scan | jq
```

To watch it update live on the terminal as devices move closer/further
away (closest 5, one per line):
```bash
watch -n 1 'curl -s http://127.0.0.1:8080/scan | jq -c ".[:5][]"'
```

Note: real Bluetooth signal strength is inherently a little noisy and
intermittent by nature (phones broadcast in occasional bursts to save
battery, not constantly) - a device dropping out of the list for a few
seconds is expected, not a bug.

## Tests

```bash
cargo test
```

## Contributors

Thanks to all the contributors who built and improved Geolocator!

<table>
  <tr>
    <td align="center"><a href="https://github.com/greatKhalifa-code"><img src="https://github.com/greatKhalifa-code.png" width="80px;" alt="greatKhalifa-code"/><br /><sub><b>greatKhalifa-code</b></sub></a></td>
    <td align="center"><a href="https://github.com/queentiffany1111-cloud"><img src="https://github.com/queentiffany1111-cloud.png" width="80px;" alt="queentiffany1111-cloud"/><br /><sub><b>queentiffany1111-cloud</b></sub></a></td>
    <td align="center"><a href="https://github.com/dangerous-funds"><img src="https://github.com/dangerous-funds.png" width="80px;" alt="dangerous-funds"/><br /><sub><b>dangerous-funds</b></sub></a></td>
    <td align="center"><a href="https://github.com/betiniSparks"><img src="https://github.com/betiniSparks.png" width="80px;" alt="betiniSparks"/><br /><sub><b>betiniSparks</b></sub></a></td>
    <td align="center"><a href="https://github.com/dangerous-fundz"><img src="https://github.com/dangerous-fundz.png" width="80px;" alt="dangerous-fundz"/><br /><sub><b>dangerous-fundz</b></sub></a></td>
    <td align="center"><a href="https://github.com/davidajohhnson"><img src="https://github.com/davidajohhnson.png" width="80px;" alt="davidajohhnson"/><br /><sub><b>davidajohhnson</b></sub></a></td>
  </tr>
  <tr>
    <td align="center"><a href="https://github.com/Dev-Chizzy"><img src="https://github.com/Dev-Chizzy.png" width="80px;" alt="Dev-Chizzy"/><br /><sub><b>Dev-Chizzy</b></sub></a></td>
    <td align="center"><a href="https://github.com/zicogoodness"><img src="https://github.com/zicogoodness.png" width="80px;" alt="zicogoodness"/><br /><sub><b>zicogoodness</b></sub></a></td>
    <td align="center"><a href="https://github.com/nathanfrealmusic"><img src="https://github.com/nathanfrealmusic.png" width="80px;" alt="nathanfrealmusic"/><br /><sub><b>nathanfrealmusic</b></sub></a></td>
    <td align="center"><a href="https://github.com/the-Devdrago"><img src="https://github.com/the-Devdrago.png" width="80px;" alt="the-Devdrago"/><br /><sub><b>the-Devdrago</b></sub></a></td>
    <td align="center"><a href="https://github.com/bixbyhalleluyah"><img src="https://github.com/bixbyhalleluyah.png" width="80px;" alt="bixbyhalleluyah"/><br /><sub><b>bixbyhalleluyah</b></sub></a></td>
    <td align="center"><a href="https://github.com/computeroracleinc"><img src="https://github.com/computeroracleinc.png" width="80px;" alt="computeroracleinc"/><br /><sub><b>computeroracleinc</b></sub></a></td>
  </tr>
</table>

