
import http.server
import socketserver
import socket
import json
import sys
import subprocess
import threading
import time
import os

PORT = 8000

def get_local_ip():
    try:
        # Connect to a dummy external IP to determine the interface used for default route
        s = socket.socket(socket.AF_INET, socket.SOCK_DGRAM)
        s.connect(("8.8.8.8", 80))
        ip = s.getsockname()[0]
        s.close()
        return ip
    except:
        return "127.0.0.1"

class HandshakeHandler(http.server.SimpleHTTPRequestHandler):
    def do_POST(self):
        if self.path == '/pair':
            content_length = int(self.headers['Content-Length'])
            post_data = self.rfile.read(content_length)
            
            try:
                data = json.loads(post_data.decode('utf-8'))
                device_ip = data.get('device_ip')
                
                if device_ip:
                    print(f"\n✅ Received pairing request from Device: {device_ip}")
                    
                    # Send success response
                    self.send_response(200)
                    self.send_header('Content-type', 'application/json')
                    self.end_headers()
                    self.wfile.write(json.dumps({"status": "ok"}).encode('utf-8'))
                    
                    # Signal main thread
                    self.server.device_ip = device_ip
                    return
            except Exception as e:
                print(f"Error parsing request: {e}")
                
        self.send_response(400)
        self.end_headers()

def generate_qr(data):
    try:
        # Try importing qrcode library
        import qrcode
        qr = qrcode.QRCode()
        qr.add_data(data)
        qr.make()
        qr.print_ascii(invert=True)
    except ImportError:
        print("\n⚠️  'qrcode' library not found. Install it for fancy QR: pip install qrcode")
        print(f"\nOr manually enter this URL on device: {data}\n")

def main():
    host_ip = get_local_ip()
    pair_url = f"http://{host_ip}:{PORT}/pair"
    
    print("\n" + "="*40)
    print(f"🚀 NL-Mirror QR Handshake")
    print("="*40)
    print(f"Scan this QR code with NL-iOS App to pair:\n")
    
    generate_qr(pair_url)
    
    print(f"\nWaiting for connection on {pair_url}...")
    
    # Start Server
    with socketserver.TCPServer(("", PORT), HandshakeHandler) as httpd:
        httpd.device_ip = None
        
        # Poll for device_ip
        threading.Thread(target=httpd.serve_forever).start()
        
        while httpd.device_ip is None:
            time.sleep(0.5)
            
        print("Stopping handshake server...")
        httpd.shutdown()
        
        # Launch Controller
        device_ip = httpd.device_ip
        print(f"🚀 Launching Controller for {device_ip}...")
        
        # Check if ios_control.py exists
        if os.path.exists("ios_control.py"):
            subprocess.run(["python3", "ios_control.py", device_ip])
        else:
            # Fallback to view_stream
            subprocess.run(["./view_stream.sh", device_ip])

if __name__ == "__main__":
    main()
