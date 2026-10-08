"""Trusted A2.1 launcher. No daemon access, mount selection or helper API enters agent context.

A command receives exactly the operator's mounts, image helpers and private scratch.
Docker is the Linux enforcement substrate; this is not a native OS interface.
"""
from dataclasses import dataclass
import json
import os
from pathlib import Path
import platform
import re
import selectors
import signal
import stat
import subprocess
import time
import uuid
from lifecycle_ledger import Ledger, LedgerError

DEFAULT_IMAGE = 'python@sha256:139020233cc412efe4c8135b0efe1c7569dc8b28ddd88bddb109b764f8977e30'
MAX_OUTPUT = 16384
MAX_INPUT = 8192


class SandboxFailure(RuntimeError):
    def __init__(self, reason, evidence=None):
        super().__init__('Linux sandbox: ' + reason)
        self.reason = reason
        self.evidence = evidence or {}


@dataclass
class Result:
    stdout: bytes
    stderr: bytes
    evidence: dict


def bounded_regular_read(path, limit):
    """Seal bytes from one opened inode; symlinks and nonregular inputs fail closed."""
    fd = os.open(path, os.O_RDONLY | os.O_NOFOLLOW | os.O_NONBLOCK | os.O_CLOEXEC)
    with os.fdopen(fd, 'rb') as stream:
        if not stat.S_ISREG(os.fstat(stream.fileno()).st_mode):
            raise SandboxFailure('nonregular_input')
        data = stream.read(limit + 1)
        if not data or len(data) > limit:
            raise SandboxFailure('input_limit')
        return data


class Sandbox:
    def __init__(self, *, readonly, writable):
        if platform.system() != 'Linux':
            raise SandboxFailure('Linux_required')
        # Resolve a locally installed immutable image; never pull/fall back during the gate.
        reference = os.environ.get('RAMEN_TASK_LINUX_IMAGE', DEFAULT_IMAGE)
        image = subprocess.run(['docker','image','inspect',reference], capture_output=True, timeout=10, check=True)
        self.image = json.loads(image.stdout)[0]['Id']
        if not self.image.startswith('sha256:') or len(self.image) != 71:
            raise SandboxFailure('unpinned_image')
        info = subprocess.run(['docker','info','--format','{{json .}}'],capture_output=True,timeout=10,check=True)
        info = json.loads(info.stdout)
        if info['OSType']!='linux' or 'name=seccomp,profile=builtin' not in info['SecurityOptions']:
            raise SandboxFailure('engine_configuration')
        self.engine = {k:info[k] for k in ['ServerVersion','KernelVersion','OSType','Architecture','SecurityOptions']}
        self.mounts = []
        for ro, mounts in [(True, readonly), (False, writable)]:
            for target, source in mounts.items():
                # This launcher is trusted; only its fixed proof namespace may be mounted.
                if target not in ['/inputs','/candidate','/store','/validator','/task']:
                    raise SandboxFailure('invalid_mount')
                path = Path(source).resolve(strict=True)
                if ',' in str(path) or '\n' in str(path):
                    raise SandboxFailure('invalid_mount')
                self.mounts.append((target,path,ro))
        if len({m[0] for m in self.mounts}) != len(self.mounts):
            raise SandboxFailure('duplicate_mount')

    @staticmethod
    def _engine(args, deadline):
        diagnostics = {'engine_command': args[0] if args and args[0] in ('create','inspect') else 'other'}

        def retain_stderr(raw):
            raw = raw or b''
            diagnostics['engine_stderr'] = raw[:16384].decode(errors='replace')
            diagnostics['engine_stderr_truncated'] = len(raw) > 16384

        remaining = deadline - time.monotonic()
        if remaining <= 0:
            raise SandboxFailure('timeout',diagnostics)
        try:
            result = subprocess.run(['docker',*args], capture_output=True, timeout=remaining)
        except subprocess.TimeoutExpired as error:
            retain_stderr(error.stderr)
            raise SandboxFailure('timeout',diagnostics) from error
        if len(result.stdout) + len(result.stderr) > 262144:
            diagnostics['engine_returncode'] = result.returncode
            retain_stderr(result.stderr)
            raise SandboxFailure('engine_output_limit',diagnostics)
        if result.returncode:
            # Trusted diagnostics stay out of agent-visible errors.
            diagnostics['engine_returncode'] = result.returncode
            retain_stderr(result.stderr)
            raise SandboxFailure('engine_failure',diagnostics)
        return result.stdout

    def run(self, command, *, input_bytes=b'', wall_ms=10000, allow_nonzero=False, invocation_id=None):
        if not command or not all(isinstance(p,str) and '\0' not in p for p in command):
            raise SandboxFailure('invalid_command')
        if len(input_bytes)>MAX_INPUT or not 1<=wall_ms<=35000:
            raise SandboxFailure('input_limit')
        start = time.monotonic()
        deadline = start + wall_ms/1000
        invocation_id = invocation_id or uuid.uuid4().hex
        if re.fullmatch(r'[0-9a-f]{32}',invocation_id) is None:
            raise SandboxFailure('invalid_invocation')
        name = 'ramenos-sw0-' + invocation_id
        evidence = {'name':name,'image_id':self.image,'engine':self.engine,
                    'wall_ms':wall_ms,'removed':False,'created':False}
        cli = None
        failure = None
        phase = 'prepare'
        stdout = bytearray()
        stderr = bytearray()
        ledger = None
        try:
            args = ['create','--name',name,'--interactive','--network','none','--read-only',
                    '--cap-drop','ALL','--security-opt','no-new-privileges',
                    '--user','65534:65534','--pids-limit','32','--memory','2g',
                    '--memory-swap','2g','--cpus','1','--ulimit','nofile=64:64',
                    '--tmpfs','/tmp:rw,nosuid,nodev,noexec,size=16m,mode=1777',
                    '--env','PATH=/usr/local/bin:/usr/bin:/bin','--env','HOME=/tmp',
                    '--env','LANG=C.UTF-8','--workdir','/tmp']
            for target, path, ro in self.mounts:
                args += ['--mount',f'type=bind,source={path},target={target}' + (',readonly' if ro else '')]
            scope = os.environ.get('RAMEN_TASK_EVALUATOR_SCOPE')
            if scope is not None:
                if re.fullmatch(r'[0-9a-f]{32}',scope) is None:
                    raise SandboxFailure('invalid_evaluator_scope')
                args += ['--label','org.ramenos.evaluator-session=' + scope]
                evidence['evaluator_scope'] = scope
            ledger_root = os.environ.get('RAMEN_TASK_EVALUATOR_LEDGER')
            if ledger_root is not None:
                ledger = Ledger(ledger_root,scope)
                ledger.begin(invocation_id,self.image)
                evidence['lifecycle_invocation'] = invocation_id
            phase = 'create'
            output = self._engine([*args,self.image,*command],deadline)
            evidence['created'] = True
            container_id = output.strip().decode('ascii')
            if re.fullmatch(r'[0-9a-f]{64}',container_id) is None:
                raise SandboxFailure('create_ack_invalid')
            evidence['container_id'] = container_id
            if ledger is not None:
                ledger.acknowledge(invocation_id,container_id)
            phase = 'inspect'
            config = json.loads(self._engine(['inspect','--type','container',name],deadline))[0]
            if scope is not None and (config['Config'].get('Labels') or {}).get('org.ramenos.evaluator-session') != scope:
                raise SandboxFailure('configuration_mismatch')
            host = config['HostConfig']
            # Assert actual daemon configuration, including absence of inherited sockets/devices.
            if not (host['NetworkMode']=='none' and host['ReadonlyRootfs'] and
                    host['CapDrop']==['ALL'] and not host['CapAdd'] and not host['Privileged'] and
                    'no-new-privileges' in host['SecurityOpt'] and
                    host['PidsLimit']==32 and host['Memory']==2147483648 and
                    host['MemorySwap']==2147483648 and host['NanoCpus']==1000000000 and
                    host['PidMode']=='' and host['IpcMode']=='private' and
                    not host['Devices'] and not host['DeviceRequests'] and
                    config['Config']['User']=='65534:65534'):
                raise SandboxFailure('configuration_mismatch')
            actual = {(m['Destination'],m['Source'],not m['RW']) for m in config['Mounts']}
            expected = {(target,str(path),ro) for target,path,ro in self.mounts}
            if actual != expected:
                raise SandboxFailure('mount_mismatch')
            evidence['configuration'] = {
                k:host[k] for k in ['NetworkMode','ReadonlyRootfs','CapDrop','CapAdd','SecurityOpt',
                  'PidsLimit','Memory','MemorySwap','NanoCpus','PidMode','IpcMode','Tmpfs','Privileged']}
            evidence['mounts'] = config['Mounts']
            evidence['user'] = config['Config']['User']
            phase = 'attach'
            cli = subprocess.Popen(['docker','start','--attach','--interactive',name],
                    stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.PIPE,
                    start_new_session=True, close_fds=True)
            with selectors.DefaultSelector() as select:
                for stream, kind in [(cli.stdout,'out'),(cli.stderr,'err'),(cli.stdin,'in')]:
                    os.set_blocking(stream.fileno(),False)
                    select.register(stream,selectors.EVENT_WRITE if kind=='in' else selectors.EVENT_READ,kind)
                offset = 0
                while select.get_map():
                    remaining = deadline-time.monotonic()
                    if remaining<=0:
                        raise SandboxFailure('timeout')
                    for key,_ in select.select(min(.05,remaining)):
                        stream = key.fileobj
                        if key.data=='in':
                            if offset<len(input_bytes):
                                try:
                                    offset += os.write(stream.fileno(),input_bytes[offset:])
                                except BlockingIOError:
                                    continue
                                except BrokenPipeError:
                                    select.unregister(stream)
                                    stream.close()
                                    continue
                            if offset==len(input_bytes):
                                select.unregister(stream)
                                stream.close()
                        else:
                            try:
                                data = os.read(stream.fileno(),4096)
                            except BlockingIOError:
                                continue
                            if not data:
                                select.unregister(stream)
                                stream.close()
                            else:
                                (stdout if key.data=='out' else stderr).extend(data)
                                if len(stdout)+len(stderr)>MAX_OUTPUT:
                                    raise SandboxFailure('output_limit')
                remaining = deadline-time.monotonic()
                if remaining<=0:
                    raise SandboxFailure('timeout')
                try:
                    returncode = cli.wait(timeout=remaining)
                except subprocess.TimeoutExpired as error:
                    raise SandboxFailure('timeout') from error
                evidence['exit_code'] = returncode
                if returncode and not allow_nonzero:
                    evidence['trusted_stderr'] = bytes(stderr).decode(errors='replace')
                    raise SandboxFailure('command_failed')
        except SandboxFailure as error:
            failure = error.reason
            evidence['failure_reason'] = failure
            evidence['failure_phase'] = phase
            for key in ('engine_command','engine_returncode','engine_stderr','engine_stderr_truncated'):
                if key in error.evidence:
                    evidence[key] = error.evidence[key]
        except (LedgerError,OSError):
            failure = 'lifecycle_error'
            evidence['failure_reason'] = failure
            evidence['failure_phase'] = phase
        finally:
            evidence['invocation_elapsed_ms'] = round((time.monotonic()-start)*1000)
            cleanup_start = time.monotonic()
            # Every exit removes the entire container, including detached descendants.
            if cli is not None and cli.poll() is None:
                try:
                    os.killpg(cli.pid,signal.SIGKILL)
                except ProcessLookupError:
                    pass
                try:
                    cli.wait(timeout=5)
                except subprocess.TimeoutExpired:
                    failure = 'cleanup_failed'
                    evidence['cleanup_failure_reason'] = failure
            if cli is not None:
                for stream in [cli.stdin,cli.stdout,cli.stderr]:
                    if stream and not stream.closed:
                        stream.close()
            try:
                target = evidence.get('container_id',name)
                subprocess.run(['docker','rm','--force',target],capture_output=True,timeout=10)
                missing = subprocess.run(['docker','inspect','--type','container',target],capture_output=True,timeout=10)
                if missing.returncode != 1 or b'No such' not in missing.stderr:
                    raise SandboxFailure('cleanup_failed')
                evidence['removed'] = True
                if ledger is not None:
                    ledger.removed(invocation_id)
            except (subprocess.TimeoutExpired,SandboxFailure,LedgerError,OSError):
                failure = 'cleanup_failed'
                evidence['cleanup_failure_reason'] = failure
            if failure and 'failure_reason' not in evidence:
                evidence['failure_reason'] = failure
                evidence['failure_phase'] = 'cleanup'
            evidence['cleanup_elapsed_ms'] = round((time.monotonic()-cleanup_start)*1000)
            if not evidence['created']:
                # A timed-out create RPC may still be in flight in the daemon;
                # absence at one inspect cannot certify cleanup of that request.
                failure = 'creation_not_confirmed'
        if failure:
            raise SandboxFailure(failure,evidence)
        return Result(bytes(stdout),bytes(stderr),evidence)
