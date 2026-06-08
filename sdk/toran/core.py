"""The @gate decorator.

A wrapped function:
  1. Captures (name, args, kwargs, context, agent_id, session_id).
  2. Asks the Rust core for a decision (ALLOW / BLOCK / REQUIRE_APPROVAL).
  3. On ALLOW, runs the function body and returns its result.
  4. On BLOCK, raises `BlockedError`.
  5. On REQUIRE_APPROVAL, blocks until a human resolves, then either
     runs the function (approved) or raises `DeniedError` /
     `TimeoutError`.

Both sync and async functions are supported. For async functions we
return a coroutine; the caller's `await` triggers the gate.
"""

from __future__ import annotations

import asyncio
import functools
import inspect
import os
import socket
import time
import uuid
from typing import Any, Callable, Optional

from .client import Client
from .config import Config, get_config
from .exceptions import (
    BlockedError,
    ConfigurationError,
    DeniedError,
    TimeoutError as ToranTimeoutError,
    ToranConnectionError,
)


_DEFAULT_CLIENT: Optional[Client] = None
_DEFAULT_LOCK = __import__("threading").Lock()


def _get_client(cfg: Config) -> Client:
    global _DEFAULT_CLIENT
    if _DEFAULT_CLIENT is not None and _DEFAULT_CLIENT.socket_path == cfg.socket_path:
        return _DEFAULT_CLIENT
    with _DEFAULT_LOCK:
        if _DEFAULT_CLIENT is None or _DEFAULT_CLIENT.socket_path != cfg.socket_path:
            _DEFAULT_CLIENT = Client(cfg.socket_path)
    return _DEFAULT_CLIENT


def _snapshot_args(func: Callable, args: tuple, kwargs: dict) -> dict:
    """Best-effort JSON snapshot of positional and keyword args.

    We refuse to serialize non-JSON values rather than silently dropping
    them. Users who need richer types should pre-serialize and pass
    the JSON form.
    """
    import json
    out: dict = {}
    try:
        sig = inspect.signature(func)
        bound = sig.bind_partial(*args, **kwargs)
        bound.apply_defaults()
        for name, val in bound.arguments.items():
            json.dumps(val)  # raises if not serializable
            out[name] = val
    except (TypeError, ValueError):
        # Fall back to a positional + kwargs dump.
        out["__args__"] = list(args)
        for k, v in kwargs.items():
            try:
                json.dumps(v)
                out[k] = v
            except (TypeError, ValueError):
                out[k] = repr(v)
    return out


def _function_name(func: Callable) -> str:
    return func.__qualname__ if hasattr(func, "__qualname__") else func.__name__


def _make_context(cfg: Config) -> dict:
    return {
        "agent_id": cfg.agent_id,
        "session_id": cfg.session_id or str(uuid.uuid4()),
        "ts": time.time(),
    }


def _invoke_decision(
    cfg: Config,
    client: Client,
    function_name: str,
    args: dict,
    context: dict,
) -> dict:
    try:
        return client.evaluate(
            function_name=function_name,
            args=args,
            context=context,
            agent_id=cfg.agent_id,
            session_id=cfg.session_id,
        )
    except ToranConnectionError:
        if cfg.fail_open:
            return {"type": "decision", "decision": {"action": "ALLOW", "rule_name": "<fail-open>"}, "approval_id": None, "notify_token": None}
        raise


def gate(
    *,
    policy: Optional[str] = None,
    timeout_secs: Optional[int] = None,
) -> Callable:
    """Decorator factory.

    Usage:
        @gate()
        def f(...): ...

        @gate(policy="email-guardian", timeout_secs=120)
        async def f(...): ...
    """
    def decorator(func: Callable) -> Callable:
        cfg_at_decorate = get_config()
        func_name = _function_name(func)
        is_coro = inspect.iscoroutinefunction(func)

        if is_coro:
            @functools.wraps(func)
            async def async_wrapper(*args, **kwargs):
                cfg = get_config()
                client = _get_client(cfg)
                snap = _snapshot_args(func, args, kwargs)
                ctx = _make_context(cfg)
                if policy:
                    ctx["policy_hint"] = policy
                resp = _invoke_decision(cfg, client, func_name, snap, ctx)
                decision = resp.get("decision", {})
                action = decision.get("action")
                if action == "BLOCK":
                    raise BlockedError(
                        f"blocked by toran policy `{decision.get('rule_name')}`",
                        function_name=func_name,
                        rule_name=decision.get("rule_name"),
                        risk_score=decision.get("risk_score"),
                    )
                if action == "ALLOW":
                    return await func(*args, **kwargs)
                # REQUIRE_APPROVAL
                approval_id = resp.get("approval_id")
                token = resp.get("notify_token")
                if not approval_id or not token:
                    raise ConfigurationError("toran core returned REQUIRE_APPROVAL without approval_id")
                wait = timeout_secs or decision.get("timeout_secs") or cfg.default_timeout_secs
                outcome = await asyncio.to_thread(
                    client.wait_for_approval, approval_id, token, wait
                )
                if outcome == "approved":
                    return await func(*args, **kwargs)
                if outcome == "denied":
                    raise DeniedError(
                        "denied by reviewer",
                        function_name=func_name,
                        approval_id=approval_id,
                    )
                if outcome == "timeout":
                    raise ToranTimeoutError(
                        f"approval timed out after {wait}s",
                        function_name=func_name,
                        approval_id=approval_id,
                    )
                raise ConfigurationError(f"unknown wait outcome: {outcome}")
            return async_wrapper
        else:
            @functools.wraps(func)
            def sync_wrapper(*args, **kwargs):
                cfg = get_config()
                client = _get_client(cfg)
                snap = _snapshot_args(func, args, kwargs)
                ctx = _make_context(cfg)
                if policy:
                    ctx["policy_hint"] = policy
                resp = _invoke_decision(cfg, client, func_name, snap, ctx)
                decision = resp.get("decision", {})
                action = decision.get("action")
                if action == "BLOCK":
                    raise BlockedError(
                        f"blocked by toran policy `{decision.get('rule_name')}`",
                        function_name=func_name,
                        rule_name=decision.get("rule_name"),
                        risk_score=decision.get("risk_score"),
                    )
                if action == "ALLOW":
                    return func(*args, **kwargs)
                approval_id = resp.get("approval_id")
                token = resp.get("notify_token")
                if not approval_id or not token:
                    raise ConfigurationError("toran core returned REQUIRE_APPROVAL without approval_id")
                wait = timeout_secs or decision.get("timeout_secs") or cfg.default_timeout_secs
                outcome = client.wait_for_approval(approval_id, token, wait)
                if outcome == "approved":
                    return func(*args, **kwargs)
                if outcome == "denied":
                    raise DeniedError(
                        "denied by reviewer",
                        function_name=func_name,
                        approval_id=approval_id,
                    )
                if outcome == "timeout":
                    raise ToranTimeoutError(
                        f"approval timed out after {wait}s",
                        function_name=func_name,
                        approval_id=approval_id,
                    )
                raise ConfigurationError(f"unknown wait outcome: {outcome}")
            return sync_wrapper
    return decorator
