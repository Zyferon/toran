"""Socket client to the Toran Rust core.

One Client object owns one Unix socket connection. The client is
thread-safe in the sense that concurrent calls serialize on an
internal lock; this is fine for the human-approval workload (a
function either runs, blocks for one human, or raises). If you need
fan-out throughput, create one Client per thread.

The wire format is a single line of JSON per message, terminated by
`\\n`. The first byte is a length prefix in the Rust protocol (see
src/protocol.rs) but our newline-delimited variant is friendlier for
ad-hoc curl debugging.
"""

from __future__ import annotations

import errno
import json
import os
import socket
import threading
import time
from typing import Any, Optional

from .exceptions import ToranConnectionError


class Client:
    """A blocking client to the Toran core over a Unix socket."""

    def __init__(self, socket_path: str, *, timeout: float = 30.0) -> None:
        self.socket_path = socket_path
        self.timeout = timeout
        self._lock = threading.Lock()
        self._sock: Optional[socket.socket] = None

    # ------------------------------------------------------------------
    # Connection management
    # ------------------------------------------------------------------

    def _connect(self) -> socket.socket:
        s = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
        s.settimeout(self.timeout)
        try:
            s.connect(self.socket_path)
        except (FileNotFoundError, ConnectionRefusedError) as e:
            s.close()
            raise ToranConnectionError(
                f"toran core not reachable at {self.socket_path}: {e}"
            ) from e
        except OSError as e:
            s.close()
            raise ToranConnectionError(
                f"socket error connecting to {self.socket_path}: {e}"
            ) from e
        return s

    def _ensure(self) -> socket.socket:
        if self._sock is None:
            self._sock = self._connect()
        return self._sock

    def close(self) -> None:
        with self._lock:
            if self._sock is not None:
                try:
                    self._sock.close()
                except OSError:
                    pass
                self._sock = None

    def _reconnect(self) -> None:
        self.close()
        self._sock = self._connect()

    # ------------------------------------------------------------------
    # Wire protocol
    # ------------------------------------------------------------------

    def _send(self, payload: dict) -> dict:
        """Send a JSON-line request and read one JSON-line response."""
        with self._lock:
            data = (json.dumps(payload) + "\n").encode("utf-8")
            last_err: Optional[BaseException] = None
            for attempt in range(2):
                try:
                    s = self._ensure()
                    s.sendall(data)
                    buf = b""
                    while not buf.endswith(b"\n"):
                        chunk = s.recv(65536)
                        if not chunk:
                            raise ToranConnectionError("toran core closed connection")
                        buf += chunk
                    return json.loads(buf.decode("utf-8").strip())
                except (OSError, ToranConnectionError) as e:
                    last_err = e
                    if attempt == 0:
                        # Try to reconnect once.
                        try:
                            self._reconnect()
                        except ToranConnectionError:
                            pass
                        continue
                    break
            assert last_err is not None
            raise ToranConnectionError(f"send failed: {last_err}")

    # ------------------------------------------------------------------
    # High-level operations
    # ------------------------------------------------------------------

    def ping(self) -> bool:
        try:
            r = self._send({"type": "ping"})
            return r.get("type") == "pong"
        except ToranConnectionError:
            return False

    def evaluate(
        self,
        function_name: str,
        args: dict,
        context: dict,
        *,
        agent_id: str,
        session_id: Optional[str] = None,
    ) -> dict:
        """Send a request for evaluation. Returns the server's
        `Decision` message verbatim. If the decision is REQUIRE_APPROVAL
        the message also includes `approval_id` and `notify_token`."""
        return self._send({
            "type": "evaluate",
            "request": {
                "function_name": function_name,
                "args": args,
                "context": context,
            },
            "agent_id": agent_id,
            "session_id": session_id or "",
        })

    def wait_for_approval(
        self,
        approval_id: str,
        token: str,
        timeout_secs: int,
    ) -> str:
        """Block until the approval resolves. Returns one of
        `approved`, `denied`, `timeout`."""
        # The Rust core polls the DB. We use its own blocking call so
        # we don't have to re-implement the wait in Python.
        r = self._send({
            "type": "wait",
            "approval_id": approval_id,
            "token": token,
            "timeout_secs": int(timeout_secs),
        })
        t = r.get("type")
        if t == "approved":
            return "approved"
        if t == "denied":
            return "denied"
        if t == "timeout":
            return "timeout"
        if t == "error":
            raise ToranConnectionError(r.get("message", "unknown error"))
        return "unknown"
