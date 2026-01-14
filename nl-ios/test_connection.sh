#!/bin/bash
# Test connection to nl-ios mirror server

if [ -z "$1" ]; then
    echo "Usage: ./test_connection.sh <IPHONE_IP>"
    echo "Example: ./test_connection.sh 10.10.60.100"
    exit 1
fi

IP=$1
PORT=9999

echo "🔍 Testing connection to nl-ios mirror at $IP:$PORT..."

# Test TCP connection
nc -zv -w 3 $IP $PORT 2>&1

if [ $? -eq 0 ]; then
    echo "✅ Connection successful! Server is listening."
    echo ""
    echo "📺 To receive video stream, run:"
    echo "   nc $IP $PORT | ffplay -f h264 -probesize 32 -flags low_delay -"
else
    echo "❌ Connection failed. Make sure:"
    echo "   1. iPhone and Mac are on same WiFi network"
    echo "   2. nl-ios app is running and broadcasting"
    echo "   3. IP address is correct"
fi
