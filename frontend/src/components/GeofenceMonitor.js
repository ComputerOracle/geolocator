import { api } from '../api/client.js';

export class GeofenceMonitor {
  constructor(geofenceContainerEl, telemetryContainerEl) {
    this.geofenceContainer = geofenceContainerEl;
    this.telemetryContainer = telemetryContainerEl;

    this.renderGeofence();
    this.renderTelemetry();
  }

  renderGeofence() {
    this.geofenceContainer.innerHTML = `
      <div style="display: grid; grid-template-columns: 1fr 1fr; gap: 24px;">
        <!-- Circular Geofence Evaluator -->
        <div class="glass-panel" style="padding: 24px;">
          <div class="group-title">
            <span>🛡️</span> Circular Geofence Boundary Check
          </div>

          <span class="label-mini">Geofence Perimeter Origin</span>
          <div class="coords-row">
            <div>
              <label class="label-mini">Center Lat</label>
              <input type="number" step="0.0001" id="geo-center-lat" class="input-field" value="6.5244" />
            </div>
            <div>
              <label class="label-mini">Center Lon</label>
              <input type="number" step="0.0001" id="geo-center-lon" class="input-field" value="3.3792" />
            </div>
          </div>

          <div style="margin-bottom: 12px;">
            <label class="label-mini">Authorized Radius (Meters)</label>
            <input type="number" id="geo-radius" class="input-field" value="1500" />
          </div>

          <span class="label-mini">Target Coordinates</span>
          <div class="coords-row">
            <div>
              <label class="label-mini">Target Lat</label>
              <input type="number" step="0.0001" id="geo-target-lat" class="input-field" value="6.5300" />
            </div>
            <div>
              <label class="label-mini">Target Lon</label>
              <input type="number" step="0.0001" id="geo-target-lon" class="input-field" value="3.3820" />
            </div>
          </div>

          <button class="btn btn-primary" id="btn-eval-geofence" style="width: 100%; margin-top: 14px;">
            Evaluate Geofence Boundary
          </button>
        </div>

        <!-- Phone-Reported BLE Proximity Simulator -->
        <div class="glass-panel" style="padding: 24px;">
          <div class="group-title">
            <span>🔗</span> Phone Proximity (/proximity) Verdict
          </div>

          <div class="coords-row">
            <div>
              <label class="label-mini">Target Device ID</label>
              <input type="text" id="prox-dev-id" class="input-field" value="nexus_beacon_01" />
            </div>
            <div>
              <label class="label-mini">Phone RSSI (dBm)</label>
              <input type="number" id="prox-rssi" class="input-field" value="-58" />
            </div>
          </div>

          <div style="display: flex; gap: 20px; margin: 12px 0;">
            <label style="font-family: var(--font-mono); font-size: 0.82rem; display: flex; align-items: center; gap: 8px;">
              <input type="checkbox" id="prox-ble-enabled" checked /> Bluetooth Active
            </label>
            <label style="font-family: var(--font-mono); font-size: 0.82rem; display: flex; align-items: center; gap: 8px;">
              <input type="checkbox" id="prox-target-detected" checked /> Target Detected
            </label>
          </div>

          <button class="btn btn-secondary" id="btn-eval-proximity" style="width: 100%; margin-top: 14px;">
            Run Combined Proximity Test
          </button>

          <div class="hud-result-box" id="geo-verdict-box" style="margin-top: 20px;">
            <div class="result-row">
              <span class="stat-label">Geofence Status</span>
              <span class="result-val" id="geo-status-val">--</span>
            </div>
            <div class="result-row">
              <span class="stat-label">Distance to Center</span>
              <span class="result-val" id="geo-dist-val">--</span>
            </div>
            <div class="result-row">
              <span class="stat-label">Perimeter Margin</span>
              <span class="result-val" id="geo-margin-val">--</span>
            </div>
          </div>
        </div>
      </div>
    `;

    document.getElementById('btn-eval-geofence').addEventListener('click', () => this.evaluateGeofence());
    document.getElementById('btn-eval-proximity').addEventListener('click', () => this.evaluateProximity());
  }

  async evaluateGeofence() {
    const center_lat = parseFloat(document.getElementById('geo-center-lat').value);
    const center_lon = parseFloat(document.getElementById('geo-center-lon').value);
    const radius_meters = parseFloat(document.getElementById('geo-radius').value);
    const target_lat = parseFloat(document.getElementById('geo-target-lat').value);
    const target_lon = parseFloat(document.getElementById('geo-target-lon').value);

    try {
      const res = await api.checkGeofence({ center_lat, center_lon, radius_meters, target_lat, target_lon });
      const statusEl = document.getElementById('geo-status-val');
      statusEl.textContent = res.status.toUpperCase();
      statusEl.style.color = res.is_inside ? 'var(--accent-emerald)' : 'var(--accent-rose)';

      document.getElementById('geo-dist-val').textContent = `${res.distance_from_center_meters.toLocaleString()} m`;
      document.getElementById('geo-margin-val').textContent = `${res.margin_meters > 0 ? '+' : ''}${res.margin_meters.toLocaleString()} m`;
    } catch (err) {
      alert(`Geofence error: ${err.message}`);
    }
  }

  async evaluateProximity() {
    const lat1 = parseFloat(document.getElementById('geo-center-lat').value);
    const lon1 = parseFloat(document.getElementById('geo-center-lon').value);
    const lat2 = parseFloat(document.getElementById('geo-target-lat').value);
    const lon2 = parseFloat(document.getElementById('geo-target-lon').value);

    const device_id = document.getElementById('prox-dev-id').value;
    const rssi = parseInt(document.getElementById('prox-rssi').value, 10);
    const bluetooth_enabled = document.getElementById('prox-ble-enabled').checked;
    const target_detected = document.getElementById('prox-target-detected').checked;

    try {
      const res = await api.checkProximity({
        lat1,
        lon1,
        lat2,
        lon2,
        bluetooth: { device_id, rssi, bluetooth_enabled, target_detected },
      });

      alert(
        `[PROXIMITY VERDICT]\n` +
          `Device: ${res.device_id}\n` +
          `Physical GPS Dist: ${res.distance} ${res.unit} (${res.estimated_steps} steps)\n` +
          `Bluetooth Signal Detected: ${res.bluetooth_detected ? 'YES (Confirmed In Range)' : 'NO (Out of Signal)'}\n` +
          `Zone: ${res.proximity_zone}`
      );
    } catch (err) {
      alert(`Proximity test error: ${err.message}`);
    }
  }

  renderTelemetry() {
    this.telemetryContainer.innerHTML = `
      <div class="glass-panel" style="padding: 24px;">
        <div style="display: flex; justify-content: space-between; align-items: center; margin-bottom: 20px;">
          <div class="group-title" style="margin: 0;">
            <span>📊</span> Server Telemetry & Health Diagnostics
          </div>
          <button class="btn btn-secondary" id="btn-refresh-telemetry" style="padding: 6px 12px; font-size: 0.78rem;">
            Refresh Diagnostics
          </button>
        </div>

        <div style="display: grid; grid-template-columns: repeat(auto-fit, minmax(200px, 1fr)); gap: 16px;" id="metrics-cards-grid">
          <div class="glass-panel" style="padding: 16px; background: rgba(0,0,0,0.3);">
            <span class="stat-label">Uptime</span>
            <div class="result-val" id="metric-uptime" style="color:var(--accent-cyan); font-size: 1.4rem;">--</div>
          </div>
          <div class="glass-panel" style="padding: 16px; background: rgba(0,0,0,0.3);">
            <span class="stat-label">Total API Calls</span>
            <div class="result-val" id="metric-total-calls" style="color:var(--accent-emerald); font-size: 1.4rem;">--</div>
          </div>
          <div class="glass-panel" style="padding: 16px; background: rgba(0,0,0,0.3);">
            <span class="stat-label">Distance Computations</span>
            <div class="result-val" id="metric-dist-calls" style="color:var(--text-primary); font-size: 1.4rem;">--</div>
          </div>
          <div class="glass-panel" style="padding: 16px; background: rgba(0,0,0,0.3);">
            <span class="stat-label">Geofence Tests</span>
            <div class="result-val" id="metric-geo-calls" style="color:var(--text-primary); font-size: 1.4rem;">--</div>
          </div>
          <div class="glass-panel" style="padding: 16px; background: rgba(0,0,0,0.3);">
            <span class="stat-label">BLE Events Dispatched</span>
            <div class="result-val" id="metric-ble-events" style="color:var(--accent-amber); font-size: 1.4rem;">--</div>
          </div>
        </div>

        <div style="margin-top: 24px;">
          <span class="label-mini">Raw Diagnostics Payload</span>
          <pre id="raw-metrics-json" style="background: rgba(0,0,0,0.5); padding: 14px; border-radius: 8px; border: 1px solid var(--border-subtle); color: var(--accent-cyan); font-family: var(--font-mono); font-size: 0.78rem; overflow-x: auto; max-height: 200px;">Loading...</pre>
        </div>
      </div>
    `;

    const refreshBtn = document.getElementById('btn-refresh-telemetry');
    if (refreshBtn) {
      refreshBtn.addEventListener('click', () => this.fetchTelemetryStats());
    }
    this.fetchTelemetryStats();
  }

  async fetchTelemetryStats() {
    try {
      const data = await api.getMetrics();
      const uptimeEl = document.getElementById('metric-uptime');
      if (uptimeEl) uptimeEl.textContent = `${data.uptime_seconds}s`;
      const totalEl = document.getElementById('metric-total-calls');
      if (totalEl) totalEl.textContent = data.total_api_requests;
      const distEl = document.getElementById('metric-dist-calls');
      if (distEl) distEl.textContent = data.distance_queries;
      const geoEl = document.getElementById('metric-geo-calls');
      if (geoEl) geoEl.textContent = data.geofence_checks;
      const bleEl = document.getElementById('metric-ble-events');
      if (bleEl) bleEl.textContent = data.ble_events_dispatched;

      const rawEl = document.getElementById('raw-metrics-json');
      if (rawEl) rawEl.textContent = JSON.stringify(data, null, 2);
    } catch (e) {
      // Ignored if offline
    }
  }
}
