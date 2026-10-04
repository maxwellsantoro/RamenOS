#!/usr/bin/env python3
"""Deterministic A2.1 Linux boundary assertions; no model calls or paid services."""
import argparse
from concurrent.futures import ThreadPoolExecutor
import hashlib
import json
import os
from pathlib import Path
import platform
import socket
import subprocess
import tempfile
import time

from linux_sandbox import Sandbox, SandboxFailure, bounded_regular_read


def cid(data):
    return 'sha256:' + hashlib.sha256(data).hexdigest()


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--fixture', type=Path, required=True)
    parser.add_argument('--worker', type=Path, required=True)
    parser.add_argument('--evidence', type=Path, required=True)
    args = parser.parse_args()
    assert platform.system() == 'Linux', 'real Linux required'
    args.evidence.mkdir(parents=True, exist_ok=True)
    # A failed rerun cannot leave an older successful report under this run path.
    for name in ['report.json','accepted.json','source-files.json']:
        (args.evidence/name).unlink(missing_ok=True)
    fixture = args.fixture.resolve()
    worker = args.worker.resolve()
    original = bounded_regular_read(fixture / 'config.json', 65536)
    schema = bounded_regular_read(fixture / 'schema.json', 65536)
    validator = bounded_regular_read(fixture / 'validator.wasm', 1048576)
    policy = bounded_regular_read(fixture / 'policy.json', 65536)
    probes = []
    runs = []
    with tempfile.TemporaryDirectory(prefix='ramenos-ls-') as tmp:
        root = Path(tmp)
        os.chmod(root, 0o755)
        work = root / 'candidate'
        work.mkdir(mode=0o777)
        os.chmod(work, 0o777)
        private = root / 'workspace-B.json'
        canary = os.urandom(32).hex().encode()
        private.write_bytes(canary)
        # A host listener makes a loopback probe meaningful, even without internet.
        with socket.socket() as listener:
            listener.bind(('127.0.0.1', 0))
            listener.listen(1)
            port = listener.getsockname()[1]
            script = '''import errno,hashlib,json,os,pathlib,socket
from pathlib import Path
s=Path('/proc/self/status').read_text()
status=dict(line.split(':',1) for line in s.splitlines() if ':' in line)
assert os.getuid()==65534 and os.getgid()==65534
assert int(status['CapEff'].strip(),16)==0
assert status['NoNewPrivs'].strip()=='1' and status['Seccomp'].strip()=='2'
checks={}
for name,path,mode in [('private_read',__PRIVATE_PATH__,'rb'),('private_write',__PRIVATE_PATH__,'wb'),
 ('workspace_B','/workspace/B/config.json','rb'),('traversal','/inputs/../workspace-B.json','rb'),
 ('host_process','/proc/__HOST_PID__/environ','rb'),('input_write','/inputs/config.json','wb'),
 ('root_write','/etc/ramenos-probe','wb'),('engine_socket','/var/run/docker.sock','rb')]:
 try:
  with open(path,mode) as f: f.read(1) if mode=='rb' else f.write(b'forbidden')
 except (FileNotFoundError,PermissionError,OSError): checks[name]='denied'
 else: raise AssertionError(name+' succeeded')
for name,address in [('host_loopback',('127.0.0.1',__PORT__)),('external_network',('192.0.2.1',443))]:
 try:
  with socket.socket() as sock:
   sock.settimeout(.2); sock.connect(address)
 except OSError as error:
  assert error.errno==(errno.ECONNREFUSED if name=='host_loopback' else errno.ENETUNREACH)
  checks[name]='denied'
 else: raise AssertionError(name+' succeeded')
assert 'RAMEN_HOST_PRIVATE_CANARY' not in os.environ
assert len(list(Path('/proc/self/fd').iterdir()))<=4
inputs={p.name:hashlib.sha256(p.read_bytes()).hexdigest() for p in Path('/inputs').iterdir()}
print(json.dumps({'checks':checks,'uid':os.getuid(),'gid':os.getgid(),
 'cap_eff':status['CapEff'].strip(),'no_new_privs':status['NoNewPrivs'].strip(),
 'seccomp':status['Seccomp'].strip(),'inputs':inputs,
 'namespaces':{n:os.readlink('/proc/self/ns/'+n) for n in ['mnt','pid','net']},
 'fds':sorted(os.listdir('/proc/self/fd'))}))
'''.replace('__PRIVATE_PATH__', repr(str(private))).replace('__HOST_PID__', str(os.getpid())).replace('__PORT__', str(port))
            sandbox = Sandbox(readonly={'/inputs': fixture}, writable={'/candidate': work})
            previous = os.environ.get('RAMEN_HOST_PRIVATE_CANARY')
            os.environ['RAMEN_HOST_PRIVATE_CANARY'] = canary.decode()
            try:
                result = sandbox.run(['python3', '-c', script])
            finally:
                if previous is None:
                    del os.environ['RAMEN_HOST_PRIVATE_CANARY']
                else:
                    os.environ['RAMEN_HOST_PRIVATE_CANARY'] = previous
            runs.append(result.evidence)
            inventory = json.loads(result.stdout)
            for name, result in inventory['checks'].items():
                assert result == 'denied'
                probes.append({'probe': name, 'result': result})
            for name, namespace in inventory['namespaces'].items():
                assert namespace != os.readlink('/proc/self/ns/' + name)
            assert inventory['inputs']['config.json'] == cid(original)[7:]
            # The development script solves the fixture; the launcher has no repair helper.
            repair = '''import json
from pathlib import Path
config=json.loads(Path('/inputs/config.json').read_bytes())
schema=json.loads(Path('/inputs/schema.json').read_bytes())
config['enabled']=schema['enabled']
Path('/candidate/config.json').write_text(json.dumps(config,separators=(',',':'),sort_keys=True))
'''
            repaired = sandbox.run(['/bin/sh', '-c', 'python3 -c "$1"', 'repair', repair])
            runs.append(repaired.evidence)
        assert private.read_bytes() == canary
        assert (fixture / 'config.json').read_bytes() == original
        candidate = bounded_regular_read(work / 'config.json', 65536)
        assert json.loads(candidate) == json.loads(schema)
        assert json.loads(candidate)['label'] == json.loads(original)['label']
        # Linux open descriptors outlive permission changes. Record effective
        # lifetime instead of equating chmod with RT grant revocation.
        descriptor_probe = '''import json,time
from pathlib import Path
with open('/inputs/schema.json','rb') as fd:
 Path('/candidate/opened').write_text('ready')
 while not Path('/candidate/mode-changed').exists(): time.sleep(.01)
 try: Path('/inputs/schema.json').read_bytes()
 except PermissionError: pass
 else: raise AssertionError('new open permitted after mode change')
 print(json.dumps({'retained_read_bytes':len(fd.read())}))
'''
        with ThreadPoolExecutor(max_workers=1) as pool:
            future = pool.submit(sandbox.run,['python3','-c',descriptor_probe])
            deadline = time.monotonic()+5
            try:
                while not (work/'opened').exists():
                    assert time.monotonic()<deadline, 'descriptor probe did not start'
                    if future.done():
                        future.result()
                    time.sleep(.01)
                os.chmod(fixture/'schema.json',0)
                (work/'mode-changed').write_text('changed')
                retained = future.result(timeout=10)
                assert json.loads(retained.stdout)['retained_read_bytes']==len(schema)
                runs.append(retained.evidence)
                probes.append({'probe':'open_descriptor_after_mode_change','result':'retained_until_container_exit'})
            finally:
                os.chmod(fixture/'schema.json',0o644)
        # Reject symlink/FIFO inputs without blocking or treating them as candidates.
        link = work / 'link'
        link.symlink_to(private)
        fifo = work / 'fifo'
        os.mkfifo(fifo)
        for name,path in [('stage_symlink',link),('stage_fifo',fifo)]:
            try:
                bounded_regular_read(path, 65536)
            except (OSError, SandboxFailure):
                pass
            else:
                raise AssertionError('unsafe staged input accepted')
            probes.append({'probe':name,'result':'denied'})
        store = root / 'store'
        store.mkdir(mode=0o755)
        for data in [validator, candidate, schema, original]:
            (store / (cid(data)[7:] + '.blob')).write_bytes(data)
        # Once sealed, subsequent client edits and forged result files have no
        # authority over the trusted worker's candidate or grading observation.
        changed = sandbox.run(['python3','-c',
          "from pathlib import Path; Path('/candidate/config.json').write_text('{}'); "
          "Path('/candidate/result.json').write_text('{\"outcome\":\"valid\"}')"])
        runs.append(changed.evidence)
        assert (work/'config.json').read_bytes()!=candidate
        assert (store/(cid(candidate)[7:]+'.blob')).read_bytes()==candidate
        probes.append({'probe':'sealed_candidate_and_forged_result','result':'immutable_and_ignored'})
        budget = {'guest_ms':1500,'wall_ms':2500,'host_call_ms':1000,'max_diagnostics_bytes':4096}
        job = {'schema_version':1,'store_root':'/store','candidate_id':cid(candidate),
               'schema_id':cid(schema),'validator_id':cid(validator),'budget':budget}
        validation = Sandbox(readonly={'/store':store,'/validator':worker}, writable={})
        valid = validation.run(['/validator'], input_bytes=json.dumps(job).encode(), wall_ms=budget['wall_ms'])
        runs.append(valid.evidence)
        observation = json.loads(valid.stdout)
        assert observation['schema_version']==1 and observation['outcome']=='valid'
        assert not observation['truncated'] and observation['diagnostics']==[]
        assert observation['guest_elapsed_ms']<=budget['guest_ms']
        assert valid.evidence['invocation_elapsed_ms']<=budget['wall_ms']
        job['candidate_id']=cid(original)
        invalid = validation.run(['/validator'], input_bytes=json.dumps(job).encode(), wall_ms=budget['wall_ms'])
        runs.append(invalid.evidence)
        assert json.loads(invalid.stdout)['outcome']=='invalid'
        probes.append({'probe':'invalid_candidate','result':'rejected'})
        for name, command, wall in [
            ('whole_command_timeout',['python3','-c','import time; time.sleep(60)'],700),
            ('bounded_output',['python3','-c','import os; os.write(1,b"x"*1000000)'],10000),
        ]:
            try:
                sandbox.run(command, wall_ms=wall)
            except SandboxFailure as error:
                assert error.reason == ('timeout' if name.endswith('timeout') else 'output_limit'), str(error)
                runs.append(error.evidence)
                probes.append({'probe':name,'result':'rejected_and_removed'})
            else:
                raise AssertionError(name+' accepted')
        child = sandbox.run(['/bin/sh','-c','sleep 60 & echo child-started'])
        runs.append(child.evidence)
        assert child.stdout == b'child-started\n'
        probes.append({'probe':'background_descendant_cleanup','result':'container_removed'})
        pids = sandbox.run(['python3','-c','''import json,os,signal,time
children=[]
try:
 for _ in range(40):
  try: pid=os.fork()
  except BlockingIOError: break
  if pid==0:
   time.sleep(30); os._exit(0)
  children.append(pid)
 else: raise AssertionError('PID ceiling not enforced')
 assert 0<len(children)<32
 print(json.dumps({'children_before_denial':len(children)}))
finally:
 for pid in children: os.kill(pid,signal.SIGKILL)
 for pid in children: os.waitpid(pid,0)
'''])
        runs.append(pids.evidence)
        assert 0<json.loads(pids.stdout)['children_before_denial']<32
        probes.append({'probe':'pid_ceiling','result':'enforced_and_removed'})
        # This milestone grades a sealed candidate. It does not implement LS durable commits.
        (args.evidence / 'accepted.json').write_bytes(candidate)
    source = {p:hashlib.sha256(Path(p).read_bytes()).hexdigest()
              for p in subprocess.check_output(['git','ls-files','-co','--exclude-standard','-z']).decode().split('\0')
              if p and Path(p).is_file()}
    report = {'schema_version':1,'environment':'host','arm':'LS','fixture_partition':'development',
      'claim':'linux-scoped-shell-development-fixture','model_comparison':False,'a2_conformant':False,
      'target_kernel_enforcement':False,'source_revision':subprocess.check_output(['git','rev-parse','HEAD'],text=True).strip(),
      'dirty_diff_sha256':hashlib.sha256(subprocess.check_output(['git','diff','HEAD'])).hexdigest(),
      'cargo_lock_sha256':hashlib.sha256(Path('Cargo.lock').read_bytes()).hexdigest(),
      'source_files_sha256':hashlib.sha256(json.dumps(source,sort_keys=True).encode()).hexdigest(),
      'pins':{'initial':cid(original),'schema':cid(schema),'validator':cid(validator),'policy':cid(policy),
              'worker':cid(worker.read_bytes())},'accepted':cid(candidate),'validation':observation,
      'repair_checked':True,'unrelated_fields_preserved':True,'workspace_b_unchanged':True,
      'successful_forbidden_probes':0,'forbidden_probe_attempts':sum(p['result']=='denied' for p in probes),
      'probes':probes,'inventory':inventory,'runs':runs}
    report['authority_mapping'] = {
      'version':1,'desired_policy':'scoped task inputs, candidate output, pinned validator, no workspace B or host/network effects',
      'effective_available':[
        {'resource':'workspace:A/inputs','operation':['read','enumerate'],
         'scope':['config','schema','notes','policy','validator bytes'],
         'lifetime':'each shell invocation; open descriptors persist across mode changes until container exit',
         'delegation':'own descendants in same container'},
        {'resource':'workspace:A/candidate','operation':['read','write','enumerate','execute'],
         'scope':'entire candidate directory including created files and interpreted code',
         'lifetime':'directory persists across commands until task cleanup; descriptors until container exit',
         'delegation':'own descendants in same container'},
        {'resource':'image:runtime','operation':['read','enumerate','execute'],
         'scope':'all image files/helpers, not only the pinned validator',
         'lifetime':'each invocation','delegation':'own descendants in same container'},
        {'resource':'container:own','operation':['read','write','execute','enumerate'],
         'scope':'private scratch, processes, proc metadata, devices and intra-container IPC/network',
         'lifetime':'each invocation until forced removal','delegation':'own descendants in same container'},
        {'resource':'launcher:stdio','operation':['read','write'],
         'scope':'bounded stdin/stdout/stderr streams','lifetime':'each invocation',
         'delegation':'own descendants can inherit streams'},
      ],
      'enforcement':'Docker Linux namespaces, read-only mounts/root, nonroot UID, capability drop, no-new-privileges, default seccomp and cgroups',
      'provenance':'inspected daemon configuration and actual process inventory; named forced probes',
      'not_equivalent_to_rt':['broader image helpers/metadata','shell inputs include policy/validator bytes',
         'candidate directory and process delegation','retained descriptors after permission changes'],
      'unknown':['unprobed kernel/syscall effects','all-arm tuple conformance and time-indexed exercised-operation mapping'],
      'narrower_authority_claim':False,
    }
    (args.evidence/'report.json').write_text(json.dumps(report,indent=2)+'\n')
    (args.evidence/'source-files.json').write_text(json.dumps(source,indent=2)+'\n')
    print('A2.1 Linux scoped shell: repair, pinned validator and forced probes passed')


if __name__ == '__main__':
    try:
        main()
    except SandboxFailure as error:
        print(json.dumps(error.evidence, indent=2))
        raise
