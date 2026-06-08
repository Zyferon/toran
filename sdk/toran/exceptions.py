"""Custom exceptions raised by the Toran SDK.

Users should catch these to handle gate outcomes gracefully:

    try:
        send_email(...)
    except BlockedError:
        ... # policy forbids this call
    except DeniedError:
        ... # a human reviewer denied
    except TimeoutError:
        ... # no one answered in time
"""

from __future__ import annotations

from typing import Any, Optional


class ToranError(Exception):
    """Base class for all Toran errors."""


class BlockedError(ToranError):
    """The function was blocked by policy. The agent can catch this
    and try an alternative."""

    def __init__(
        self,
        message: str = "blocked by toran policy",
        *,
        function_name: Optional[str] = None,
        rule_name: Optional[str] = None,
        risk_score: Optional[int] = None,
    ) -> None:
        super().__init__(message)
        self.function_name = function_name
        self.rule_name = rule_name
        self.risk_score = risk_score


class DeniedError(ToranError):
    """A human reviewer explicitly denied the request."""

    def __init__(
        self,
        message: str = "denied by reviewer",
        *,
        function_name: Optional[str] = None,
        approval_id: Optional[str] = None,
        resolved_by: Optional[str] = None,
    ) -> None:
        super().__init__(message)
        self.function_name = function_name
        self.approval_id = approval_id
        self.resolved_by = resolved_by


class TimeoutError(ToranError):  # noqa: A001
    """No human responded within the configured timeout window."""

    def __init__(
        self,
        message: str = "approval timed out",
        *,
        function_name: Optional[str] = None,
        approval_id: Optional[str] = None,
    ) -> None:
        super().__init__(message)
        self.function_name = function_name
        self.approval_id = approval_id

    # Python 3.11+ wants this to be a real TimeoutError alias.
    @property
    def timeout(self) -> Optional[float]:
        return None


# Also expose a non-shadowing alias for callers that prefer it.
ToranTimeoutError = TimeoutError


class ToranConnectionError(ToranError):
    """The Rust core is not reachable. The agent should fall back to
    safe mode or fail closed."""


class ConfigurationError(ToranError):
    """Toran was mis-configured (missing socket path, bad policy, ...)."""
