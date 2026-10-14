"""Bounded live libtest stream observations, diagnostic only; never acceptance."""
import re
import time


class TestTiming:
    def __init__(self, cases):
        if not isinstance(cases, (list, tuple)) or not 1 <= len(cases) <= 256:
            raise ValueError('bounded diagnostic case roster')
        if any(type(case) is not str or re.fullmatch(r'[A-Za-z0-9_:]{1,256}', case) is None for case in cases):
            raise ValueError('diagnostic case name')
        self.expected = set(cases)
        self.buffer = b''; self.discard = False
        self.rows = {}; self.unknown = set(); self.diagnostic_error = None

    def __call__(self, stream, block):
        if stream != 'stdout': return
        if type(block) is not bytes or len(block) > 65536:
            self.diagnostic_error = 'diagnostic chunk bound'; return
        for piece in block.splitlines(keepends=True):
            ended = piece.endswith(b'\n')
            if len(self.buffer) + len(piece) > 4096:
                self.buffer = b''; self.discard = True
                self.diagnostic_error = 'diagnostic line bound'
            if not self.discard:
                self.buffer += piece
                match = re.match(rb'^test ([A-Za-z0-9_:]{1,256}) \.\.\. ', self.buffer)
                if match:
                    case = match[1].decode('ascii')
                    now = time.monotonic_ns()
                    if case not in self.expected:
                        if len(self.unknown) < 256: self.unknown.add(case)
                    else:
                        row = self.rows.setdefault(case, {'case': case, 'start_monotonic_ns': now,
                                                         'end_monotonic_ns': None, 'outcome': 'incomplete'})
                        result = re.fullmatch(rb'(ok|FAILED|ignored)\r?\n', self.buffer[match.end():])
                        if result:
                            if row['end_monotonic_ns'] is not None:
                                self.diagnostic_error = 'duplicate diagnostic case'
                            else:
                                row['end_monotonic_ns'] = now; row['outcome'] = result[1].decode('ascii')
            if ended: self.buffer = b''; self.discard = False

    def report(self):
        rows = []
        for row in self.rows.values():
            row = dict(row)
            row['elapsed_seconds'] = None if row['end_monotonic_ns'] is None else (
                row['end_monotonic_ns'] - row['start_monotonic_ns']) / 1e9
            rows.append(row)
        return {'scope': 'observed_stream_prefix_to_completion', 'acceptance': False,
                'cases': rows, 'unknown_cases': sorted(self.unknown),
                'unobserved_cases': sorted(self.expected - self.rows.keys()),
                'diagnostic_error': self.diagnostic_error}
