"""Example: gate a LangChain-style tool.

We don't import langchain here (so this example is self-contained),
but the pattern is identical: any class with a `run` / `_run` method
can be wrapped.
"""

import os
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, os.path.abspath(os.path.join(HERE, "..")))

from toran.integrations import ToranTool  # noqa: E402


class MySendEmailTool:
    name = "send_email"
    description = "Send an email to a recipient."

    def _run(self, query: str) -> str:
        return f"sent: {query}"


def main() -> int:
    raw = MySendEmailTool()
    safe = ToranTool(raw, policy="email-guardian")
    print(safe("hello world"))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
