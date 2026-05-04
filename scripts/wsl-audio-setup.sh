#!/bin/bash
# WSL Audio Setup — PulseAudio for microphone capture
# Run this on Windows host to enable WSL mic access.
#
# Prerequisites:
# 1. Install PulseAudio for Windows:
#    winget install PulseAudio.PulseAudio
#    Or download from: https://www.freedesktop.org/wiki/Software/PulseAudio/Ports/Windows/Support/
#
# Setup:
# 2. Start PulseAudio server on Windows (in PowerShell as admin):
#    & 'C:\Program Files\PulseAudio\bin\pulseaudio.exe' --exit-idle-time=-1
#
# 3. Configure WSL to connect to Windows PulseAudio server:
#    export PULSE_SERVER=tcp:$(hostname).local
#
# This script applies the WSL config permanently.

set -euo pipefail

# Detect Windows host IP from WSL
WINDOWS_IP=$(grep -m1 nameserver /etc/resolv.conf | awk '{print $2}')
echo "Windows host IP: $WINDOWS_IP"

# Add PulseAudio env to ~/.bashrc if not already present
LINE="export PULSE_SERVER=tcp:$WINDOWS_IP"
if ! grep -q "PULSE_SERVER" ~/.bashrc 2>/dev/null; then
    echo "$LINE" >> ~/.bashrc
    echo "Added: $LINE to ~/.bashrc"
else
    echo "PULSE_SERVER already configured in ~/.bashrc"
fi

echo ""
echo "=== Next Steps ==="
echo "1. On Windows: Start PulseAudio server"
echo "   powershell.exe -Command \"Start-Process 'C:\\Program Files\\PulseAudio\\bin\\pulseaudio.exe' -ArgumentList '--exit-idle-time=-1'\""
echo ""
echo "2. Test audio devices from WSL:"
echo "   pactl info"
echo "   pactl list sources short"
echo ""
echo "3. Run ghostai-pilot with mic:"
echo "   cd /home/maiso/.hermes/hermes-agent/.worktrees/t_13c89aca"
echo "   cargo run --no-default-features --features \"audio\""
