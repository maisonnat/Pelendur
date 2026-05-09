#!/usr/bin/env python3
"""Generate comprehensive test audio for Pelendur STT pipeline testing.
Creates WAV files at 16kHz mono in multiple languages and lengths."""
import subprocess, tempfile, os, sys, time
from pathlib import Path

AUDIO_DIR = Path(__file__).parent / "audio"
AUDIO_DIR.mkdir(exist_ok=True)

# English - short (fast tests)
EN_SHORT = [
    ("en_hello.wav", "Hello, how are you?"),
    ("en_yes.wav", "Yes, that sounds good."),
    ("en_no.wav", "No, I don't think so."),
    ("en_test_one_two.wav", "Testing one two three."),
]

# English - interview style
EN_INTERVIEW = [
    ("en_strength.wav", "My greatest strength is my ability to learn quickly and adapt to new situations."),
    ("en_challenge.wav", "I once had to lead a team through a major software migration with very tight deadlines."),
    ("en_teamwork.wav", "I believe in clear communication and regular check-ins to keep everyone aligned."),
]

# Spanish
ES_SHORT = [
    ("es_hola.wav", "Hola, ¿cómo estás?"),
    ("es_si.wav", "Sí, estoy de acuerdo."),
    ("es_gracias.wav", "Muchas gracias por la oportunidad."),
]

ES_INTERVIEW = [
    ("es_fortaleza.wav", "Mi mayor fortaleza es mi capacidad para resolver problemas complejos bajo presión."),
    ("es_experiencia.wav", "Tengo más de diez años de experiencia en ventas técnicas y gestión de cuentas."),
]

# Mixed noise floor test (silence + speech)
SILENCE_TESTS = [
    ("silence_500ms.wav", ""),         # pure silence
]

ALL_FILES = EN_SHORT + EN_INTERVIEW + ES_SHORT + ES_INTERVIEW + SILENCE_TESTS

def generate_wav(filename: str, text: str) -> Path:
    out_path = AUDIO_DIR / filename
    if out_path.exists():
        print(f"  ✓ EXISTS: {filename} ({out_path.stat().st_size//1024}KB)")
        return out_path

    if not text:
        # Generate pure silence WAV
        import struct, wave
        with wave.open(str(out_path), 'w') as wf:
            wf.setnchannels(1)
            wf.setsampwidth(2)  # 16-bit
            wf.setframerate(16000)
            # 500ms of silence
            for _ in range(8000):
                wf.writeframes(struct.pack('<h', 0))
        print(f"  ✓ GENERATED: {filename} (silence)")
        return out_path

    with tempfile.NamedTemporaryFile(suffix=".mp3", delete=False) as tmp:
        mp3_path = tmp.name

    try:
        from gtts import gTTS
        # Detect language
        lang = "es" if any(c in text for c in "áéíóúñ¿¡") else "en"
        # Slower speed for clearer audio
        tts = gTTS(text=text, lang=lang, slow=False)
        tts.save(mp3_path)
        # Convert to 16kHz mono WAV
        cmd = [
            "ffmpeg", "-y", "-i", mp3_path,
            "-acodec", "pcm_s16le",
            "-ac", "1",
            "-ar", "16000",
            str(out_path)
        ]
        subprocess.run(cmd, capture_output=True, check=True)
        kb = out_path.stat().st_size // 1024
        duration = out_path.stat().st_size / 32000  # 16-bit mono @ 16kHz = 32000 bytes/sec
        print(f"  ✓ GENERATED: {filename} ({kb}KB, {duration:.1f}s)")
        return out_path
    finally:
        os.unlink(mp3_path)

print(f"\n{'='*50}")
print(f"  Generating {len(ALL_FILES)} test audio files")
print(f"{'='*50}\n")
total_start = time.time()

for filename, text in ALL_FILES:
    generate_wav(filename, text)

elapsed = time.time() - total_start
print(f"\n{'='*50}")
print(f"  Done in {elapsed:.1f}s")
print(f"  Files in: {AUDIO_DIR}")
print(f"{'='*50}\n")

# Verify all files
print("Verification:")
for f in sorted(AUDIO_DIR.glob("*.wav")):
    dur = f.stat().st_size / 32000
    print(f"  {f.name:35s} {f.stat().st_size//1024:4d}KB  {dur:5.1f}s")
