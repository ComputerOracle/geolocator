import './styles/theme.css';
import { api } from './api/client.js';

// Global state
export const state = {
  activeTab: 'tab-map',
  activeDevices: [],
  metrics: null,
  isOnline: false,
};

// Initialize Header Telemetry polling
async function refreshTelemetry() {
  try {
    const health = await api.getHealth();
    state.isOnline = true;
    state.metrics = health;

    document.getElementById('hud-active-devices').textContent = health.currently_tracked_devices;
    document.getElementById('hud-total-queries').textContent = health.total_api_requests;

    const statusBadge = document.getElementById('hud-server-status');
    const statusText = document.getElementById('hud-status-text');
    statusText.textContent = 'ONLINE';
    statusBadge.style.color = 'var(--accent-emerald)';
    statusBadge.style.borderColor = 'rgba(0, 255, 136, 0.3)';
  } catch (err) {
    state.isOnline = false;
    const statusBadge = document.getElementById('hud-server-status');
    const statusText = document.getElementById('hud-status-text');
    statusText.textContent = 'STANDBY';
    statusBadge.style.color = 'var(--accent-amber)';
    statusBadge.style.borderColor = 'rgba(255, 170, 0, 0.3)';
  }
}

// Tab Switching Controller
function setupTabs() {
  const tabs = document.querySelectorAll('.nav-tab');
  const sections = document.querySelectorAll('.view-section');

  tabs.forEach((tab) => {
    tab.addEventListener('click', () => {
      tabs.forEach((t) => t.classList.remove('active'));
      sections.forEach((s) => s.classList.remove('active'));

      tab.classList.add('active');
      const targetId = tab.getAttribute('data-tab');
      const targetSection = document.getElementById(targetId);
      if (targetSection) {
        targetSection.classList.add('active');
        state.activeTab = targetId;
        window.dispatchEvent(new CustomEvent('tab-changed', { detail: targetId }));
      }
    });
  });
}

import { MapVisualizer } from './components/MapVisualizer.js';
import { RadarScanner } from './components/RadarScanner.js';

// Lifecycle boot
document.addEventListener('DOMContentLoaded', () => {
  setupTabs();
  refreshTelemetry();
  setInterval(refreshTelemetry, 5000);

  // Initialize Map Component
  const mapContainer = document.getElementById('tab-map');
  if (mapContainer) {
    new MapVisualizer(mapContainer);
  }

  // Initialize Radar Component
  const radarContainer = document.getElementById('tab-radar');
  if (radarContainer) {
    new RadarScanner(radarContainer);
  }
});
