"""Command-line entry point for merit_ledger."""

from __future__ import annotations

import argparse

from merit_ledger.__about__ import __version__


def main() -> None:
    """Run the merit_ledger CLI."""
    parser = argparse.ArgumentParser(
        prog="merit_ledger",
        description="Keep track of your merit. A Chinese Mahayana Buddhist practice from 1000 years ago.",  # pylint: disable=line-too-long
    )
    parser.add_argument(
        "--version",
        action="version",
        version=f"%(prog)s {__version__}",
    )
    parser.add_argument(
        "--backend-only",
        action="store_true",
        help="Start the backend and wait for health, then exit (no window).",
    )
    args = parser.parse_args()

    from merit_ledger import app

    raise SystemExit(app.run(with_ui=not args.backend_only))


if __name__ == "__main__":
    main()
