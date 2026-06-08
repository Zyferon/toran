"""Unit tests for the Python SDK that do not require a running core.

We mock the Client to verify the decorator routes the right calls and
raises the right exceptions.
"""

import asyncio
import sys
import os
from unittest import mock

# Add the local sdk/ to sys.path so `import toran` works without pip install.
HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, os.path.abspath(os.path.join(HERE, "..", "..")))

from sdk.toran import (  # noqa: E402
    gate, configure, get_config,
    BlockedError, DeniedError, TimeoutError as ToranTimeoutError,
)
from sdk.toran.client import Client  # noqa: E402


def _client_returning(payload):
    c = mock.MagicMock(spec=Client)
    c.socket_path = "/tmp/toran.sock"
    c.evaluate = mock.MagicMock(return_value=payload)
    c.wait_for_approval = mock.MagicMock(return_value="approved")
    return c


def test_gate_calls_original_on_allow(monkeypatch):
    configure(socket_path="/tmp/toran.sock", agent_id="a", session_id="s")
    payload = {"type": "decision", "decision": {"action": "ALLOW", "rule_name": "r", "risk_score": 10, "timeout_secs": 60, "elapsed_ns": 100}, "approval_id": None, "notify_token": None}
    fake = _client_returning(payload)
    monkeypatch.setattr("sdk.toran.core._get_client", lambda cfg: fake)
    @gate()
    def add(a, b):
        return a + b
    assert add(2, 3) == 5
    fake.evaluate.assert_called_once()


def test_gate_raises_blocked(monkeypatch):
    configure(socket_path="/tmp/toran.sock", agent_id="a", session_id="s")
    payload = {"type": "decision", "decision": {"action": "BLOCK", "rule_name": "no_email", "risk_score": 100, "timeout_secs": 60, "elapsed_ns": 100}}
    fake = _client_returning(payload)
    monkeypatch.setattr("sdk.toran.core._get_client", lambda cfg: fake)
    @gate()
    def f():
        return "should not run"
    try:
        f()
    except BlockedError as e:
        assert e.rule_name == "no_email"
        assert e.function_name.endswith("f")
    else:
        raise AssertionError("BlockedError not raised")


def test_gate_waits_then_runs_on_approval(monkeypatch):
    configure(socket_path="/tmp/toran.sock", agent_id="a", session_id="s")
    payload = {
        "type": "decision",
        "decision": {"action": "REQUIRE_APPROVAL", "rule_name": "wire", "risk_score": 95, "timeout_secs": 30, "elapsed_ns": 200},
        "approval_id": "abc",
        "notify_token": "tok",
    }
    fake = _client_returning(payload)
    fake.wait_for_approval = mock.MagicMock(return_value="approved")
    monkeypatch.setattr("sdk.toran.core._get_client", lambda cfg: fake)
    @gate()
    def send_money(amount):
        return f"sent {amount}"
    assert send_money(100) == "sent 100"
    fake.wait_for_approval.assert_called_once()


def test_gate_raises_denied(monkeypatch):
    configure(socket_path="/tmp/toran.sock", agent_id="a", session_id="s")
    payload = {
        "type": "decision",
        "decision": {"action": "REQUIRE_APPROVAL", "rule_name": "wire", "risk_score": 95, "timeout_secs": 30, "elapsed_ns": 200},
        "approval_id": "abc",
        "notify_token": "tok",
    }
    fake = _client_returning(payload)
    fake.wait_for_approval = mock.MagicMock(return_value="denied")
    monkeypatch.setattr("sdk.toran.core._get_client", lambda cfg: fake)
    @gate()
    def f():
        return "no"
    try:
        f()
    except DeniedError as e:
        assert e.approval_id == "abc"
    else:
        raise AssertionError("DeniedError not raised")


def test_gate_raises_timeout(monkeypatch):
    configure(socket_path="/tmp/toran.sock", agent_id="a", session_id="s")
    payload = {
        "type": "decision",
        "decision": {"action": "REQUIRE_APPROVAL", "rule_name": "wire", "risk_score": 95, "timeout_secs": 1, "elapsed_ns": 200},
        "approval_id": "abc",
        "notify_token": "tok",
    }
    fake = _client_returning(payload)
    fake.wait_for_approval = mock.MagicMock(return_value="timeout")
    monkeypatch.setattr("sdk.toran.core._get_client", lambda cfg: fake)
    @gate()
    def f():
        return "no"
    try:
        f()
    except ToranTimeoutError as e:
        assert e.approval_id == "abc"
    else:
        raise AssertionError("TimeoutError not raised")


def test_async_gate(monkeypatch):
    configure(socket_path="/tmp/toran.sock", agent_id="a", session_id="s")
    payload = {"type": "decision", "decision": {"action": "ALLOW", "rule_name": "r", "risk_score": 10, "timeout_secs": 60, "elapsed_ns": 100}}
    fake = _client_returning(payload)
    monkeypatch.setattr("sdk.toran.core._get_client", lambda cfg: fake)
    @gate()
    async def af(x):
        return x * 2
    out = asyncio.run(af(5))
    assert out == 10
