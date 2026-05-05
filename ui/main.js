// Pelendur HUD UI Logic
document.addEventListener('DOMContentLoaded', () => {
  const { invoke } = window.__TAURI__.core;
  const { listen } = window.__TAURI__.event;

  const mainSuggestion = document.getElementById('main-suggestion');
  const transcriptionFeed = document.getElementById('transcription-feed');
  const partialDiv = document.getElementById('partial-transcription');
  const audioSourceBtn = document.getElementById('audio-source-btn');
  const interviewBtn = document.getElementById('interview-btn');
  const lockBtn = document.getElementById('lock-btn');
  const clearBtn = document.getElementById('clear-btn');
  const regenerateBtn = document.getElementById('regenerate-btn');
  const statusIndicator = document.getElementById('status-indicator');
  const profileBtn = document.getElementById('profile-btn');
  const minimalBtn = document.getElementById('minimal-btn');
  const minimalIcon = document.getElementById('minimal-icon');

  const processModal = document.getElementById('process-modal');
  const processList = document.getElementById('process-list');
  const closeModalBtn = document.getElementById('close-modal-btn');
  const modalOverlay = document.getElementById('modal-overlay');

  // Interview elements
  const interviewStatusBar = document.getElementById('interview-status-bar');
  const interviewCompanyDisplay = document.getElementById('interview-company-display');
  const interviewTimer = document.getElementById('interview-timer');
  const companyModal = document.getElementById('company-modal');
  const companyModalOverlay = document.getElementById('company-modal-overlay');
  const companyList = document.getElementById('company-list');
  const customCompanyInput = document.getElementById('custom-company-input');
  const startWithCustomBtn = document.getElementById('start-with-custom-btn');
  const closeCompanyModalBtn = document.getElementById('close-company-modal-btn');
  const summaryModal = document.getElementById('summary-modal');
  const summaryModalOverlay = document.getElementById('summary-modal-overlay');
  const summaryContent = document.getElementById('summary-content');
  const closeSummaryModalBtn = document.getElementById('close-summary-modal-btn');

  const state = {
    knowledgeContext: [],
    relevantStories: [],
    conversationHistory: [],
    knowledgeCache: new Map(),
    knowledgeCacheTime: new Map(),
    interviewActive: false,
    interviewStartTime: null,
    interviewTickInterval: null,
  };

  let isLocked = false;
  let knowledgeDebounceTimer = null;
  const KNOWLEDGE_CACHE_TTL = 30000;
  const KNOWLEDGE_DEBOUNCE_MS = 2000;

  async function init() {
    console.log('--- Pelendur HUD Initializing ---');
    mainSuggestion.textContent = "Esperando audio... Seleccioná el dispositivo.";

    // Check if interview was already active (app restart)
    try {
      const interviewState = await invoke('get_interview_state');
      if (interviewState.active) {
        startInterviewUI(interviewState.company, interviewState.started_at);
      }
    } catch (e) {
      console.log('Could not check interview state:', e);
    }

    // Listen for knowledge graph context updates
    await listen('knowledge-context', (event) => {
      const data = event.payload || {};
      const ctxBar = document.getElementById('knowledge-context-bar');
      const ctxIndicator = document.getElementById('context-indicator');
      const skillTags = document.getElementById('skill-tags');
      if (!ctxBar || !ctxIndicator || !skillTags) return;

      const count = data.stories_count || 0;
      const skills = data.skills || [];
      if (count > 0) {
        ctxIndicator.textContent = `📚 ${count} ${count === 1 ? 'story' : 'stories'} found`;
        ctxBar.classList.remove('hidden');
      } else {
        ctxBar.classList.add('hidden');
      }

      if (skills.length > 0) {
        skillTags.innerHTML = skills.slice(0, 5).map(s =>
          `<span class="skill-tag">${s}</span>`
        ).join('');
        skillTags.style.display = 'flex';
      } else {
        skillTags.style.display = 'none';
      }
    });

    // Listen for transcriptions
    await listen('transcription-update', (event) => {
      console.log('JS Received Transcription:', event.payload.text);
      addTranscription(event.payload.text);
    });

    // Listen for suggestions
    await listen('suggestion-update', (event) => {
      console.log('JS Received Suggestion:', event.payload.text);
      updateSuggestion(event.payload.text);
    });

    let lastPartialText = '';

    await listen('partial-transcription', (event) => {
      if (!partialDiv) return;
      const text = event.payload?.text || event.payload || '';
      if (!text || text === lastPartialText) return;
      lastPartialText = text;
      partialDiv.textContent = text;
      partialDiv.classList.remove('partial-hidden');
      partialDiv.classList.add('partial-visible');
    });

    await listen('speech-start', () => {
      if (!partialDiv) return;
      lastPartialText = '';
      partialDiv.innerHTML = '<span class="listening-indicator">\u{1F399} Listening...</span>';
      partialDiv.classList.remove('partial-hidden');
      partialDiv.classList.add('partial-visible');
    });

    // Listen for global lock toggle
    await listen('lock-state-changed', (event) => {
      isLocked = event.payload;
      lockBtn.textContent = isLocked ? '🔒' : '🔓';
      document.body.classList.toggle('locked', isLocked);
    });

    // Listen for minimal mode changes from Rust
    await listen('minimal-mode-changed', (event) => {
      const hudContainer = document.getElementById('hud-container');
      const isMinimal = event.payload;
      hudContainer.classList.toggle('minimal', isMinimal);
      minimalBtn.classList.toggle('active', isMinimal);
    });

    // Listen for interview state changes
    await listen('interview-state-changed', (event) => {
      const data = event.payload;
      if (data.active) {
        startInterviewUI(data.company, data.started_at);
      } else {
        endInterviewUI();
      }
    });

    // Listen for interview summary
    await listen('interview-summary', (event) => {
      showSummaryModal(event.payload);
    });
  }

  // ─── Interview Mode ────────────────────────────────────────────────────

  function startInterviewUI(company, startedAt) {
    state.interviewActive = true;
    state.interviewStartTime = startedAt ? new Date(startedAt) : new Date();
    interviewBtn.textContent = '⏹️';
    interviewBtn.classList.add('active');
    interviewBtn.title = 'End Interview';
    interviewBtn.style.borderColor = '#ff4444';
    interviewCompanyDisplay.textContent = company || 'Unknown';
    interviewStatusBar.classList.remove('hidden');
    mainSuggestion.textContent = `🎙️ Interview Started at ${company}. Good luck!`;
    
    // Start the timer
    if (state.interviewTickInterval) clearInterval(state.interviewTickInterval);
    state.interviewTickInterval = setInterval(updateInterviewTimer, 1000);
    updateInterviewTimer();
  }

  function endInterviewUI() {
    state.interviewActive = false;
    state.interviewStartTime = null;
    interviewBtn.textContent = '🎙️';
    interviewBtn.classList.remove('active');
    interviewBtn.title = 'Start Interview Mode';
    interviewBtn.style.borderColor = '';
    interviewStatusBar.classList.add('hidden');
    if (state.interviewTickInterval) {
      clearInterval(state.interviewTickInterval);
      state.interviewTickInterval = null;
    }
  }

  function updateInterviewTimer() {
    if (!state.interviewStartTime) return;
    const elapsed = Math.floor((Date.now() - state.interviewStartTime.getTime()) / 1000);
    const mins = Math.floor(elapsed / 60);
    const secs = elapsed % 60;
    interviewTimer.textContent = `${String(mins).padStart(2, '0')}:${String(secs).padStart(2, '0')}`;
  }

  // Open company selection modal
  interviewBtn.addEventListener('click', async () => {
    if (state.interviewActive) {
      // End interview
      try {
        interviewBtn.textContent = '⏳';
        interviewBtn.disabled = true;
        const summary = await invoke('end_interview');
        console.log('Interview ended:', summary);
      } catch (err) {
        console.error('Failed to end interview:', err);
        interviewBtn.textContent = '🎙️';
        interviewBtn.disabled = false;
        mainSuggestion.textContent = `Error ending interview: ${err}`;
      }
    } else {
      // Show company selector
      companyModal.classList.remove('hidden');
      companyModalOverlay.classList.remove('hidden');
      companyList.innerHTML = '<div class="loading-text">Loading companies...</div>';
      try {
        const companies = await invoke('list_companies');
        companyList.innerHTML = '';
        if (companies.length === 0) {
          companyList.innerHTML = '<div class="company-empty">No companies found. Type a custom company name below.</div>';
        }
        companies.forEach(c => {
          const item = document.createElement('div');
          item.className = 'company-item';
          const industry = c.industry ? ` <span class="company-industry">${c.industry}</span>` : '';
          item.innerHTML = `<span class="company-name">${escapeHtml(c.name)}</span>${industry}`;
          item.onclick = () => startInterview(c.name);
          companyList.appendChild(item);
        });
      } catch (err) {
        companyList.innerHTML = `<div class="company-error">Error: ${err}</div>`;
      }
    }
  });

  async function startInterview(companyName) {
    companyModal.classList.add('hidden');
    companyModalOverlay.classList.add('hidden');
    try {
      await invoke('start_interview', { companyName: companyName });
    } catch (err) {
      console.error('Failed to start interview:', err);
      mainSuggestion.textContent = `Error starting interview: ${err}`;
    }
  }

  // Custom company input
  customCompanyInput.addEventListener('keydown', (e) => {
    if (e.key === 'Enter') {
      const name = customCompanyInput.value.trim();
      if (name) startInterview(name);
    }
  });
  startWithCustomBtn.addEventListener('click', () => {
    const name = customCompanyInput.value.trim();
    if (name) startInterview(name);
  });

  // Close company modal
  const closeCompanyModals = [closeCompanyModalBtn, companyModalOverlay];
  closeCompanyModals.forEach(el => {
    if (el) el.addEventListener('click', () => {
      companyModal.classList.add('hidden');
      companyModalOverlay.classList.add('hidden');
    });
  });

  // ─── Summary Modal ─────────────────────────────────────────────────────

  function showSummaryModal(summary) {
    document.getElementById('interview-btn').textContent = '🎙️';
    document.getElementById('interview-btn').disabled = false;
    summaryContent.innerHTML = buildSummaryHTML(summary);
    summaryModal.classList.remove('hidden');
    summaryModalOverlay.classList.remove('hidden');
  }

  function buildSummaryHTML(summary) {
    let html = `<div class="summary-header">
      <div class="summary-meta">
        <span class="summary-company">${escapeHtml(summary.company)}</span>
        <span class="summary-duration">${formatDuration(summary.duration_seconds)}</span>
        <span class="summary-questions">${summary.transcript_count} questions</span>
      </div>
    </div>`;

    // Summary text
    html += `<div class="summary-section">
      <div class="summary-text">${summary.summary_text.replace(/\n/g, '<br>')}</div>
    </div>`;

    // Strengths
    if (summary.strengths && summary.strengths.length > 0) {
      html += `<div class="summary-section">
        <div class="summary-section-title">✅ Key Strengths</div>
        <ul class="summary-list">
          ${summary.strengths.map(s => `<li>${escapeHtml(s)}</li>`).join('')}
        </ul>
      </div>`;
    }

    // Areas to improve
    if (summary.areas_to_improve && summary.areas_to_improve.length > 0) {
      html += `<div class="summary-section">
        <div class="summary-section-title">📈 Areas to Improve</div>
        <ul class="summary-list">
          ${summary.areas_to_improve.map(s => `<li>${escapeHtml(s)}</li>`).join('')}
        </ul>
      </div>`;
    }

    // Recommended stories
    if (summary.recommended_stories && summary.recommended_stories.length > 0) {
      html += `<div class="summary-section">
        <div class="summary-section-title">⭐ Recommended STAR Stories</div>
        <ul class="summary-list">
          ${summary.recommended_stories.map(s => `<li>${escapeHtml(s)}</li>`).join('')}
        </ul>
      </div>`;
    }

    return html;
  }

  function formatDuration(seconds) {
    const mins = Math.floor(seconds / 60);
    const secs = seconds % 60;
    return `${mins}m ${secs}s`;
  }

  // Close summary modal
  const closeSummaryElements = [closeSummaryModalBtn, summaryModalOverlay];
  closeSummaryElements.forEach(el => {
    if (el) el.addEventListener('click', () => {
      summaryModal.classList.add('hidden');
      summaryModalOverlay.classList.add('hidden');
    });
  });

  // ─── Core HUD Logic ────────────────────────────────────────────────────

  function addTranscription(text) {
    state.conversationHistory.push(text);
    if (state.conversationHistory.length > 20) {
      state.conversationHistory.shift();
    }
    
    if (partialDiv) {
      partialDiv.classList.remove('partial-visible');
      partialDiv.classList.add('partial-hidden');
      partialDiv.textContent = '';
    }

    const item = document.createElement('div');
    item.className = 'transcription-item';
    
    const isMe = text.length < 25;
    if (isMe) {
      item.classList.add('me');
      item.textContent = `• ${text}`;
    } else {
      item.classList.add('interviewer');
      item.textContent = `Q: ${text}`;
    }

    transcriptionFeed.prepend(item);
    while (transcriptionFeed.children.length > 5) {
      transcriptionFeed.removeChild(transcriptionFeed.lastChild);
    }
  }

  function updateSuggestion(text) {
    const formattedText = text
      .replace(/STAR \d+:/g, '<strong>💡 Pro Tip:</strong>')
      .replace(/Technical tip:/g, '<strong>🛠 Tech:</strong>')
      .replace(/Caution:/g, '<strong>⚠️ Caution:</strong>');
      
    mainSuggestion.innerHTML = formattedText;

    const transcription = state.conversationHistory.join(' ');
    if (transcription.length >= 10) {
      scheduleKnowledgeUpdate(transcription);
    }
  }

  function scheduleKnowledgeUpdate(transcription) {
    if (knowledgeDebounceTimer) {
      clearTimeout(knowledgeDebounceTimer);
    }
    knowledgeDebounceTimer = setTimeout(() => {
      updateKnowledgeContext(transcription);
    }, KNOWLEDGE_DEBOUNCE_MS);
  }

  async function updateKnowledgeContext(transcription) {
    if (!transcription || transcription.length < 10) return;
    
    const cacheKey = transcription.slice(-100);
    const now = Date.now();
    const cached = state.knowledgeCache.get(cacheKey);
    if (cached && (now - state.knowledgeCacheTime.get(cacheKey) || 0) < KNOWLEDGE_CACHE_TTL) {
      state.knowledgeContext = cached.context;
      state.relevantStories = cached.stories;
      renderKnowledgePanel();
      return;
    }
    
    try {
      const results = await invoke('search_knowledge_context', { 
        query: transcription 
      });
      state.knowledgeContext = results.filter(r => r.relevance > 0.5);
      
      const stories = await invoke('find_relevant_stories', { 
        context: transcription 
      });
      state.relevantStories = stories;
      
      state.knowledgeCache.set(cacheKey, {
        context: state.knowledgeContext,
        stories: state.relevantStories,
      });
      state.knowledgeCacheTime.set(cacheKey, now);
      
      renderKnowledgePanel();
    } catch (e) {
      console.error('Knowledge context error:', e);
    }
  }

  function renderKnowledgePanel() {
    const panel = document.getElementById('knowledge-panel');
    const skillsSection = document.getElementById('relevant-skills');
    const storiesSection = document.getElementById('relevant-stories');
    const storiesContent = document.getElementById('stories-content');
    
    if (!panel || !skillsSection || !storiesSection) return;
    
    const hasContext = state.knowledgeContext.length > 0;
    const hasStories = state.relevantStories.length > 0;
    
    if (hasContext || hasStories) {
      panel.classList.remove('hidden');
      panel.classList.add('visible');
    }
    
    if (hasContext) {
      const skills = state.knowledgeContext
        .filter(r => r.entity_type === 'skill')
        .slice(0, 5);
      
      if (skills.length > 0) {
        skillsSection.innerHTML = skills.map(s => 
          `<span class="skill-tag">${escapeHtml(s.name)}</span>`
        ).join('');
        skillsSection.classList.add('visible');
      } else {
        skillsSection.classList.remove('visible');
      }
    } else {
      skillsSection.classList.remove('visible');
    }
    
    if (hasStories) {
      const count = state.relevantStories.length;
      storiesSection.querySelector('.section-header').textContent = `⭐ Historias (${count})`;
      storiesContent.innerHTML = state.relevantStories.map(story => `
        <div class="story-card">
          <div class="story-title">${escapeHtml(story.title || 'Untitled')}</div>
          ${story.tags ? `<div class="story-tags">${story.tags.split(',').map(t => 
            `<span class="story-tag">${escapeHtml(t.trim())}</span>`
          ).join('')}</div>` : ''}
        </div>
      `).join('');
      storiesSection.classList.add('visible');
    } else {
      storiesSection.classList.remove('visible');
    }
  }

  function toggleKnowledgePanel() {
    const panel = document.getElementById('knowledge-panel');
    if (panel) {
      panel.classList.toggle('hidden');
      panel.classList.toggle('visible');
    }
  }

  function toggleSection(sectionId) {
    const content = document.getElementById(sectionId + '-content');
    if (content) {
      content.classList.toggle('hidden');
    }
  }

  function escapeHtml(text) {
    const div = document.createElement('div');
    div.textContent = text;
    return div.innerHTML;
  }

  // Audio Source Selection
  audioSourceBtn.addEventListener('click', async () => {
    processModal.classList.remove('hidden');
    modalOverlay.classList.remove('hidden');
    
    processList.innerHTML = '<div class="process-item">Escaneando...</div>';
    try {
      const devices = await invoke('get_audio_devices');
      const processes = await invoke('get_audio_processes');
      
      processList.innerHTML = '';
      
      devices.forEach(d => {
        const item = document.createElement('div');
        item.className = 'process-item';
        item.innerHTML = `<span>${d.label} ${d.name}</span>`;
        item.onclick = () => selectSource(null, d.index);
        processList.appendChild(item);
      });

      const appHeader = document.createElement('div');
      appHeader.className = 'process-header';
      appHeader.innerHTML = '<strong>Aplicaciones</strong>';
      processList.appendChild(appHeader);

      processes.forEach(p => {
        const item = document.createElement('div');
        item.className = 'process-item';
        item.innerHTML = `<span>${p.name}</span> <span class="pid">PID: ${p.pid}</span>`;
        item.onclick = () => selectSource(p.pid, null);
        processList.appendChild(item);
      });
    } catch (err) {
      processList.innerHTML = `<div class="process-item">Error: ${err}</div>`;
    }
  });

  async function selectSource(pid, deviceIndex) {
    processModal.classList.add('hidden');
    modalOverlay.classList.add('hidden');
    
    try {
      const mode = deviceIndex !== null && deviceIndex !== undefined ? 'mic' : 'system';
      await invoke('start_capture', { mode, pid, deviceIndex });
      statusIndicator.style.backgroundColor = '#4CAF50';
      mainSuggestion.textContent = "Conectado. Escuchando...";
    } catch (err) {
      console.error('JS: Capture start failed:', err);
      statusIndicator.style.backgroundColor = '#f44336';
    }
  }

  closeModalBtn.addEventListener('click', () => {
    processModal.classList.add('hidden');
    modalOverlay.classList.add('hidden');
  });

  clearBtn.addEventListener('click', async () => {
    await invoke('clear_feed');
    mainSuggestion.textContent = "Esperando audio...";
    transcriptionFeed.innerHTML = '';
  });

  regenerateBtn.addEventListener('click', () => invoke('regenerate'));

  profileBtn.addEventListener('click', async () => {
    try {
      await invoke('open_profile_window');
    } catch (err) {
      console.error('Failed to open profile window:', err);
    }
  });

  // Minimal mode toggle
  minimalBtn.addEventListener('click', async () => {
    try {
      const hudContainer = document.getElementById('hud-container');
      const isCurrentlyMinimal = hudContainer.classList.contains('minimal');
      await invoke('set_minimal_mode', { minimal: !isCurrentlyMinimal });
    } catch (err) {
      console.error('Failed to toggle minimal mode:', err);
    }
  });

  // Click on minimal icon to expand back
  minimalIcon.addEventListener('click', async () => {
    try {
      await invoke('set_minimal_mode', { minimal: false });
    } catch (err) {
      console.error('Failed to expand from minimal mode:', err);
    }
  });

  init().catch(console.error);
});
