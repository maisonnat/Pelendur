#!/usr/bin/env python3
"""Generate synthetic test audio WAV files for Pelendur STP testing.
Uses gTTS (Google Text-to-Speech) to generate realistic interview questions at 16kHz mono WAV."""

import subprocess
import tempfile
from pathlib import Path

AUDIO_DIR = Path(__file__).parent / "audio"

QUESTIONS = [
    ("what_is_your_greatest_strength.wav", "What is your greatest strength?"),
    ("tell_me_about_yourself.wav", "Tell me about yourself"),
    ("why_do_you_want_this_job.wav", "Why do you want this job?"),
    ("describe_a_challenge.wav", "Describe a challenge you overcame"),
    ("where_do_you_see_yourself.wav", "Where do you see yourself in five years?"),
]

def generate_wav(filename: str, text: str) -> Path:
    """Generate a 16kHz mono WAV file using gTTS + ffmpeg."""
    out_path = AUDIO_DIR / filename
    if out_path.exists():
        print(f"  ✓ Already exists: {filename}")
        return out_path

    # Use gTTS to generate speech, convert to 16kHz mono WAV
    with tempfile.NamedTemporaryFile(suffix=".mp3", delete=False) as tmp:
        mp3_path = tmp.name

    try:
        from gtts import gTTS
        tts = gTTS(text, lang="en", slow=False)
        tts.save(mp3_path)

        # Convert to 16kHz mono WAV
        subprocess.run(
            ["ffmpeg", "-y", "-i", mp3_path,
             "-ar", "16000", "-ac", "1",
             "-sample_fmt", "s16",
             str(out_path)],
            capture_output=True, check=True
        )
        size = out_path.stat().st_size
        print(f"  ✓ Generated: {filename} ({size/1024:.1f} KB)")
    except ImportError:
        print("  ⚠ gtts not installed. Install with: pip install gtts")
        print("  ⚠ Or use fallback: generate_silent_wav()")
        return generate_silent_wav(out_path, text)
    finally:
        Path(mp3_path).unlink(missing_ok=True)

    return out_path


def generate_silent_wav(out_path: Path, text: str) -> Path:
    """Fallback: generate a short silent WAV (for CI without gTTS)."""
    import struct
    import math

    sample_rate = 16000
    duration = 2.0  # 2 seconds of silence
    num_samples = int(sample_rate * duration)

    with open(out_path, "wb") as f:
        # WAV header
        data_size = num_samples * 2  # 16-bit
        f.write(b"RIFF")
        f.write(struct.pack("<I", 36 + data_size))
        f.write(b"WAVE")
        f.write(b"fmt ")
        f.write(struct.pack("<I", 16))  # chunk size
        f.write(struct.pack("<H", 1))   # PCM
        f.write(struct.pack("<H", 1))   # mono
        f.write(struct.pack("<I", sample_rate))
        f.write(struct.pack("<I", sample_rate * 2))  # byte rate
        f.write(struct.pack("<H", 2))   # block align
        f.write(struct.pack("<H", 16))  # bits per sample
        f.write(b"data")
        f.write(struct.pack("<I", data_size))
        # Write silence
        for _ in range(num_samples):
            f.write(struct.pack("<h", 0))

    size = out_path.stat().st_size
    print(f"  ⚠ Generated silent fallback: {out_path.name} ({size/1024:.1f} KB)")
    return out_path


def main():
    AUDIO_DIR.mkdir(parents=True, exist_ok=True)
    print(f"🎤 Generating test audio files in {AUDIO_DIR}")
    for filename, question in QUESTIONS:
        generate_wav(filename, question)
    print(f"\n✅ Done! {len(list(AUDIO_DIR.glob('*.wav')))} WAV files in {AUDIO_DIR}")


if __name__ == "__main__":
    main()
