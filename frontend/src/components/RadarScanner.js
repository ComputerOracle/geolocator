import '../styles/radar.css';
import { api } from '../api/client.js';

export class RadarScanner {
  constructor(containerEl) {
    this.container = containerEl;
    this.canvas = null;
    this.ctx = null;
    this.sweepAngle = 0;
    this.devices = new Map(); // id -> device data
    this.animationId = null;

    this.render();
    this.initCanvas();
    this.startRadarAnimation();
    this.bindDataStreams();
  }

  render() {
    this.container.innerHTML = `
      <div class="radar-layout">
        <div class="radar-scope-card glass-panel">
          <canvas id="sonar-canvas" width="500" height="500"></canvas>
          <div class="radar-legend">
            <span>INNER: 1-5m</span>
            <span>MID: 10-25m</span>
            <span>OUTER: 50m+</span>
          </div>
        </div>

        <div class="target-list-panel glass-panel">
          <div style="display: flex; justify-content: space-between; align-items: center; margin-bottom: 8px;">
            <div class="group-title" style="margin: 0;">
              <span>📡</span> Nearby Discovered Targets
            </div>
            <button class="btn btn-secondary" id="btn-refresh-scan" style="padding: 6px 12px; font-size: 0.75rem;">
              Force Scan
            </button>
          </div>

          <div id="radar-device-list" style="display: flex; flex-direction: column; gap: 10px;">
            <div style="text-align: center; color: var(--text-muted); font-size: 0.88rem; padding: 40px 0;">
              Listening for ambient BLE advertisements...
            </div>
          </div>
        </div>
      </div>

      <div id="history-modal-container"></div>
    `;

    document.getElementById('btn-refresh-scan').addEventListener('click', () => this.fetchScanData());
  }

  initCanvas() {
    this.canvas = document.getElementById('sonar-canvas');
    this.ctx = this.canvas.getContext('2d');
  }

  startRadarAnimation() {
    const draw = () => {
      this.drawRadarScope();
      this.sweepAngle = (this.sweepAngle + 0.02) % (Math.PI * 2);
      this.animationId = requestAnimationFrame(draw);
    };
    draw();
  }

  drawRadarScope() {
    if (!this.ctx) return;
    const ctx = this.ctx;
    const w = this.canvas.width;
    const h = this.canvas.height;
    const cx = w / 2;
    const cy = h / 2;
    const radius = w / 2 - 15;

    ctx.clearRect(0, 0, w, h);

    // Draw Concentric Range Rings
    ctx.strokeStyle = 'rgba(0, 240, 255, 0.15)';
    ctx.lineWidth = 1;

    for (let r of [0.25, 0.5, 0.75, 1.0]) {
      ctx.beginPath();
      ctx.arc(cx, cy, radius * r, 0, Math.PI * 2);
      ctx.stroke();
    }

    // Crosshairs
    ctx.beginPath();
    ctx.moveTo(cx, cy - radius);
    ctx.lineTo(cx, cy + radius);
    ctx.moveTo(cx - radius, cy);
    ctx.lineTo(cx + radius, cy);
    ctx.stroke();

    // Radar Sweep Cone
    const sweepGradient = ctx.createConicGradient(this.sweepAngle, cx, cy);
    sweepGradient.addColorStop(0, 'rgba(0, 255, 136, 0.35)');
    sweepGradient.addColorStop(0.12, 'rgba(0, 240, 255, 0.08)');
    sweepGradient.addColorStop(0.25, 'rgba(0, 240, 255, 0)');
    sweepGradient.addColorStop(1, 'rgba(0, 240, 255, 0)');

    ctx.fillStyle = sweepGradient;
    ctx.beginPath();
    ctx.arc(cx, cy, radius, 0, Math.PI * 2);
    ctx.fill();

    // Leading Sweep Needle
    ctx.strokeStyle = '#00ff88';
    ctx.lineWidth = 1.5;
    ctx.beginPath();
    ctx.moveTo(cx, cy);
    ctx.lineTo(cx + Math.cos(this.sweepAngle) * radius, cy + Math.sin(this.sweepAngle) * radius);
    ctx.stroke();

    // Plot Devices
    this.devices.forEach((dev) => {
      // Assign stable pseudo-random angle based on device ID string
      const angle = this.hashIdToAngle(dev.id);
      // Map estimated distance (0 to 50m) to radius
      const normalizedDist = Math.min(Math.max((dev.estimated_distance_meters || 5) / 50, 0.1), 0.95);
      const r = normalizedDist * radius;

      const x = cx + Math.cos(angle) * r;
      const y = cy + Math.sin(angle) * r;

      // Draw blip
      ctx.shadowBlur = 10;
      ctx.shadowColor = dev.trend === 'approaching' ? '#00ff88' : '#00f0ff';
      ctx.fillStyle = dev.trend === 'approaching' ? '#00ff88' : '#00f0ff';
      ctx.beginPath();
      ctx.arc(x, y, 5, 0, Math.PI * 2);
      ctx.fill();
      ctx.shadowBlur = 0;

      // Device label
      ctx.fillStyle = '#8b949e';
      ctx.font = '10px "JetBrains Mono", monospace';
      ctx.fillText(dev.name.slice(0, 12), x + 8, y + 4);
    });
  }

  hashIdToAngle(str) {
    let hash = 0;
    for (let i = 0; i < str.length; i++) {
      hash = (hash << 5) - hash + str.charCodeAt(i);
      hash |= 0;
    }
    return Math.abs(hash % 360) * (Math.PI / 180);
  }

  bindDataStreams() {
    // Initial fetch
    this.fetchScanData();
    setInterval(() => this.fetchScanData(), 4000);

    // Live SSE broadcast listener
    api.subscribeEvents((event) => {
      this.devices.set(event.id, {
        id: event.id,
        name: event.name,
        rssi: event.rssi,
        estimated_distance_meters: event.estimated_distance_meters,
        trend: 'approaching',
      });
      this.updateDeviceListUI();
    });
  }

  async fetchScanData() {
    try {
      const results = await api.getScan();
      results.forEach((d) => this.devices.set(d.id, d));
      this.updateDeviceListUI();
    } catch (e) {
      // Backend scanner may be quiet if no real BLE devices nearby
    }
  }

  updateDeviceListUI() {
    const listEl = document.getElementById('radar-device-list');
    if (!listEl) return;

    if (this.devices.size === 0) {
      listEl.innerHTML = `
        <div style="text-align: center; color: var(--text-muted); font-size: 0.88rem; padding: 40px 0;">
          No BLE devices heard recently. Scanning active...
        </div>
      `;
      return;
    }

    listEl.innerHTML = Array.from(this.devices.values())
      .map(
        (dev) => `
        <div class="device-card" data-devid="${dev.id}">
          <div class="dev-info-left">
            <span class="dev-name">
              <span>📱</span> ${dev.name}
            </span>
            <span class="dev-id-tag">${dev.id}</span>
          </div>

          <div class="dev-metrics-right">
            <span class="rssi-pill">${dev.rssi} dBm (~${dev.estimated_distance_meters}m)</span>
            <span class="trend-pill trend-${dev.trend || 'stationary'}">${dev.trend || 'active'}</span>
          </div>
        </div>
      `
      )
      .join('');

    // Attach click listeners for history inspection
    listEl.querySelectorAll('.device-card').forEach((card) => {
      card.addEventListener('click', () => {
        const devId = card.getAttribute('data-devid');
        this.openHistoryModal(devId);
      });
    });
  }

  async openHistoryModal(deviceId) {
    const modalContainer = document.getElementById('history-modal-container');
    try {
      const history = await api.getDeviceHistory(deviceId);
      modalContainer.innerHTML = `
        <div class="history-modal-overlay">
          <div class="history-card glass-panel">
            <div style="display: flex; justify-content: space-between; align-items: center; margin-bottom: 16px;">
              <h3 style="color: var(--accent-cyan); font-size: 1.1rem;">${history.name} Telemetry</h3>
              <button class="btn btn-secondary" id="btn-close-modal" style="padding: 4px 8px;">✕</button>
            </div>

            <div style="font-family: var(--font-mono); font-size: 0.82rem; margin-bottom: 12px; color: var(--text-muted);">
              Current RSSI: <strong style="color:var(--accent-emerald)">${history.current_rssi} dBm</strong> (Smoothed: ${history.smoothed_rssi} dBm) • Trend: <strong>${history.trend}</strong>
            </div>

            <div style="max-height: 220px; overflow-y: auto; background: rgba(0,0,0,0.4); padding: 12px; border-radius: 8px;">
              <table style="width: 100%; font-family: var(--font-mono); font-size: 0.75rem; text-align: left;">
                <thead>
                  <tr style="color: var(--text-dim); border-bottom: 1px solid var(--border-subtle);">
                    <th>Time</th>
                    <th>Raw RSSI</th>
                    <th>Distance</th>
                  </tr>
                </thead>
                <tbody>
                  ${history.history
                    .slice(-10)
                    .reverse()
                    .map(
                      (h) => `
                    <tr>
                      <td style="color: var(--text-muted);">${new Date(h.timestamp_ms).toLocaleTimeString()}</td>
                      <td style="color: var(--accent-cyan);">${h.rssi} dBm</td>
                      <td style="color: var(--accent-emerald);">${h.distance_meters.toFixed(2)}m</td>
                    </tr>
                  `
                    )
                    .join('')}
                </tbody>
              </table>
            </div>
          </div>
        </div>
      `;

      document.getElementById('btn-close-modal').addEventListener('click', () => {
        modalContainer.innerHTML = '';
      });
    } catch (e) {
      alert(`Could not load history for ${deviceId}`);
    }
  }
}
