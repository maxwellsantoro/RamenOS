#!/usr/bin/env python3
"""A2.4 real contained shell consumer and independent durable-state assertions."""

import base64
import hashlib
import json
import os
from pathlib import Path
import socket
import subprocess
import sys
import tempfile
import unittest
from unittest.mock import patch
from linux_sandbox import SandboxFailure
from lt_backend import TaskError
from ls_transactions import LinuxShellTask

FIXTURE = Path(os.environ["RAMEN_TASK_LS_FIXTURE"])
WORKER = Path(os.environ["RAMEN_TASK_VALIDATOR_WORKER"])
OBSERVATIONS = []

# This runs inside the agent container and uses only files/shell commands.
WORKFLOW = """import json,subprocess
from pathlib import Path
b=json.loads(Path('/task/bootstrap.json').read_bytes())
def call(n,verb,*args,expected='ok'):
 p=subprocess.run(['/task/taskctl','--request-id',str(n),verb,*args],capture_output=True,text=True)
 r=json.loads(p.stdout)
 assert r['status']==expected,(verb,r,p.stderr)
 assert p.returncode==(0 if expected=='ok' else 1)
 return r['result']
cap=call(1,'grant','--policy-cap',b['policy_cap'],'--rights','read,stage,validate,commit,observe')['task_cap']
state=call(2,'state','--task-cap',cap)['state']
call(3,'read','--task-cap',cap,'--resource',b['resources'][0]['resource'],'--output','/candidate/config.json')
config=json.loads(Path('/candidate/config.json').read_bytes())
schema=json.loads(Path('/inputs/schema.json').read_bytes())
config['enabled']=schema['enabled']
Path('/candidate/config.json').write_text(json.dumps(config,sort_keys=True,separators=(',',':')))
staged=call(4,'stage','--task-cap',cap,'--file','/candidate/config.json')
commit=['--task-cap',cap,'--candidate-cap',staged['candidate_cap'],'--expected-revision',state['revision'],'--expected-content-id',state['content_id']]
call(5,'commit',*commit,expected='validation_failed')
call(6,'validate','--task-cap',cap,'--candidate-cap',staged['candidate_cap'],'--validator-id','sha256:'+'0'*64,expected='denied')
v=call(7,'validate','--task-cap',cap,'--candidate-cap',staged['candidate_cap'],'--validator-id',state['validator_id'])
assert v['outcome']=='valid'
# Client changes and forged results cannot alter the sealed candidate.
Path('/candidate/config.json').write_text('{}')
Path('/candidate/validator-result.json').write_text('{"outcome":"valid"}')
receipt=call(8,'commit',*commit)
assert receipt['revision']=='1' and receipt['content_id']==staged['content_id']
assert call(8,'commit',*commit)==receipt
call(8,'commit',*commit[:-4],'--expected-revision','1','--expected-content-id',state['content_id'],expected='request_reuse')
lookup=call(9,'receipt','--task-cap',cap,'--commit-request-id','8')
assert lookup['content_id']==receipt['content_id']
call(10,'read','--task-cap',cap,'--resource','resource:00000000000003e7',expected='denied')
readcap=call(11,'grant','--policy-cap',b['policy_cap'],'--rights','read')['task_cap']
call(12,'stage','--task-cap',readcap,'--file','/candidate/config.json',expected='denied')
call(13,'receipt','--task-cap',readcap,'--commit-request-id','8',expected='denied')
call(14,'revoke','--policy-cap',b['policy_cap'],'--task-cap',cap)
call(15,'state','--task-cap',cap,expected='denied')
Path('/candidate/retry.json').write_text(json.dumps({'commit':commit,'receipt':receipt}))
print(json.dumps({'receipt':receipt,'sealed':staged['content_id'],'client_changed':Path('/candidate/config.json').read_text()=='{}'}))
"""


class ShellTests(unittest.TestCase):
    def test_shell_transactions_restart_and_boundary(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            private = root / "private-canary"
            private.write_bytes(os.urandom(32))
            original = private.read_bytes()
            with LinuxShellTask(
                FIXTURE, root / "journal", root / "candidate", WORKER
            ) as session:
                result = session.run(["python3", "-c", WORKFLOW], wall_ms=20000)
                answer = json.loads(result.stdout)
                self.assertEqual(session.task.state["revision"], 1)
                accepted = session.task.load(session.task.state["content_id"])
                self.assertEqual(
                    json.loads(accepted),
                    json.loads((FIXTURE / "schema.json").read_bytes()),
                )
                self.assertEqual(
                    answer["sealed"], "sha256:" + hashlib.sha256(accepted).hexdigest()
                )
                self.assertTrue(answer["client_changed"])
                self.assertTrue(
                    all(run["removed"] for run in session.task.state["runs"])
                )
                # Check real shell access to protected broker/journal/helper and host paths.
                probe = """import json,os,socket
from pathlib import Path
checks={}
for path,mode in [('/task/bootstrap.json','wb'),('/task/taskctl','wb'),('/task/socket','wb'),('/inputs/schema.json','wb'),('/store/journal.json','rb'),('/workspace/B/config.json','rb'),(__PRIVATE__,'rb'),('/var/run/docker.sock','rb')]:
 try:
  with open(path,mode) as f: f.write(b'forged') if mode=='wb' else f.read(1)
 except OSError: checks[path]='denied'
 else: raise AssertionError(path)
# Raw broker packets cannot supply a domain or host path.
b=json.loads(Path('/task/bootstrap.json').read_bytes())
s=socket.socket(socket.AF_UNIX); s.connect('/task/socket')
s.sendall(json.dumps({'schema_version':1,'request_id':'20','call':{'operation':'request_grant','policy_cap':b['policy_cap'],'task_id':'17','resource':'resource:0000000000000001','rights':['read'],'lifetime_ms':100,'domain_id':7}}).encode()+bytes([10]))
r=json.loads(s.makefile('rb').readline()); assert r['status']=='invalid' and r['result'] is None
print(json.dumps(checks))
""".replace("__PRIVATE__", repr(str(private)))
                checked = session.run(["python3", "-c", probe], wall_ms=10000)
                self.assertEqual(len(json.loads(checked.stdout)), 8)
                self.assertEqual(private.read_bytes(), original)
                first = session.evidence()
            with LinuxShellTask(
                FIXTURE, root / "journal", root / "candidate", WORKER
            ) as session:
                retry = """import json,subprocess
from pathlib import Path
b=json.loads(Path('/task/bootstrap.json').read_bytes()); old=json.loads(Path('/candidate/retry.json').read_bytes())
def run(n,verb,*args):
 p=subprocess.run(['/task/taskctl','--request-id',str(n),verb,*args],capture_output=True,text=True); r=json.loads(p.stdout); assert p.returncode==0,(r,p.stderr); return r['result']
cap=run(21,'grant','--policy-cap',b['policy_cap'],'--rights','commit')['task_cap']
args=old['commit']; args[1]=cap
assert run(8,'commit',*args)==old['receipt']
assert run(22,'receipt','--task-cap',cap,'--commit-request-id','8')['revision']=='1'
print('recovered')
"""
                session.run(["python3", "-c", retry], wall_ms=10000)
                self.assertEqual(session.task.state["revision"], 1)
                OBSERVATIONS.append(
                    {
                        "first": first,
                        "restarted": session.evidence(),
                        "accepted": answer["sealed"],
                    }
                )

    def test_transport_peers_frames_descriptors_and_cleanup_checkpoint(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            with LinuxShellTask(
                FIXTURE, root / "journal", root / "candidate", WORKER
            ) as session:
                baseline = len(list(Path("/proc/self/fd").iterdir()))
                # An ordinary host process has the wrong OS identity even if it
                # knows the endpoint. No model capability is disclosed.
                with socket.socket(socket.AF_UNIX) as connection:
                    connection.settimeout(3)
                    connection.connect(str(session.endpoint / "socket"))
                    response = json.loads(connection.makefile("rb").readline())
                    self.assertEqual(
                        (response["status"], response["result"]), ("denied", None)
                    )
                probe = r"""import array,json,os,socket,time
from pathlib import Path
b=json.loads(Path('/task/bootstrap.json').read_bytes())
packets=[b'{"schema_version":1,"schema_version":1}', b'['*2000+b'0'+b']'*2000,b'{}\n{}',b' '*131073]
results=[]
for packet in packets:
 with socket.socket(socket.AF_UNIX) as s:
  s.settimeout(3); s.connect('/task/socket'); s.sendall(packet+bytes([10]))
  try: result=json.loads(s.makefile('rb').readline()); assert result['status']=='invalid'
  except (OSError,ValueError): pass
 results.append('rejected')
# Reading a partial frame has a whole-request deadline, not one timeout per byte.
start=time.monotonic()
with socket.socket(socket.AF_UNIX) as s:
 s.settimeout(3); s.connect('/task/socket'); s.sendall(b'{')
 r=json.loads(s.makefile('rb').readline()); assert r['status']=='invalid'
assert time.monotonic()-start<2.5
fd=os.open('/inputs/schema.json',os.O_RDONLY)
try:
 with socket.socket(socket.AF_UNIX) as s:
  s.settimeout(3); s.connect('/task/socket')
  request={'schema_version':1,'request_id':'40','call':{'operation':'get_task_state','task_cap':'cap:000000000000ffff'}}
  s.sendmsg([json.dumps(request).encode()+bytes([10])],[(socket.SOL_SOCKET,socket.SCM_RIGHTS,array.array('i',[fd]*8))])
  r=json.loads(s.makefile('rb').readline()); assert r['status']=='denied'
finally: os.close(fd)
print(json.dumps({'malformed':results,'stalled':'rejected','foreign_descriptor':'no_authority'}))
"""
                checked = session.run(["python3", "-c", probe], wall_ms=10000)
                self.assertEqual(len(json.loads(checked.stdout)["malformed"]), 4)
                self.assertEqual(len(list(Path("/proc/self/fd").iterdir())), baseline)
                nonzero = session.run(
                    ["/bin/sh", "-c", "printf failed; printf diagnostic >&2; exit 7"]
                )
                self.assertEqual(
                    (nonzero.stdout, nonzero.stderr, nonzero.evidence["exit_code"]),
                    (b"failed", b"diagnostic", 7),
                )
                self.assertEqual(session.task.state["revision"], 0)
                OBSERVATIONS.append(
                    {
                        "transport": session.evidence(),
                        "descriptor_inventory_stable": True,
                    }
                )
                failure = SandboxFailure(
                    "creation_not_confirmed",
                    {"created": False, "removed": True, "injected": True},
                )
                with patch.object(session.sandbox, "run", side_effect=failure):
                    with self.assertRaises(SandboxFailure):
                        session.run(["/bin/true"])
                self.assertTrue(session.transport_poison)
                OBSERVATIONS.append({"injected_cleanup_fault": session.evidence()})
            with self.assertRaises(TaskError):
                LinuxShellTask(FIXTURE, root / "journal", root / "candidate", WORKER)

    def test_standalone_launcher_and_lost_reply_recovery(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            command = r"""python3 - <<'INNER'
import json,socket,subprocess
from pathlib import Path
b=json.loads(Path('/task/bootstrap.json').read_bytes())
def run(n,verb,*args):
 p=subprocess.run(['/task/taskctl','--request-id',str(n),verb,*args],capture_output=True,text=True)
 r=json.loads(p.stdout); assert p.returncode==0,(r,p.stderr); return r['result']
cap=run(1,'grant','--policy-cap',b['policy_cap'],'--rights','read,stage,validate,commit,observe')['task_cap']
state=run(2,'state','--task-cap',cap)['state']
Path('/candidate/config.json').write_bytes(Path('/inputs/schema.json').read_bytes())
candidate=run(3,'stage','--task-cap',cap,'--file','/candidate/config.json')['candidate_cap']
assert run(4,'validate','--task-cap',cap,'--candidate-cap',candidate,'--validator-id',state['validator_id'])['outcome']=='valid'
call={'operation':'commit_candidate','task_cap':cap,'candidate_cap':candidate,'expected_revision':'0','expected_content_id':state['content_id']}
with socket.socket(socket.AF_UNIX) as s:
 s.connect('/task/socket'); s.sendall(json.dumps({'schema_version':1,'request_id':'55','call':call}).encode()+bytes([10]))
 # Deliberately abandon the response; receipt lookup does not retry the effect.
 s.shutdown(socket.SHUT_WR)
receipt=run(5,'receipt','--task-cap',cap,'--commit-request-id','55')
assert receipt['revision']=='1'
Path('/candidate/receipt.json').write_text(json.dumps(receipt))
print(json.dumps(receipt))
INNER"""
            launcher = Path(__file__).with_name("ls_launcher.py")
            args = [
                sys.executable,
                str(launcher),
                "--fixture",
                str(FIXTURE),
                "--store",
                str(root / "journal"),
                "--candidate",
                str(root / "candidate"),
                "--worker",
                str(WORKER),
                "--wall-ms",
                "15000",
            ]
            requests = [
                {"schema_version": 1, "request_id": "1", "command": command},
                {
                    "schema_version": 1,
                    "request_id": "2",
                    "command": "printf second; exit 7",
                },
                {
                    "schema_version": 1,
                    "request_id": "3",
                    "command": "true",
                    "wall_ms": 35000,
                },
            ]
            result = subprocess.run(
                args,
                input=b"".join(json.dumps(r).encode() + bytes([10]) for r in requests),
                capture_output=True,
                timeout=30,
            )
            self.assertEqual(result.returncode, 0, result.stderr)
            self.assertEqual(result.stderr, b"")
            records = [json.loads(line) for line in result.stdout.splitlines()]
            self.assertEqual(records[0]["interface"], "scoped_shell")
            receipt = json.loads(base64.b64decode(records[1]["stdout_base64"]))
            self.assertEqual(receipt["revision"], "1")
            self.assertEqual(records[2]["exit_code"], 7)
            self.assertEqual(records[3]["status"], "invalid")
            oversized = subprocess.run(
                args, input=b" " * 8193 + bytes([10]), capture_output=True, timeout=10
            )
            self.assertEqual(oversized.returncode, 1)
            self.assertEqual(len(oversized.stdout.splitlines()), 1)
            with LinuxShellTask(
                FIXTURE, root / "journal", root / "candidate", WORKER
            ) as session:
                self.assertEqual(session.task.state["revision"], 1)
                OBSERVATIONS.append(
                    {
                        "launcher_checked": True,
                        "lost_reply_receipt": receipt,
                        "recovered": session.evidence(),
                    }
                )


if __name__ == "__main__":
    result = unittest.main(exit=False).result
    if not result.wasSuccessful():
        raise SystemExit(1)
    evidence = Path(os.environ["RAMEN_TASK_LS_EVIDENCE"])
    evidence.mkdir(parents=True, exist_ok=True)
    source = {
        p: hashlib.sha256(Path(p).read_bytes()).hexdigest()
        for p in subprocess.check_output(
            ["git", "ls-files", "-co", "--exclude-standard", "-z"]
        )
        .decode()
        .split("\0")
        if p and Path(p).is_file()
    }
    (evidence / "source-files.json").write_text(json.dumps(source, indent=2) + "\n")
    report_pins = {
        "source_revision": subprocess.check_output(
            ["git", "rev-parse", "HEAD"], text=True
        ).strip(),
        "source_files_sha256": hashlib.sha256(
            json.dumps(source, sort_keys=True).encode()
        ).hexdigest(),
        "cargo_lock_sha256": hashlib.sha256(
            Path("Cargo.lock").read_bytes()
        ).hexdigest(),
        "worker_sha256": hashlib.sha256(WORKER.read_bytes()).hexdigest(),
        "launcher_sha256": hashlib.sha256(
            Path(__file__).with_name("ls_launcher.py").read_bytes()
        ).hexdigest(),
    }
    (evidence / "report.json").write_text(
        json.dumps(
            {
                "schema_version": 1,
                "arm": "LS",
                "environment": "linux",
                "target_kernel_enforcement": False,
                "test_cases": result.testsRun,
                **report_pins,
                "claim": "scripted-scoped-shell-durable-transactions",
                "fixture_partition": "development",
                "model_comparison": False,
                "a2_conformant": False,
                "observations": OBSERVATIONS,
            },
            indent=2,
        )
        + "\n"
    )
