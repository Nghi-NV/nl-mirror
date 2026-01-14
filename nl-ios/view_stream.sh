#!/bin/bash
IP=${1:-10.10.60.14}
PORT=9999

echo "📺 Connecting to nl-ios mirror at $IP:$PORT..."
# Loop to auto-reconnect
while true; do
    echo "📺 Connecting to nl-ios mirror at $IP:$PORT..."
    echo "   Window height limited to 900px for desktop viewing."
    
    # -y 900: Set window height to 900
    # Using a shorter timeout for nc check could effectively rely on ffplay closing when pipe closes
    nc $IP $PORT | ffplay -f h264 \
        -flags low_delay -fflags nobuffer \
        -probesize 32 -analyzeduration 0 \
        -sync ext \
        -framedrop \
        -hide_banner \
        -loglevel warning \
        -y 900 \
        -i -
        
    echo "⚠️ Connection lost or closed. Reconnecting in 2 seconds..."
    sleep 2
done
