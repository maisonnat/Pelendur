## Exploration: Windows System Audio Capture for USB/Bluetooth Headphones

### Current State
The GhostAI Pilot project uses `cpal` with WASAPI loopback to capture system audio on Windows. The current implementation in `src/audio.rs` attempts to:
1. Find loopback devices by name (containing "loopback")
2. Fall back to default output device as loopback source
3. Use first available input device as last resort

**Problem**: WASAPI loopback doesn't work directly with USB headphones (Alienware AW520H) or Bluetooth headphones (Galaxy Buds Pro) because:
- USB/BT audio devices use different audio endpoints than traditional analog outputs
- Windows audio stack treats USB/BT devices as separate render endpoints
- WASAPI loopback only works with the default system audio renderer (usually Realtek/analog)

**Current setup attempts**:
- VoiceMeeter is installed but B1 routing isn't working
- Stereo Mix only captures audio going through Realtek chip
- User needs to hear audio through headphones WHILE GhostAI captures it

### Affected Areas
- `src/audio.rs` — Main audio capture logic using cpal/WASAPI
- `WINDOWS_AUDIO_SETUP.md` — Documentation for audio setup
- `enable-stereomix.ps1` — Script to enable Stereo Mix
- `VoiceMeeterSetup/` — Contains VoiceMeeter installer

### Approaches

#### 1. **VoiceMeeter Configuration** — Route all audio through VoiceMeeter virtual mixer
   - **Pros**: 
     - Can mix multiple audio sources
     - Supports hardware output passthrough
     - Can route to virtual cables
     - GUI for configuration
   - **Cons**:
     - Complex setup, requires understanding of audio routing
     - B1/B2/B3 routing can be confusing
     - May introduce latency
     - No command-line configuration (requires GUI or registry edits)
   - **Effort**: Medium

#### 2. **VB-Cable Alone** — Install VB-Cable and route audio through it
   - **Pros**:
     - Simple virtual audio cable
     - Free and lightweight
     - Can be set as default playback device
     - "Listen to this device" feature allows hearing while capturing
   - **Cons**:
     - Requires changing default audio device
     - May not work well with USB/BT headphones if they're not the default
     - Single cable limitation (can't mix multiple sources)
   - **Effort**: Low

#### 3. **WASAPI Loopback Workaround** — Technical workaround for USB/BT devices
   - **Pros**:
     - No additional software needed
     - Direct Windows API access
   - **Cons**:
     - Fundamentally limited by Windows audio architecture
     - USB/BT devices create separate audio sessions
     - Would require hooking into audio graph or using undocumented APIs
   - **Effort**: High (if possible at all)

#### 4. **PipeWire/Alternative Virtual Audio** — Windows alternatives to VoiceMeeter
   - **Pros**:
     - Modern audio routing
     - Lower latency than VoiceMeeter
     - Better API support
   - **Cons**:
     - Limited Windows support (most alternatives are Linux-focused)
     - No mature Windows virtual audio solutions beyond VB-Audio products
   - **Effort**: Medium-High

#### 5. **Programmatic Approach** — Use Windows Audio Policy API/Core Audio API
   - **Pros**:
     - Direct integration with application
     - Can potentially redirect audio streams programmatically
     - No user configuration needed
   - **Cons**:
     - Extremely complex (Windows audio stack is notoriously difficult)
     - Requires COM interfaces, undocumented APIs
     - May require admin privileges
     - High development time
   - **Effort**: Very High

### Recommendation

**Recommended Solution: VB-Cable + "Listen to this device" configuration**

**Why**:
1. **Simplicity**: VB-Cable is the simplest virtual audio solution
2. **Reliability**: Well-established, works with all audio applications
3. **User Experience**: "Listen to this device" allows hearing audio while capturing
4. **Integration**: cpal already detects VB-Cable devices (see code: `name.contains("cable")`)

**Implementation Steps**:

1. **Install VB-Cable**:
   ```
   Download from https://vb-audio.com/Cable/
   Run VBCABLE_Setup_x64.exe as Administrator
   Click "Install Driver"
   Restart PC
   ```

2. **Configure Audio Routing**:
   - Open Sound Settings → More sound settings
   - **Playback tab**: Set "CABLE Input (VB-Audio Virtual Cable)" as default
   - **Recording tab**: Verify "CABLE Output" appears
   - **For headphones**: Right-click your headphones → Properties → Listen tab → Check "Listen to this device" → Set playback to "CABLE Input"

3. **Update GhostAI Configuration**:
   - Modify `src/audio.rs` to prioritize VB-Cable devices
   - Add auto-detection of "CABLE Output" as preferred capture device
   - Add configuration option for manual device selection

4. **Alternative: VoiceMeeter Setup** (if VB-Cable alone doesn't work):
   - Install VoiceMeeter Banana (more advanced)
   - Set VoiceMeeter Input as default playback
   - Route hardware out to headphones
   - Capture from VoiceMeeter Output

### Risks
- **Latency**: Virtual audio cables add 10-50ms latency (acceptable for transcription)
- **Compatibility**: Some applications may not respect default device changes
- **Audio Quality**: May need to adjust sample rates (44.1kHz vs 48kHz)
- **User Complexity**: Non-technical users may struggle with audio routing setup

### Ready for Proposal
**Yes** — The recommended approach is clear and implementable. The orchestrator should:
1. Propose VB-Cable installation and configuration
2. Update audio.rs to prioritize VB-Cable devices
3. Add clear setup instructions to documentation
4. Consider adding VoiceMeeter as advanced alternative