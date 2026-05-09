// ── Pelendur HUD — Main Controller ──────────────────────────────

document.addEventListener('DOMContentLoaded', async () => {
  const container = document.getElementById('hud-container');
  const mainSuggestion = document.getElementById('main-suggestion');
  const partialDiv = document.getElementById('partial-transcription');
  const feed = document.getElementById('transcription-feed');
  const statusDot = document.getElementById('status-indicator');
  const audioBtn = document.getElementById('audio-source-btn');
  const interviewBtn = document.getElementById('interview-btn');
  const minimalBtn = document.getElementById('minimal-btn');
  const lockBtn = document.getElementById('lock-btn');
  const clearBtn = document.getElementById('clear-btn');
  const regenerateBtn = document.getElementById('regenerate-btn');
  const closeBtn = document.getElementById('close-btn');
  const profileBtn = document.getElementById('profile-btn');
  const minimalIcon = document.getElementById('minimal-icon');
  const modalOverlay = document.getElementById('modal-overlay');
  const processModal = document.getElementById('process-modal');
  const closeModalBtn = document.getElementById('close-modal-btn');
  const processList = document.getElementById('process-list');
  const companyModal = document.getElementById('company-modal');
  const companyModalOverlay = document.getElementById('company-modal-overlay');
  const companyList = document.getElementById('company-list');
  const customCompanyInput = document.getElementById('custom-company-input');
  const startCustomBtn = document.getElementById('start-with-custom-btn');
  const closeCompanyModalBtn = document.getElementById('close-company-modal-btn');
  const summaryModal = document.getElementById('summary-modal');
  const summaryModalOverlay = document.getElementById('summary-modal-overlay');
  const summaryContent = document.getElementById('summary-content');
  const closeSummaryModalBtn = document.getElementById('close-summary-modal-btn');
  const interviewStatusBar = document.getElementById('interview-status-bar');
  const interviewCompanyDisplay = document.getElementById('interview-company-display');
  const interviewTimer = document.getElementById('interview-timer');
  const knowledgePanel = document.getElementById('knowledge-panel');
  const knowledgeContextBar = document.getElementById('knowledge-context-bar');
  const dashboard = document.getElementById('readiness-dashboard');

  let meetingMode = false;
  let interviewActive = false;
  let interviewStartTime = null;
  let streamBuffer = '';
  let timerInterval = null;

  // ── Readiness Dashboard ────────────────────────────────────────

  async function updateReadiness() {
    try {
      const status = await window.__TAURI_INTERNALS__.invoke('get_readiness');
      const components = ['stt', 'llm', 'kg', 'audio'];
      
      components.forEach(c => {
        const dot = document.querySelector(`.pl-dashboard__dot[data-component="${c}"]`);
        const detail = document.getElementById(`detail-${c}`);
        if (!dot) return;
        
        const stateMapping = {
          'ready': 'ready', 'connected': 'ready', 'capturing': 'active',
          'warming': 'loading', 'local': 'warning', 'offline': 'error',
          'error': 'error', 'idle': 'idle'
        };
        const state = stateMapping[status[c]] || 'loading';
        
        dot.className = 'pl-dashboard__dot';
        dot.classList.add(`pl-dashboard__dot--${state}`);
        
        if (detail) {
          if (c === 'stt') detail.textContent = status.stt_model || 'whisper';
          else if (c === 'llm') detail.textContent = status.llm_model || 'llm';
          else if (c === 'kg') detail.textContent = status.kg === 'ready' ? 'ready' : '—';
          else if (c === 'audio') detail.textContent = status.audio === 'capturing' ? '🎤 live' : 'idle';
        }
      });

      // Overall status badge
      const badge = document.getElementById('dashboard-badge');
      if (badge) {
        const overall = status.overall || 'limited';
        badge.className = `pl-badge--${overall}`;
        badge.textContent = overall === 'ready' ? '✅ Ready for meeting' :
                            overall === 'limited' ? '⚠️ Limited (no audio)' :
                            '❌ Needs attention';
      }

      // Metrics
      const metrics = document.getElementById('dashboard-metrics');
      if (metrics) {
        const count = status.transcription_count || 0;
        const lat = status.latency_ms || 0;
        const uptime = status.uptime_seconds || 0;
        const mins = Math.floor(uptime / 60);
        const secs = uptime % 60;
        metrics.textContent = `${count} transcribed · ${lat}ms latency · ${mins}m${secs}s uptime`;
      }
    } catch (e) {
      console.warn('Readiness poll failed:', e);
    }
  }

  // Poll readiness every 3 seconds
  updateReadiness();
  setInterval(updateReadiness, 3000);

  // ── Event Listeners ───────────────────────────────────────────

  try {
    const { listen } = await import('@tauri-apps/api/event');

    // Listen for system status updates
    await listen('system-status', (event) => {
      updateReadiness();
    });

    // Listen for partial transcription
    await listen('partial-transcription', (event) => {
      partialDiv.textContent = event.payload.text || '';
      partialDiv.className = event.payload.text ? 'pl-partial-visible' : 'pl-partial-hidden';
    });

    // Listen for transcription update
    await listen('transcription-update', (event) => {
      const text = event.payload.text || '';
      if (text) {
        const entry = document.createElement('div');
        entry.className = 'pl-feed-entry';
        entry.innerHTML = `<span class="pl-feed-time">${new Date().toLocaleTimeString()}</span><span class="pl-feed-text">${escapeHtml(text)}</span>`;
        feed.prepend(entry);
        // Keep only last 20 entries
        while (feed.children.length > 20) feed.removeChild(feed.lastChild);
      }
    });

    // Listen for suggestion streaming
    await listen('suggestion-stream', (event) => {
      const delta = event.payload.text || '';
      streamBuffer += delta;
      mainSuggestion.innerHTML = escapeHtml(streamBuffer) + ' <span class="pl-cursor">▌</span>';
      if (!container.classList.contains('pl-meeting-mode') && meetingMode) {
        container.classList.add('pl-meeting-mode');
      }
    });

    await listen('suggestion-update', (event) => {
      streamBuffer = event.payload.text || '';
      mainSuggestion.textContent = streamBuffer;
      container.classList.remove('pl-meeting-mode');
    });

    await listen('audio-level-update', (event) => {
      // Visual VU meter
      const rms = event.payload.rms || 0;
      const bars = Math.round(rms * 100);
    });

    await listen('lock-state-changed', (event) => {
      lockBtn.textContent = event.payload ? '🔒' : '🔓';
      lockBtn.classList.toggle('pl-btn--active', event.payload);
    });

    await listen('minimal-mode-changed', (event) => {
      container.classList.toggle('pl-minimal', event.payload);
      minimalIcon.classList.toggle('hidden', !event.payload);
    });

    await listen('interview-state-changed', (event) => {
      const data = event.payload;
      interviewActive = data.active;
      if (data.active) {
        interviewStatusBar.classList.remove('hidden');
        interviewCompanyDisplay.textContent = data.company || '—';
        interviewStartTime = new Date(data.started_at || Date.now());
        timerInterval = setInterval(updateInterviewTimer, 1000);
      } else {
        interviewStatusBar.classList.add('hidden');
        if (timerInterval) clearInterval(timerInterval);
      }
    });

    await listen('interview-summary', (event) => {
      showSummaryModal(event.payload);
    });

    await listen('suggestions-cleared', () => {
      feed.innerHTML = '';
      mainSuggestion.textContent = 'Esperando audio... Seleccionó el dispositivo.';
      streamBuffer = '';
    });

  } catch (e) {
    console.warn('Tauri event system not available:', e);
  }

  // ── UI Actions ────────────────────────────────────────────────

  audioBtn.addEventListener('click', async () => {
    try {
      const devices = await window.__TAURI_INTERNALS__.invoke('get_audio_devices');
      processList.innerHTML = '';
      
      // Add system audio option
      const sysItem = document.createElement('div');
      sysItem.className = 'pl-process-item clickable';
      sysItem.innerHTML = '<span>🔊 System Audio</span>';
      sysItem.addEventListener('click', () => selectSource(null, null, 'system'));
      processList.appendChild(sysItem);
      
      // Add microphone devices
      devices.forEach((d, i) => {
        const item = document.createElement('div');
        item.className = 'pl-process-item clickable';
        item.innerHTML = `<span>${d.label || '🎤 Microphone'}</span><span class="pl-process-detail">${d.name || ''}</span>`;
        item.addEventListener('click', () => selectSource(null, i, 'mic'));
        processList.appendChild(item);
      });
      
      // Add dual mode
      if (devices.length > 0) {
        const dual = document.createElement('div');
        dual.className = 'pl-process-item clickable';
        dual.innerHTML = '<span>🎙️🔊 System + Microphone</span><span class="pl-process-detail">Dual capture (mixed)</span>';
        dual.addEventListener('click', () => selectSource(null, 0, 'dual'));
        processList.appendChild(dual);
      }
      
      processModal.classList.remove('hidden');
      modalOverlay.classList.remove('hidden');
    } catch (err) {
      console.error('Failed to get audio devices:', err);
    }
  });

  async function selectSource(pid, deviceIndex, mode) {
    processModal.classList.add('hidden');
    modalOverlay.classList.add('hidden');
    try {
      const payload = mode ? { mode, pid, deviceIndex } : { pid, deviceIndex };
      await window.__TAURI_INTERNALS__.invoke('start_capture', payload);
      statusDot.style.backgroundColor = '#4ADE80';
      mainSuggestion.textContent = mode === 'dual' ? '🎙️🔊 System + Mic active' : '🎤 Listening...';
      meetingMode = true;
      container.classList.add('pl-meeting-mode');
      updateReadiness();
    } catch (err) {
      console.error('Capture start failed:', err);
      statusDot.style.backgroundColor = '#FB7185';
    }
  }

  closeModalBtn.addEventListener('click', () => {
    processModal.classList.add('hidden');
    modalOverlay.classList.add('hidden');
  });
  modalOverlay.addEventListener('click', () => {
    processModal.classList.add('hidden');
    modalOverlay.classList.add('hidden');
  });

  clearBtn.addEventListener('click', async () => {
    try {
      await window.__TAURI_INTERNALS__.invoke('clear_feed');
      feed.innerHTML = '';
      mainSuggestion.textContent = 'Cleared. Waiting for audio...';
      streamBuffer = '';
    } catch (e) {
      console.warn('Clear failed:', e);
    }
  });

  regenerateBtn.addEventListener('click', async () => {
    try {
      mainSuggestion.textContent = 'Regenerating...';
      await window.__TAURI_INTERNALS__.invoke('regenerate');
    } catch (e) {
      console.warn('Regenerate failed:', e);
    }
  });

  closeBtn.addEventListener('click', async () => {
    try { await window.__TAURI_INTERNALS__.invoke('close_app'); } catch (e) {}
    window.close();
  });

  lockBtn.addEventListener('click', async () => {
    const locked = lockBtn.textContent === '🔓';
    try {
      await window.__TAURI_INTERNALS__.invoke('set_lock_state', { locked });
    } catch (e) {
      console.warn('Lock failed:', e);
    }
  });

  minimalBtn.addEventListener('click', async () => {
    const minimal = !container.classList.contains('pl-minimal');
    try {
      await window.__TAURI_INTERNALS__.invoke('set_minimal_mode', { minimal });
    } catch (e) {}
  });

  minimalIcon.addEventListener('click', async () => {
    try {
      await window.__TAURI_INTERNALS__.invoke('set_minimal_mode', { minimal: false });
    } catch (e) {}
  });

  profileBtn.addEventListener('click', async () => {
    try {
      await window.__TAURI_INTERNALS__.invoke('open_profile_window');
    } catch (e) {
      console.warn('Profile window failed:', e);
    }
  });

  interviewBtn.addEventListener('click', () => {
    companyModal.classList.remove('hidden');
    companyModalOverlay.classList.remove('hidden');
  });

  closeCompanyModalBtn.addEventListener('click', () => {
    companyModal.classList.add('hidden');
    companyModalOverlay.classList.add('hidden');
  });
  companyModalOverlay.addEventListener('click', () => {
    companyModal.classList.add('hidden');
    companyModalOverlay.classList.add('hidden');
  });

  closeSummaryModalBtn.addEventListener('click', () => {
    summaryModal.classList.add('hidden');
    summaryModalOverlay.classList.add('hidden');
  });
  summaryModalOverlay.addEventListener('click', () => {
    summaryModal.classList.add('hidden');
    summaryModalOverlay.classList.add('hidden');
  });

  function updateInterviewTimer() {
    if (!interviewStartTime) return;
    const elapsed = Math.floor((Date.now() - interviewStartTime) / 1000);
    const m = String(Math.floor(elapsed / 60)).padStart(2, '0');
    const s = String(elapsed % 60).padStart(2, '0');
    interviewTimer.textContent = `${m}:${s}`;
  }

  function escapeHtml(text) {
    const d = document.createElement('div');
    d.textContent = text;
    return d.innerHTML;
  }

  function showSummaryModal(data) {
    summaryContent.innerHTML = '';
    if (typeof data === 'string') {
      summaryContent.textContent = data;
    } else if (data && data.summary) {
      summaryContent.textContent = data.summary;
    } else {
      summaryContent.textContent = JSON.stringify(data, null, 2);
    }
    summaryModal.classList.remove('hidden');
    summaryModalOverlay.classList.remove('hidden');
  }

  // Initial state
  mainSuggestion.textContent = 'Esperando audio... Seleccionó el dispositivo.';
});
