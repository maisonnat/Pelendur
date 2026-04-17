// Pelendur HUD UI Logic
// Use a safe wrapper to wait for Tauri
document.addEventListener('DOMContentLoaded', () => {
  const { invoke } = window.__TAURI__.core;
  const { listen } = window.__TAURI__.event;

  const mainSuggestion = document.getElementById('main-suggestion');
  const transcriptionFeed = document.getElementById('transcription-feed');
  const partialDiv = document.getElementById('partial-transcription');
  const audioSourceBtn = document.getElementById('audio-source-btn');
  const lockBtn = document.getElementById('lock-btn');
  const clearBtn = document.getElementById('clear-btn');
  const regenerateBtn = document.getElementById('regenerate-btn');
  const statusIndicator = document.getElementById('status-indicator');
  const profileBtn = document.getElementById('profile-btn');

  const processModal = document.getElementById('process-modal');
  const processList = document.getElementById('process-list');
  const closeModalBtn = document.getElementById('close-modal-btn');
  const modalOverlay = document.getElementById('modal-overlay');

  const state = {
    knowledgeContext: [],
    relevantStories: [],
    conversationHistory: [],
    knowledgeCache: new Map(),
    knowledgeCacheTime: new Map(),
  };

  let isLocked = false;
  let knowledgeDebounceTimer = null;
  const KNOWLEDGE_CACHE_TTL = 30000;
  const KNOWLEDGE_DEBOUNCE_MS = 2000;

  async function init() {
    console.log('--- Pelendur HUD Initializing ---');
    mainSuggestion.textContent = "Esperando audio... Seleccioná el dispositivo.";

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
  }

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
      await invoke('start_capture', { pid, deviceIndex });
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

  init().catch(console.error);
});
