"""Toran framework integrations.

These wrappers make `@gate` transparent when you cannot decorate a
function (third-party classes, framework-managed tools, etc.).

For the common cases we provide:
  * `wrap_function(func, policy=...)`         - any callable
  * `ToranTool(tool, policy=...)`             - LangChain-style tool
  * `wrap_crewai_tool(tool, policy=...)`      - CrewAI
  * `wrap_pydantic_ai_tool(func, policy=...)` - Pydantic AI function tool
  * `wrap_autogen_function(func, policy=...)` - AutoGen register_function
"""

from __future__ import annotations

import asyncio
import functools
import inspect
from typing import Any, Callable, Optional

from .config import get_config
from .core import gate


def wrap_function(func: Callable, *, policy: Optional[str] = None) -> Callable:
    """Wrap an existing function with the @gate decorator.

    Useful for third-party functions you cannot edit.
    """
    return (gate(policy=policy) if policy else gate())(func)


class ToranTool:
    """A drop-in wrapper for LangChain `Tool` (and similar) objects.

    Usage:
        from langchain.tools import MoveFileTool
        from toran.integrations import ToranTool
        agent_tool = ToranTool(MoveFileTool(), policy="filesystem-guardian")
        # Pass `agent_tool` to your agent instead of `MoveFileTool()`.
    """

    def __init__(self, tool: Any, *, policy: Optional[str] = None, timeout_secs: Optional[int] = None) -> None:
        self._tool = tool
        self._policy = policy
        self._timeout = timeout_secs
        # Mirror attributes callers usually read.
        self.name = getattr(tool, "name", tool.__class__.__name__)
        self.description = getattr(tool, "description", "")

    def __getattr__(self, item: str) -> Any:
        return getattr(self._tool, item)

    def __call__(self, *args, **kwargs):
        @gate(policy=self._policy, timeout_secs=self._timeout)
        def _invoke(query: str = "") -> Any:
            if hasattr(self._tool, "_run"):
                return self._tool._run(query=query, *args, **kwargs)
            if hasattr(self._tool, "run"):
                return self._tool.run(query=query, *args, **kwargs)
            return self._tool(*args, **kwargs)
        return _invoke(*args, **kwargs)

    async def arun(self, *args, **kwargs):
        @gate(policy=self._policy, timeout_secs=self._timeout)
        async def _ainvoke(query: str = "") -> Any:
            if hasattr(self._tool, "_arun"):
                return await self._tool._arun(query=query, *args, **kwargs)
            loop = asyncio.get_event_loop()
            return await loop.run_in_executor(None, lambda: self.__call__(*args, **kwargs))
        return await _ainvoke(*args, **kwargs)


def wrap_crewai_tool(tool: Any, *, policy: Optional[str] = None) -> Any:
    """Wrap a CrewAI BaseTool.

    CrewAI exposes a `run` method (sync) and may have `_run`. We hook
    into whichever exists.
    """
    return ToranTool(tool, policy=policy)


def wrap_pydantic_ai_tool(func: Callable, *, policy: Optional[str] = None) -> Callable:
    """Wrap a Pydantic AI function-style tool.

    Pydantic AI tools are plain functions or methods; decoration is
    identical to a normal function.
    """
    return wrap_function(func, policy=policy)


def wrap_autogen_function(func: Callable, *, policy: Optional[str] = None) -> Callable:
    """Wrap an AutoGen function for `register_function`.

    AutoGen calls the function with kwargs; `wrap_function` handles
    that case naturally.
    """
    return wrap_function(func, policy=policy)


# Re-export gate at this level for convenience.
__all__ = [
    "wrap_function",
    "ToranTool",
    "wrap_crewai_tool",
    "wrap_pydantic_ai_tool",
    "wrap_autogen_function",
]
