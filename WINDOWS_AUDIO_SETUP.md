# Windows System Audio Capture Setup

GhostAI Pilot uses [cpal](https://github.com/RustAudio/cpal) with WASAPI for audio capture on Windows.

> **Recommended:** Set `AUDIO_MODE=polling` in your `.env` file. Polling mode provides reliable system audio capture without requiring virtual audio cables or Stereo Mix on most Windows setups. The `auto` mode (default) will attempt WASAPI loopback first and fall back to polling if needed.

## The Problem

Windows doesn't have a simple "capture system audio" API like Linux's PulseAudio monitor. WASAPI provides **loopback** recording, but it requires either:

1. Hardware/driver support (Stereo Mix)
2. Virtual audio cable software

## Option 1: Stereo Mix (Simplest)

Many sound cards have a "Stereo Mix" recording device built-in:

1. Right-click the speaker icon in system tray → **Sound settings**
2. Click **More sound settings** (opens classic Control Panel)
3. Go to **Recording** tab
4. Right-click in empty space → **Show Disabled Devices**
5. Find **Stereo Mix** → Right-click → **Enable**
6. Set it as **Default Device**

GhostAI will detect Stereo Mix as an input device automatically.

## Option 2: VB-Cable (Recommended)

Free virtual audio cable — routes speaker output to a virtual input:

1. Download from https://vb-audio.com/Cable/
2. Extract and run `VBCABLE_Setup_x64.exe` as Administrator
3. Click **Install Driver**
4. Restart your PC
5. Set **CABLE Input** as your default playback device
6. GhostAI captures from **CABLE Output** (the virtual mic)

To still hear audio yourself:
- Open Sound Settings → Advanced → More sound settings
- Go to **Playback** tab
- Right-click your speakers → **Properties** → **Listen** tab
- Check **Listen to this device**
- Set playback device to your real speakers

## Option 3: VoiceMeeter (Advanced)

More powerful routing but more complex:

1. Download from https://vb-audio.com/Voicemeeter/
2. Install and restart
3. Set VoiceMeeter Input as default playback
4. GhostAI captures from VoiceMeeter Output

## Troubleshooting

**"No audio input device found"**
- Make sure you enabled Stereo Mix or installed VB-Cable
- Check that the device appears in Sound Settings → Recording

**Captures microphone instead of system audio**
- You're hearing your own voice, not the meeting audio
- Enable Stereo Mix or install VB-Cable

**Audio sounds distorted or choppy**
- Try changing the sample rate in Sound Settings
- Right-click device → Properties → Advanced → Set to 44100 Hz or 48000 Hz

**WASAPI exclusive mode conflicts**
- Some apps take exclusive control of audio
- Close other audio apps, or disable exclusive mode:
  Sound Settings → Device Properties → Advanced → Uncheck "Allow applications to take exclusive control"
