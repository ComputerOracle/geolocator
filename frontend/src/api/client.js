/**
 * Geolocator API Client & Real-time SSE Manager
 */
export class GeoClient {
  constructor(baseUrl = '') {
    // If running in development Vite (port 5173), default to localhost:8080 backend
    if (!baseUrl && window.location.port !== '8080') {
      this.baseUrl = 'http://127.0.0.1:8080';
    } else {
      this.baseUrl = baseUrl;
    }
    this.eventSource = null;
  }

  async getHealth() {
    const res = await fetch(`${this.baseUrl}/health`);
    if (!res.ok) throw new Error(`Health check failed: ${res.statusText}`);
    return res.json();
  }

  async getMetrics() {
    const res = await fetch(`${this.baseUrl}/metrics`);
    if (!res.ok) throw new Error(`Metrics failed: ${res.statusText}`);
    return res.json();
  }

  async calculateDistance(coords, { unit = 'm', step_threshold = 10000, stride_m = 0.75 } = {}) {
    const url = new URL(`${this.baseUrl}/distance`);
    if (unit) url.searchParams.set('unit', unit);
    if (step_threshold) url.searchParams.set('step_threshold', step_threshold);
    if (stride_m) url.searchParams.set('stride_m', stride_m);

    const res = await fetch(url.toString(), {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(coords),
    });

    const data = await res.json();
    if (!res.ok) throw new Error(data.details || data.error || 'Distance query failed');
    return data;
  }

  async checkProximity(payload, { unit = 'm', step_threshold = 10000, stride_m = 0.75 } = {}) {
    const url = new URL(`${this.baseUrl}/proximity`);
    if (unit) url.searchParams.set('unit', unit);
    if (step_threshold) url.searchParams.set('step_threshold', step_threshold);
    if (stride_m) url.searchParams.set('stride_m', stride_m);

    const res = await fetch(url.toString(), {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(payload),
    });

    const data = await res.json();
    if (!res.ok) throw new Error(data.details || data.error || 'Proximity query failed');
    return data;
  }

  async checkGeofence(payload) {
    const res = await fetch(`${this.baseUrl}/geofence`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(payload),
    });

    const data = await res.json();
    if (!res.ok) throw new Error(data.details || data.error || 'Geofence evaluation failed');
    return data;
  }

  async getScan() {
    const res = await fetch(`${this.baseUrl}/scan`);
    if (!res.ok) throw new Error(`Scan failed: ${res.statusText}`);
    return res.json();
  }

  async getDeviceHistory(deviceId) {
    const res = await fetch(`${this.baseUrl}/history/${encodeURIComponent(deviceId)}`);
    const data = await res.json();
    if (!res.ok) throw new Error(data.details || data.error || 'History query failed');
    return data;
  }

  subscribeEvents(onMessage, onError) {
    if (this.eventSource) {
      this.eventSource.close();
    }

    try {
      this.eventSource = new EventSource(`${this.baseUrl}/events`);
      this.eventSource.onmessage = (event) => {
        try {
          const parsed = JSON.parse(event.data);
          if (onMessage) onMessage(parsed);
        } catch (e) {
          console.error('Failed to parse SSE payload', e);
        }
      };
      this.eventSource.onerror = (err) => {
        if (onError) onError(err);
      };
    } catch (e) {
      console.warn('SSE stream not available, falling back to polling', e);
    }
  }

  disconnectEvents() {
    if (this.eventSource) {
      this.eventSource.close();
      this.eventSource = null;
    }
  }
}

export const api = new GeoClient();
