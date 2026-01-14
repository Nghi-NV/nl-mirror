
import cv2
import numpy as np
import subprocess
import threading
import sys
import time

# Configuration
FFMPEG_CMD = [
    'ffmpeg',
    '-y', # Overwrite output files
    '-f', 'h264', # Input format
    '-flags', 'low_delay',
    '-fflags', 'nobuffer',
    '-probesize', '32',
    '-analyzeduration', '0',
    '-i', '-', # Read from stdin
    '-f', 'rawvideo', # Output raw video
    '-pix_fmt', 'bgr24', # OpenCV compatible
    '-'
]

def get_device_udid():
    """Get the first connected iOS device UDID via idb."""
    try:
        result = subprocess.check_output(['idb', 'list-targets', '--json']).decode('utf-8')
        # Simple parsing logic or just take the first line if not json
        # idb list-targets plain output often formatted as "Name | UDID | State"
        # Let's use simple parsing for robustness if json fails
        lines = subprocess.check_output(['idb', 'list-targets']).decode('utf-8').splitlines()
        for line in lines:
            if "Booted" in line or "Connected" in line: # Try to find active device
                parts = line.split('|')
                if len(parts) >= 2:
                    return parts[1].strip()
        return None
    except Exception as e:
        print(f"Error finding device: {e}")
        return None

def tap(udid, x, y):
    """Send tap command via idb."""
    # Run in thread to allow non-blocking UI
    def _run():
        subprocess.run(['idb', 'ui', 'tap', str(int(x)), str(int(y)), '--udid', udid], 
                       stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    threading.Thread(target=_run).start()

def main():
    if len(sys.argv) < 2:
        print("Usage: python3 ios_control.py <IP_ADDRESS> [UDID]")
        sys.exit(1)

    ip = sys.argv[1]
    port = 9999
    
    udid = sys.argv[2] if len(sys.argv) > 2 else get_device_udid()
    if not udid:
        print("❌ Could not find connected iOS device UDID. Please specify it as second argument.")
        sys.exit(1)
        
    print(f"📱 Controlling Device: {udid}")
    print(f"📺 Connecting to Stream: {ip}:{port}")
    
    # Start Netcat to receive stream
    nc_proc = subprocess.Popen(['nc', ip, str(port)], stdout=subprocess.PIPE, stderr=subprocess.DEVNULL)
    
    # Start FFMPEG to decode stream
    ffmpeg_proc = subprocess.Popen(FFMPEG_CMD, stdin=nc_proc.stdout, stdout=subprocess.PIPE, stderr=subprocess.DEVNULL)
    
    # Determine window name
    window_name = "NL-iOS Mirror (Click to Control)"
    cv2.namedWindow(window_name, cv2.WINDOW_NORMAL)
    cv2.resizeWindow(window_name, 540, 960) # Half resolution initial size
    
    # Mouse callback
    # We need to assume device resolution or fetch it
    # For simplicity, let's assume we are viewing 1080x1920 (streaming resolution)
    # And iPhone logic resolution is usually 1/3 or 1/2 of that.
    # Actually idb expects Logical Points.
    # iPhone 11/12/13/14 Pro Max: 428 x 926 points (roughly)
    # Standard 1080p stream -> usually 3x scale -> 360x640 points?
    # Let's Fetch screen info if possible using `idb describe`
    # For now, let's use a dynamic scaling factor based on window size vs known logic size?
    # Or just hardcode for testing:
    # Assuming video is 1080x1920.
    # iPhone points are usually width / scale.
    # Let's try sending pixels / 3 (since typical iPhone is @3x)
    
    scale_factor = 3.0 
    
    def mouse_callback(event, x, y, flags, param):
        if event == cv2.EVENT_LBUTTONDOWN:
            # Map window coordinates to video frame coordinates
            # Window size might differ from frame size if resized
            # But cv2 mouse gives window coordinates
            # We verify the frame size in the loop
            
            # This is tricky without knowing exact current frame size scaling
            # Let's store current frame dimension in param
            frame_w, frame_h = param['w'], param['h']
            
            # But OpenCV window resize stretches content.
            # We need the ratio.
            # Actually, let's just use the ratio of (x/window_width) * device_logic_width
            
            # Better strategy: Get device info first via idb describe
            pass

    # Start loop
    width = 1080
    height = 1920
    
    # Attempt to get screen dimensions
    try:
        desc = subprocess.check_output(['idb', 'describe', '--udid', udid, '--json']).decode('utf-8')
        import json
        info = json.loads(desc)
        # Try to find screen dimensions
        # If unavailable, fallback
        pass
    except:
        pass

    # Simple approach: Map click to relative percentage, then apply to device 'point' width
    # But we don't know device point width without query.
    # Let's just output log for now and do a best guess: 
    # Use 360x(Height ratio) (Standard iPhone width in points is often 375, 390, 414, or 428)
    # Let's guess 390 (iPhone 12/13/14)
    target_width_points = 390
    
    current_frame_size = {'w': 1080, 'h': 1920} # Default
    
    def on_mouse(event, x, y, flags, param):
        if event == cv2.EVENT_LBUTTONDOWN:
            # Get current window size
            try:
                # cv2.getWindowImageRect might give exact size
                rect = cv2.getWindowImageRect(window_name)
                win_w, win_h = rect[2], rect[3]
            except:
                win_w, win_h = 540, 960 # Fallback
            
            if win_w == 0: win_w = 1
            if win_h == 0: win_h = 1
            
            # Calculate relative position (0.0 - 1.0)
            rel_x = x / win_w
            rel_y = y / win_h
            
            # Map to device points
            # We assume user can tune this or we fetch it.
            # Let's accept input? No, too complex.
            # Let's assume 1080p stream is 3x logic.
            # 1080 / 3 = 360.
            # But if device is iPhone 15 Pro Max (1290p), it's 430.
            # Let's send a TAP based on 414 width (average huge phone)
            
            # TODO: Improve this
            final_x = rel_x * 390 # Guessing iPhone width
            final_y = rel_y * 844 # Guessing iPhone height
            
            print(f"Click: {x},{y} -> Rel: {rel_x:.2f},{rel_y:.2f} -> Device: {int(final_x)},{int(final_y)}")
            tap(udid, final_x, final_y)

    cv2.setMouseCallback(window_name, on_mouse)

    print("🚀 Starting Stream...")
    
    while True:
        # Read raw video frame
        # 1080 * 1920 * 3 bytes (BGR)
        # Note: Resolution might change dynamically! 
        # FFMpeg rawvideo pipe doesn't tell us boundary.
        # This is the hard part of piping raw video without container.
        # However, if we assume fixed resolution for specific session start...
        
        # Better approach: Read chunks?
        # Actually cv2.VideoCapture can read from pipe!
        # Let's restart logic using cv2.VideoCapture with pipe
        break

    ffmpeg_proc.kill()
    nc_proc.kill()

# Restart using VideoCapture approach for easier frame handling
def main_cv2():
    if len(sys.argv) < 2:
        print("Usage: python3 ios_control.py <IP_ADDRESS> [UDID]")
        sys.exit(1)

    ip = sys.argv[1]
    port = 9999
    udid = sys.argv[2] if len(sys.argv) > 2 else get_device_udid()
    
    if not udid:
        print("❌ Device UDID not found for control.")
        # Proceed viewing only?
        # sys.exit(1)

    stream_url = f"tcp://{ip}:{port}"
    print(f"📺 Connecting to {stream_url}")
    
    # Open valid stream. Using ffplay pipeline string for cv2 is tricky.
    # But cv2 can open 'tcp://...' directly if ffmpeg backend is enabled!
    # Or piped string.
    
    # We use a pipe string for GStreamer or FFmpeg
    # Simple TCP usually works if stream is valid MPEG-TS or raw H264?
    # Our stream sends raw H264 NAL units. OpenCV FFmpeg backend usually handles it.
    
    cap = cv2.VideoCapture(f"tcp://{ip}:{port}")
    
    # If connection fails
    if not cap.isOpened():
        print("Failed to open stream. Trying pipe method...")
        # Fallback to shell pipe
        # Use a generic pipe command
        cmd = f"nc {ip} {port} | ffmpeg -i - -f rawvideo -pix_fmt bgr24 -an -vcodec rawvideo -"
        # Not easily supported in cv2.VideoCapture constructor on all platforms
        print("Please check connection.")
        return

    window_name = "NL-iOS Mirror"
    cv2.namedWindow(window_name, cv2.WINDOW_NORMAL)
    cv2.resizeWindow(window_name, 540, 960)
    
    if udid:
        # Fetch device point size
        try:
             # Basic heuristic:
             # Just map click 1:1 to resolution? No, idb uses points.
             pass
        except: pass
        
        def on_mouse(event, x, y, flags, param):
            if event == cv2.EVENT_LBUTTONDOWN:
                frame_w = cap.get(cv2.CAP_PROP_FRAME_WIDTH)
                frame_h = cap.get(cv2.CAP_PROP_FRAME_HEIGHT)
                
                # Window size
                rect = cv2.getWindowImageRect(window_name)
                win_w = rect[2]
                win_h = rect[3]
                
                # Scale
                rel_x = x / win_w
                rel_y = y / win_h
                
                # We need to map to Logic Points.
                # Heuristic: Frame Width (Pixels) / Scale = Logic Points
                # Scale is usually 3.0 for modern iPhones, 2.0 for older.
                # If width > 1000 => Scale 3.
                scale = 3.0 if frame_w > 1000 else 2.0
                
                logic_x = (rel_x * frame_w) / scale
                logic_y = (rel_y * frame_h) / scale
                
                tap(udid, logic_x, logic_y)
                
        cv2.setMouseCallback(window_name, on_mouse)

    while True:
        ret, frame = cap.read()
        if not ret:
            print("Lost frame. Reconnecting...")
            time.sleep(1)
            cap.open(f"tcp://{ip}:{port}")
            continue
            
        cv2.imshow(window_name, frame)
        
        if cv2.waitKey(1) & 0xFF == ord('q'):
            break

    cap.release()
    cv2.destroyAllWindows()

if __name__ == "__main__":
    main_cv2()
