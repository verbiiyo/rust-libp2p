import asyncio
import websockets
import http.server
import socketserver
import threading

PORT = 8000
SIGNAL_PORT = 8765

peers = set()

# --- WebSocket signaling server ---
async def signaling(websocket):
    peers.add(websocket)
    try:
        async for msg in websocket:
            for peer in peers:
                if peer != websocket:
                    await peer.send(msg)
    finally:
        peers.remove(websocket)

def start_signaling():
    asyncio.set_event_loop(asyncio.new_event_loop())
    start_server = websockets.serve(signaling, "localhost", SIGNAL_PORT)
    asyncio.get_event_loop().run_until_complete(start_server)
    asyncio.get_event_loop().run_forever()

# --- Static file server (serves index.html + pkg/ etc) ---
def start_static():
    handler = http.server.SimpleHTTPRequestHandler
    with socketserver.TCPServer(("", PORT), handler) as httpd:
        print(f"Serving at http://localhost:{PORT}")
        httpd.serve_forever()

# --- Start both ---
threading.Thread(target=start_signaling, daemon=True).start()
start_static()
