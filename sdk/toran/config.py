"""Toran configuration.

Settings come from three sources, in order of increasing priority:
  1. Built-in defaults.
  2. Environment variables (TORAN_SOCKET_PATH, TORAN_AGENT_ID, ...).
  3. Explicit calls to `toran.configure(...)` from user code.

The configured singleton is read by the `@gate` decorator at decoration
time. We snapshot the config inside the wrapper to make behaviour
deterministic even if `configure` is called later.
"""

from __future__ import annotations

import os
import threading
from dataclasses import dataclass, field, replace
from typing import Optional


@dataclass(frozen=True)
class Config:
    socket_path: str = "/tmp/toran.sock"
    agent_id: str = "agent-default"
    session_id: Optional[str] = None
    # If true, on connection failure the SDK fails OPEN (calls the
    # original function). If false, it fails CLOSED (raises
    # ToranConnectionError). Default fail-closed is safer.
    fail_open: bool = False
    # Default timeout (seconds) if a policy does not specify one.
    default_timeout_secs: int = 300
    # Optional: explicit policy name hint (informational only; the
    # Rust core owns the actual policy set).
    policy_hint: Optional[str] = None
    # API base URL (used for the optional HTTP fallback path).
    api_base: str = "http://127.0.0.1:7878"
    prefer_http: bool = False
    extra: dict = field(default_factory=dict)


_LOCK = threading.Lock()
_CURRENT: Config = Config()


def _from_env() -> Config:
    cfg = Config()
    if v := os.environ.get("TORAN_SOCKET_PATH"):
        cfg = replace(cfg, socket_path=v)
    if v := os.environ.get("TORAN_AGENT_ID"):
        cfg = replace(cfg, agent_id=v)
    if v := os.environ.get("TORAN_SESSION_ID"):
        cfg = replace(cfg, session_id=v)
    if v := os.environ.get("TORAN_FAIL_OPEN"):
        cfg = replace(cfg, fail_open=v.lower() in ("1", "true", "yes"))
    if v := os.environ.get("TORAN_DEFAULT_TIMEOUT"):
        try:
            cfg = replace(cfg, default_timeout_secs=int(v))
        except ValueError:
            pass
    if v := os.environ.get("TORAN_API_BASE"):
        cfg = replace(cfg, api_base=v)
    if v := os.environ.get("TORAN_PREFER_HTTP"):
        cfg = replace(cfg, prefer_http=v.lower() in ("1", "true", "yes"))
    return cfg


def configure(**kwargs) -> Config:
    """Update the global Toran configuration. Thread-safe.

    Example:
        import toran
        toran.configure(socket_path="/var/run/toran.sock",
                        agent_id="prod-agent-7",
                        fail_open=False)
    """
    global _CURRENT
    with _LOCK:
        base = _from_env()
        merged = replace(base, **kwargs)
        _CURRENT = merged
        return merged


def get_config() -> Config:
    """Return the current effective configuration (a snapshot)."""
    with _LOCK:
        return _CURRENT
