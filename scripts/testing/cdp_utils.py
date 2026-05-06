import json
import time
import asyncio
import base64
from typing import Any

try:
    import websockets
except ImportError:
    websockets = None


class CDPError(Exception):
    pass


class CDPConnection:
    def __init__(self, host: str = "127.0.0.1", port: int = 9224):
        self.host = host
        self.port = port
        self.ws = None
        self._msg_id = 0

    async def connect(self) -> None:
        if websockets is None:
            raise ImportError("websockets not installed. Run: pip install websockets")
        import httpx
        async with httpx.AsyncClient() as client:
            resp = await client.get(f"http://{self.host}:{self.port}/json")
            pages = resp.json()
        hud_tab = None
        for page in pages:
            if page.get("title", "").lower().startswith("pelendur"):
                hud_tab = page
                break
        if not hud_tab:
            hud_tab = pages[0] if pages else None
        if not hud_tab:
            raise CDPError("No WebView2 pages found on CDP port")
        self.ws = await websockets.connect(hud_tab["webSocketDebuggerUrl"])

    async def _send(self, method: str, params: dict | None = None) -> dict:
        self._msg_id += 1
        msg = {"id": self._msg_id, "method": method}
        if params:
            msg["params"] = params
        await self.ws.send(json.dumps(msg))
        resp = json.loads(await self.ws.recv())
        while resp.get("id") != self._msg_id:
            resp = json.loads(await self.ws.recv())
        if "error" in resp:
            raise CDPError(resp["error"]["message"])
        return resp.get("result", {})

    async def invoke(self, command: str, args: dict | None = None) -> Any:
        js = f"window.__TAURI__.invoke('{command}', {json.dumps(args or {})})"
        result = await self._send("Runtime.evaluate", {
            "expression": js,
            "awaitPromise": True,
        })
        val = result.get("result", {}).get("value")
        if result.get("result", {}).get("type") == "string":
            try:
                return json.loads(val)
            except (TypeError, json.JSONDecodeError):
                pass
        return val

    async def eval(self, js: str) -> Any:
        result = await self._send("Runtime.evaluate", {
            "expression": js,
            "awaitPromise": True,
        })
        return result.get("result", {}).get("value")

    async def screenshot(self) -> bytes:
        result = await self._send("Page.captureScreenshot", {"format": "png"})
        data = result.get("data", "")
        return base64.b64decode(data)

    async def close(self) -> None:
        if self.ws:
            await self.ws.close()
