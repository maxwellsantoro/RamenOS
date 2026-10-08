#!/usr/bin/env python3
"""Run one real command and retain an atomic monotonic timing record.

The record is instrumentation, not an acceptance receipt. Command output remains
on the original streams. Environment values are never copied into records.
"""
import argparse
from datetime import datetime, timezone
import json
import os
from pathlib import Path
import re
import secrets
import signal
import subprocess
import time


def main():
    parser = argparse.ArgumentParser(__doc__)
    parser.add_argument('--directory', type=Path, default=Path('out/foundry/timings'))
    parser.add_argument('--label', required=True)
    parser.add_argument('command', nargs=argparse.REMAINDER)
    args = parser.parse_args()
    if not re.fullmatch(r'[A-Za-z0-9][A-Za-z0-9_.-]{0,127}', args.label):
        parser.error('label must be a bounded stage identifier')
    command = args.command[1:] if args.command[:1] == ['--'] else args.command
    if not command or len(command) > 256 or any(len(a) > 8192 for a in command):
        parser.error('a bounded command is required')
    args.directory.mkdir(parents=True, exist_ok=True)
    row = {'schema_version': 1, 'label': args.label, 'argv': command,
           'started_utc': datetime.now(timezone.utc).isoformat(),
           'process_id': None, 'child_returncode': None, 'wait_reaped': False,
           'cleanup_uncertain': False}
    started = time.monotonic()
    process = None
    interrupted = None
    old_handlers = {}

    def forward(signum, _frame):
        nonlocal interrupted
        interrupted = signum
        if process is not None and process.poll() is None:
            # Signal only the new session created for this command.
            try:
                os.killpg(process.pid, signum)
            except ProcessLookupError:
                pass

    try:
        for signum in (signal.SIGINT, signal.SIGTERM):
            old_handlers[signum] = signal.signal(signum, forward)
        process = subprocess.Popen(command, start_new_session=True)
        row['process_id'] = process.pid
        if interrupted is not None:
            forward(interrupted, None)
        while True:
            try:
                code = process.wait(timeout=1)
                break
            except subprocess.TimeoutExpired:
                if interrupted is not None:
                    try:
                        code = process.wait(timeout=10)
                    except subprocess.TimeoutExpired:
                        try:
                            os.killpg(process.pid, signal.SIGKILL)
                        except ProcessLookupError:
                            pass
                        try:
                            code = process.wait(timeout=2)
                        except subprocess.TimeoutExpired:
                            code = None
                            row['cleanup_uncertain'] = True
                    break
        row['child_returncode'] = code
        row['wait_reaped'] = code is not None
        result = 128 + interrupted if interrupted is not None else (1 if code is None else code if code >= 0 else 128 - code)
    except OSError as error:
        row['error'] = str(error)
        row['cleanup_uncertain'] = process is not None and not row['wait_reaped']
        result = 127
    finally:
        for signum, handler in old_handlers.items():
            signal.signal(signum, handler)
    row.update(elapsed_seconds=time.monotonic() - started, exit_code=result,
               status='PASS' if result == 0 else 'FAIL')
    identity = args.label + '-' + secrets.token_hex(16)
    temporary = args.directory / (identity + '.tmp')
    destination = args.directory / (identity + '.json')
    fd = os.open(temporary, os.O_WRONLY | os.O_CREAT | os.O_EXCL, 0o600)
    with os.fdopen(fd, 'w') as output:
        json.dump(row, output, sort_keys=True)
        output.write('\n')
        output.flush()
        os.fsync(output.fileno())
    os.replace(temporary, destination)
    print(f"FOUNDRY_TIMING: {args.label} status={row['status']} elapsed={row['elapsed_seconds']:.3f}s record={destination}", flush=True)
    return result


if __name__ == '__main__':
    raise SystemExit(main())
