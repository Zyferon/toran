"""Toran: runtime human-approval gatekeeper for AI agents.

Public API:

    from toran import gate, configure, BlockedError, DeniedError, TimeoutError

    @gate()
    def send_email(to, subject, body):
        return mailer.send(to, subject, body)

The decorator intercepts the call, talks to the Rust core over a
local Unix socket, and either runs the function, raises an exception,
or blocks until a human resolves an approval request.
"""

from .config import configure, get_config
from .exceptions import (
    ToranError,
    BlockedError,
    DeniedError,
    TimeoutError,
    ToranConnectionError,
    ConfigurationError,
)
from .core import gate
from .client import Client

__all__ = [
    "gate",
    "configure",
    "get_config",
    "Client",
    "ToranError",
    "BlockedError",
    "DeniedError",
    "TimeoutError",
    "ToranConnectionError",
    "ConfigurationError",
]

__version__ = "0.1.0"
