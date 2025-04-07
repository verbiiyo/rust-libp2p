# server.py
import asyncio
import threading
import http.server
import socketserver
import websockets

SIGNAL_PORT = 9001
HTTP_PORT = 8000


# --- WebSocket Signaling Server ---
peers = set()

async def signaling(websocket, path):
    peers.add(websocket)
    try:
        async for msg in websocket:
            for peer in peers:
                if peer != websocket:
                    await peer.send(msg)
    finally:
        peers.remove(websocket)

async def start_signaling():
    async with websockets.serve(signaling, "localhost", SIGNAL_PORT):
        print(f"🔌 Signaling server running on ws://localhost:{SIGNAL_PORT}")
        await asyncio.Future()  # Run forever


# --- Static File Server ---
def start_static():
    handler = http.server.SimpleHTTPRequestHandler
    with socketserver.TCPServer(("", HTTP_PORT), handler) as httpd:
        print(f"📦 Static server running on http://localhost:{HTTP_PORT}")
        httpd.serve_forever()


# --- Entrypoint ---
if __name__ == "__main__":
    threading.Thread(target=start_static, daemon=True).start()
    asyncio.run(start_signaling())
