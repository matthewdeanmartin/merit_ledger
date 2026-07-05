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
    # TODO: add subcommands here
    args = parser.parse_args()
    _ = args  # remove once subcommands are added


if __name__ == "__main__":
    main()
