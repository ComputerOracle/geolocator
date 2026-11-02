import { api } from '../api/client.js';

export class TripPlanner {
  constructor(containerEl) {
    this.container = containerEl;
    this.presets = [
      { name: 'Lagos Island -> Ikeja', lat1: 6.4541, lon1: 3.4246, lat2: 6.6018, lon2: 3.3515 },
      { name: 'Greenwich -> Westminster (London)', lat1: 51.4769, lon1: -0.0005, lat2: 51.4995, lon2: -0.1273 },
      { name: 'Central Park -> Brooklyn Bridge (NYC)', lat1: 40.7829, lon1: -73.9654, lat2: 40.7061, lon2: -73.9969 },
      { name: 'Shibuya -> Shinjuku (Tokyo)', lat1: 35.6580, lon1: 139.7016, lat2: 35.6938, lon2: 139.7034 },
    ];
    this.render();
  }

  render() {
    this.container.innerHTML = `
      <div style="display: grid; grid-template-columns: 1fr 1fr; gap: 24px;">
        <div class="glass-panel" style="padding: 24px;">
          <div class="group-title">
            <span>📏</span> Multi-Point Route & Stride Estimator
          </div>

          <div style="margin-bottom: 16px;">
            <label class="label-mini">Select Location Preset</label>
            <select id="preset-select" class="input-field">
              <option value="">-- Custom Coordinates --</option>
              ${this.presets.map((p, i) => `<option value="${i}">${p.name}</option>`).join('')}
            </select>
          </div>

          <div class="coords-row">
            <div>
              <label class="label-mini">Origin Lat</label>
              <input type="number" step="0.0001" id="plan-lat1" class="input-field" value="6.5244" />
            </div>
            <div>
              <label class="label-mini">Origin Lon</label>
              <input type="number" step="0.0001" id="plan-lon1" class="input-field" value="3.3792" />
            </div>
          </div>

          <div class="coords-row">
            <div>
              <label class="label-mini">Destination Lat</label>
              <input type="number" step="0.0001" id="plan-lat2" class="input-field" value="6.6018" />
            </div>
            <div>
              <label class="label-mini">Destination Lon</label>
              <input type="number" step="0.0001" id="plan-lon2" class="input-field" value="3.3515" />
            </div>
          </div>

          <div class="coords-row" style="margin-top: 10px;">
            <div>
              <label class="label-mini">Stride Profile</label>
              <select id="plan-stride" class="input-field">
                <option value="0.75" selected>Standard Walk (0.75m)</option>
                <option value="0.65">Petite Pace (0.65m)</option>
                <option value="1.05">Jogging / Running (1.05m)</option>
                <option value="1.30">Long Stride Sprint (1.30m)</option>
              </select>
            </div>
            <div>
              <label class="label-mini">Target Step Limit</label>
              <input type="number" id="plan-limit" class="input-field" value="10000" />
            </div>
          </div>

          <button class="btn btn-primary" id="btn-calculate-route" style="width: 100%; margin-top: 16px;">
            Calculate Trip Metrics
          </button>
        </div>

        <div class="glass-panel" style="padding: 24px; display: flex; flex-direction: column; justify-content: space-between;">
          <div>
            <div class="group-title">
              <span>🚶</span> Trip Route Verdict
            </div>

            <div class="hud-result-box" id="route-result-box" style="margin-top: 16px;">
              <div class="result-row">
                <span class="stat-label">Total Distance</span>
                <span class="result-val" id="plan-res-dist">--</span>
              </div>
              <div class="result-row">
                <span class="stat-label">Estimated Walking Steps</span>
                <span class="result-val" id="plan-res-steps" style="color:var(--accent-cyan)">--</span>
              </div>
              <div class="result-row">
                <span class="stat-label">Heading Direction</span>
                <span class="result-val" id="plan-res-bearing" style="color:var(--accent-amber)">--</span>
              </div>
              <div class="result-row">
                <span class="stat-label">Proximity Zone</span>
                <span class="badge-zone" id="plan-res-zone">--</span>
              </div>
            </div>

            <!-- Visual Progress Bar for Threshold -->
            <div style="margin-top: 24px;">
              <div style="display: flex; justify-content: space-between; font-family: var(--font-mono); font-size: 0.78rem; margin-bottom: 6px;">
                <span style="color: var(--text-muted)">Threshold Consumption</span>
                <span id="plan-pct-text" style="color: var(--accent-cyan)">0%</span>
              </div>
              <div style="width: 100%; height: 12px; background: rgba(0,0,0,0.4); border-radius: 6px; overflow: hidden; border: 1px solid var(--border-subtle);">
                <div id="plan-progress-bar" style="height: 100%; width: 0%; background: linear-gradient(90deg, var(--accent-cyan), var(--accent-emerald)); transition: width 0.4s ease;"></div>
              </div>
            </div>
          </div>

          <div id="plan-verdict-banner" style="padding: 12px; border-radius: 8px; font-family: var(--font-mono); font-size: 0.85rem; text-align: center; background: rgba(255,255,255,0.03); border: 1px solid var(--border-subtle);">
            Ready to evaluate path.
          </div>
        </div>
      </div>
    `;

    document.getElementById('preset-select').addEventListener('change', (e) => {
      const idx = e.target.value;
      if (idx !== '') {
        const p = this.presets[idx];
        document.getElementById('plan-lat1').value = p.lat1;
        document.getElementById('plan-lon1').value = p.lon1;
        document.getElementById('plan-lat2').value = p.lat2;
        document.getElementById('plan-lon2').value = p.lon2;
        this.calculate();
      }
    });

    document.getElementById('btn-calculate-route').addEventListener('click', () => this.calculate());
  }

  async calculate() {
    const lat1 = parseFloat(document.getElementById('plan-lat1').value);
    const lon1 = parseFloat(document.getElementById('plan-lon1').value);
    const lat2 = parseFloat(document.getElementById('plan-lat2').value);
    const lon2 = parseFloat(document.getElementById('plan-lon2').value);
    const stride = parseFloat(document.getElementById('plan-stride').value);
    const limit = parseInt(document.getElementById('plan-limit').value, 10);

    try {
      const res = await api.calculateDistance(
        { lat1, lon1, lat2, lon2 },
        { unit: 'km', step_threshold: limit, stride_m: stride }
      );

      document.getElementById('plan-res-dist').textContent = `${res.distance} km (${res.distance_meters.toLocaleString()} m)`;
      document.getElementById('plan-res-steps').textContent = `${res.estimated_steps.toLocaleString()} steps`;
      document.getElementById('plan-res-bearing').textContent = `${res.bearing_degrees}° (${res.direction})`;

      const zone = document.getElementById('plan-res-zone');
      zone.textContent = res.proximity_zone;
      zone.className = `badge-zone ${res.proximity_zone}`;

      const pct = Math.min(Math.round((res.estimated_steps / limit) * 100), 100);
      document.getElementById('plan-pct-text').textContent = `${pct}% of limit`;
      const bar = document.getElementById('plan-progress-bar');
      bar.style.width = `${pct}%`;

      const banner = document.getElementById('plan-verdict-banner');
      if (res.within_threshold) {
        bar.style.background = 'linear-gradient(90deg, var(--accent-cyan), var(--accent-emerald))';
        banner.style.background = 'rgba(0, 255, 136, 0.1)';
        banner.style.borderColor = 'rgba(0, 255, 136, 0.3)';
        banner.style.color = 'var(--accent-emerald)';
        banner.textContent = `✓ Walkable: Within your target limit of ${limit.toLocaleString()} steps!`;
      } else {
        bar.style.background = 'linear-gradient(90deg, var(--accent-amber), var(--accent-rose))';
        banner.style.background = 'rgba(255, 51, 102, 0.1)';
        banner.style.borderColor = 'rgba(255, 51, 102, 0.3)';
        banner.style.color = 'var(--accent-rose)';
        banner.textContent = `⚠ Exceeds limit by ${(res.estimated_steps - limit).toLocaleString()} steps. Consider transit.`;
      }
    } catch (err) {
      alert(`Route calculation error: ${err.message}`);
    }
  }
}
