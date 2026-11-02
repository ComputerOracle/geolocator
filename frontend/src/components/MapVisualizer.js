import L from 'leaflet';
import '../styles/map.css';
import { api } from '../api/client.js';

export class MapVisualizer {
  constructor(containerEl) {
    this.container = containerEl;
    this.map = null;
    this.markerA = null;
    this.markerB = null;
    this.polyline = null;
    this.unit = 'm';

    // Default coordinates (Lagos coordinates from README / testing)
    this.pointA = [6.5244, 3.3792];
    this.pointB = [6.6018, 3.3515];

    this.render();
    this.initMap();
    this.calculate();
  }

  render() {
    this.container.innerHTML = `
      <div class="map-container-grid">
        <div class="map-card glass-panel">
          <div id="leaflet-map"></div>

          <!-- Bearing Compass HUD -->
          <div class="compass-hud">
            <div class="compass-dial">
              <div class="compass-needle" id="compass-needle"></div>
              <div class="compass-heading-text" id="compass-heading">0° N</div>
            </div>
          </div>
        </div>

        <div class="map-sidebar">
          <div class="glass-panel control-group">
            <div class="group-title">
              <span>📍</span> Geolocation Waypoints
            </div>

            <span class="label-mini">Point A (Origin)</span>
            <div class="coords-row">
              <input type="number" step="0.0001" id="input-lat1" class="input-field" value="${this.pointA[0]}" />
              <input type="number" step="0.0001" id="input-lon1" class="input-field" value="${this.pointA[1]}" />
            </div>

            <span class="label-mini">Point B (Target)</span>
            <div class="coords-row">
              <input type="number" step="0.0001" id="input-lat2" class="input-field" value="${this.pointB[0]}" />
              <input type="number" step="0.0001" id="input-lon2" class="input-field" value="${this.pointB[1]}" />
            </div>

            <div style="display: flex; gap: 8px; margin-bottom: 12px;">
              <select id="select-unit" class="input-field" style="width: auto;">
                <option value="m" selected>Meters</option>
                <option value="km">Kilometers</option>
                <option value="mi">Miles</option>
                <option value="ft">Feet</option>
              </select>
              <button class="btn btn-primary" id="btn-calc" style="flex: 1;">Update Path</button>
            </div>
          </div>

          <div class="glass-panel control-group">
            <div class="group-title">
              <span>🧭</span> Navigation Telemetry
            </div>

            <div class="hud-result-box">
              <div class="result-row">
                <span class="stat-label">Calculated Distance</span>
                <span class="result-val" id="res-distance">--</span>
              </div>
              <div class="result-row">
                <span class="stat-label">Estimated Steps</span>
                <span class="result-val" id="res-steps" style="color: var(--accent-cyan)">--</span>
              </div>
              <div class="result-row">
                <span class="stat-label">Bearing</span>
                <span class="result-val" id="res-bearing" style="color: var(--accent-amber)">--</span>
              </div>
              <div class="result-row">
                <span class="stat-label">Proximity Zone</span>
                <span class="badge-zone" id="res-zone">--</span>
              </div>
              <div class="result-row">
                <span class="stat-label">Threshold Check</span>
                <span id="res-threshold" style="font-family: var(--font-mono); font-size: 0.85rem;">--</span>
              </div>
            </div>
          </div>
        </div>
      </div>
    `;

    document.getElementById('btn-calc').addEventListener('click', () => this.handleInputUpdate());
    document.getElementById('select-unit').addEventListener('change', (e) => {
      this.unit = e.target.value;
      this.calculate();
    });
  }

  initMap() {
    this.map = L.map('leaflet-map', {
      zoomControl: true,
      attributionControl: false,
    }).setView(this.pointA, 12);

    L.tileLayer('https://{s}.basemaps.cartocdn.com/dark_all/{z}/{x}/{y}{r}.png', {
      maxZoom: 19,
      subdomains: 'abcd',
    }).addTo(this.map);

    const iconA = L.divIcon({
      className: 'custom-pin-a',
      html: `<div style="background:#00f0ff;width:16px;height:16px;border-radius:50%;box-shadow:0 0 10px #00f0ff;border:2px solid #fff;"></div>`,
      iconSize: [16, 16],
    });

    const iconB = L.divIcon({
      className: 'custom-pin-b',
      html: `<div style="background:#ff3366;width:16px;height:16px;border-radius:50%;box-shadow:0 0 10px #ff3366;border:2px solid #fff;"></div>`,
      iconSize: [16, 16],
    });

    this.markerA = L.marker(this.pointA, { draggable: true, icon: iconA }).addTo(this.map);
    this.markerB = L.marker(this.pointB, { draggable: true, icon: iconB }).addTo(this.map);

    this.markerA.on('drag', () => {
      const pos = this.markerA.getLatLng();
      this.pointA = [pos.lat, pos.lng];
      document.getElementById('input-lat1').value = pos.lat.toFixed(5);
      document.getElementById('input-lon1').value = pos.lng.toFixed(5);
      this.updatePolyline();
    });

    this.markerA.on('dragend', () => this.calculate());

    this.markerB.on('drag', () => {
      const pos = this.markerB.getLatLng();
      this.pointB = [pos.lat, pos.lng];
      document.getElementById('input-lat2').value = pos.lat.toFixed(5);
      document.getElementById('input-lon2').value = pos.lng.toFixed(5);
      this.updatePolyline();
    });

    this.markerB.on('dragend', () => this.calculate());

    this.updatePolyline();

    window.addEventListener('tab-changed', (e) => {
      if (e.detail === 'tab-map' && this.map) {
        setTimeout(() => this.map.invalidateSize(), 150);
      }
    });
  }

  updatePolyline() {
    if (this.polyline) {
      this.map.removeLayer(this.polyline);
    }
    this.polyline = L.polyline([this.pointA, this.pointB], {
      color: '#00f0ff',
      weight: 3,
      opacity: 0.8,
      dashArray: '8, 8',
    }).addTo(this.map);
  }

  handleInputUpdate() {
    const lat1 = parseFloat(document.getElementById('input-lat1').value);
    const lon1 = parseFloat(document.getElementById('input-lon1').value);
    const lat2 = parseFloat(document.getElementById('input-lat2').value);
    const lon2 = parseFloat(document.getElementById('input-lon2').value);

    this.pointA = [lat1, lon1];
    this.pointB = [lat2, lon2];

    this.markerA.setLatLng(this.pointA);
    this.markerB.setLatLng(this.pointB);
    this.map.fitBounds([this.pointA, this.pointB], { padding: [50, 50] });
    this.updatePolyline();
    this.calculate();
  }

  async calculate() {
    try {
      const result = await api.calculateDistance(
        {
          lat1: this.pointA[0],
          lon1: this.pointA[1],
          lat2: this.pointB[0],
          lon2: this.pointB[1],
        },
        { unit: this.unit }
      );

      document.getElementById('res-distance').textContent = `${result.distance} ${result.unit}`;
      document.getElementById('res-steps').textContent = `${result.estimated_steps.toLocaleString()} steps`;
      document.getElementById('res-bearing').textContent = `${result.bearing_degrees}° (${result.direction})`;

      const zoneEl = document.getElementById('res-zone');
      zoneEl.textContent = result.proximity_zone;
      zoneEl.className = `badge-zone ${result.proximity_zone}`;

      const thresholdEl = document.getElementById('res-threshold');
      if (result.within_threshold) {
        thresholdEl.innerHTML = `<span style="color:var(--accent-emerald)">✓ Within Threshold</span>`;
      } else {
        thresholdEl.innerHTML = `<span style="color:var(--accent-rose)">⚠ Exceeds Threshold</span>`;
      }

      // Rotate compass needle
      const needle = document.getElementById('compass-needle');
      const headingText = document.getElementById('compass-heading');
      needle.style.transform = `rotate(${result.bearing_degrees}deg)`;
      headingText.textContent = `${result.bearing_degrees}° ${result.direction}`;
    } catch (err) {
      console.error('Map distance calculation error', err);
    }
  }
}
