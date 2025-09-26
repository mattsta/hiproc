"""Command-line entry point for the hiproc server."""

import argparse

import uvicorn


def main():
    """Starts the uvicorn server for the hiproc FastAPI application."""
    parser = argparse.ArgumentParser(description="Run the hiproc server.")
    parser.add_argument(
        "--host", type=str, default="127.0.0.1", help="The host to bind to."
    )
    parser.add_argument("--port", type=int, default=8128, help="The port to run on.")
    parser.add_argument(
        "--dev",
        "--reload",
        action="store_true",
        help="Enable development mode with auto-reload (higher CPU usage).",
    )
    args = parser.parse_args()

    uvicorn.run("hiproc.main:app", host=args.host, port=args.port, reload=args.dev)


if __name__ == "__main__":
    main()
