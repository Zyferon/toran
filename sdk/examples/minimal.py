"""Minimal Toran example.

Run the core first:
    $ cargo run -- start

Then in another terminal:
    $ TORAN_SOCKET_PATH=/tmp/toran.sock python3 examples/minimal.py
"""

import os
import sys

# Allow `python3 examples/minimal.py` to import the local SDK.
HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, os.path.abspath(os.path.join(HERE, "..")))

from toran import gate, configure, BlockedError, TimeoutError, DeniedError  # noqa: E402

configure(socket_path=os.environ.get("TORAN_SOCKET_PATH", "/tmp/toran.sock"))


@gate()
def read_file(path: str) -> str:
    """Always allowed by the default policy."""
    with open(path) as f:
        return f.read()


@gate()
def delete_user(user_id: int) -> bool:
    """Always requires human approval under the example policy."""
    print(f"(would delete user {user_id})")
    return True


def main() -> int:
    try:
        # If you have a file to read, do it. Otherwise, just exercise the gate.
        result = read_file("/etc/hostname")
        print(f"hostname: {result.strip()}")
    except BlockedError as e:
        print(f"BLOCKED: {e}")
    except Exception as e:
        print(f"error: {type(e).__name__}: {e}")

    print("\nNow asking for human approval to delete user 42...")
    try:
        delete_user(42)
        print("user deleted (a human approved)")
    except DeniedError as e:
        print(f"DENIED: {e}")
    except TimeoutError as e:
        print(f"TIMEOUT: {e}")
    except BlockedError as e:
        print(f"BLOCKED: {e}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
