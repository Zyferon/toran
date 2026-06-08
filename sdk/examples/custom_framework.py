"""Example: wrap a custom function without modifying it.

Use `wrap_function` when the function lives in a third-party library
and you can't add a decorator.
"""

import os
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, os.path.abspath(os.path.join(HERE, "..")))

from toran.integrations import wrap_function  # noqa: E402
from toran import gate  # noqa: E402


def transfer_money(amount: int, currency: str = "USD") -> str:
    return f"transferred {amount} {currency}"


# Equivalent decorators:
gated1 = gate()(transfer_money)
gated2 = wrap_function(transfer_money, policy="financial-guardian")

print(gated1(50, "USD"))      # allowed (small)
print(gated2(50_000, "USD"))  # requires approval
