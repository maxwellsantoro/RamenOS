#!/usr/bin/env python3
"""Exercise the actual JSON executable and published schemas; never calls a model."""
import base64
import hashlib
import json
import os
from pathlib import Path
import re
import select
import subprocess
import sys
import tempfile
from jsonschema import Draft202012Validator

binary = Path(sys.argv[1]).resolve()
worker = Path(os.environ['RAMEN_TASK_VALIDATOR_WORKER']).resolve()
root = Path(os.environ['RAMEN_TASK_ADAPTER_EVIDENCE_DIR'])
root.mkdir(parents=True, exist_ok=True)
contract_bytes = subprocess.check_output([str(binary),'--describe'],timeout=10).rstrip(b'\n')
contract = json.loads(contract_bytes)
response_schema = Draft202012Validator(contract['response_schema'])
bootstrap_schema = Draft202012Validator(contract['bootstrap_schema'])
tools = {t['name']:Draft202012Validator(t['input_schema']) for t in contract['tools']}
for validator in [response_schema,bootstrap_schema,*tools.values()]:
    Draft202012Validator.check_schema(validator.schema)
transcript = []


def read_line(process):
    if not select.select([process.stdout],[],[],8)[0]:
        raise AssertionError('adapter did not respond')
    line = process.stdout.readline(131074)
    assert line.endswith(b'\n') and len(line)<=131073, 'unbounded/partial adapter output'
    return json.loads(line)


def start(fixture, store):
    p = subprocess.Popen([str(binary),'--fixture',str(fixture),'--store',str(store),'--worker',str(worker)],
      stdin=subprocess.PIPE,stdout=subprocess.PIPE,stderr=subprocess.PIPE,bufsize=0)
    b = read_line(p)
    bootstrap_schema.validate(b)
    return p,b


def stop(p):
    if not p.stdin.closed:
        p.stdin.close()
    code=p.wait(timeout=8)
    err=p.stderr.read(16385)
    p.stdout.close(); p.stderr.close()
    assert code==0 and not err, err


def send(p, id, call, valid=True):
    req={'schema_version':1,'request_id':str(id),'call':call}
    if valid:
        tools[call['operation']].validate(req)
    p.stdin.write(json.dumps(req,separators=(',',':')).encode()+b'\n')
    p.stdin.flush()
    reply=read_line(p)
    response_schema.validate(reply)
    transcript.append({'request':req,'response':reply})
    return reply


def b64(data):
    return base64.b64encode(data).decode()


with tempfile.TemporaryDirectory(prefix='ramenos-adapter-') as tmp:
    directory=Path(tmp)
    fixture=directory/'fixture'; fixture.mkdir()
    source=Path('tools/agent_task/fixtures')
    for name in ['config.json','schema.json','policy.json','notes.txt']:
        (fixture/name).write_bytes((source/name).read_bytes())
    # Export the compiled, identical WAT bytes without a helper exposed to the consumer.
    subprocess.run(['cargo','run','-p','store_service','--features','agent_task_v1_dev',
      '--example','export_agent_task_fixture','--',str(fixture)],check=True)
    store=directory/'store'
    p=None
    try:
        p,b=start(fixture,store)
        grant=send(p,1,{'operation':'request_grant','policy_cap':b['policy_cap'],'task_id':b['task_id'],
          'resource':b['resources'][0]['resource'],'rights':['read','stage','validate','commit','observe'],'lifetime_ms':60000})
        cap=grant['result']['task_cap']
        state=send(p,2,{'operation':'get_task_state','task_cap':cap})['result']['state']
        inputs=[]
        for i,resource in enumerate(b['resources']):
            result=send(p,3+i,{'operation':'read_input','task_cap':cap,'resource':resource['resource']})
            data=base64.b64decode(result['result']['bytes_base64'],validate=True)
            assert 'sha256:'+hashlib.sha256(data).hexdigest()==result['result']['content_id']
            inputs.append(data)
        config=json.loads(inputs[0]); schema=json.loads(inputs[1]); original_label=config['label']
        config['enabled']=schema['enabled']
        candidate=json.dumps(config,separators=(',',':'),sort_keys=True).encode()
        staged=send(p,6,{'operation':'stage_candidate','task_cap':cap,'bytes_base64':b64(candidate)})
        candidate_cap=staged['result']['candidate_cap']
        validation=send(p,7,{'operation':'validate_candidate','task_cap':cap,'candidate_cap':candidate_cap,'validator_id':state['validator_id']})
        assert validation['result']['outcome']=='valid' and not validation['result']['truncated']
        commit={'operation':'commit_candidate','task_cap':cap,'candidate_cap':candidate_cap,
          'expected_revision':state['revision'],'expected_content_id':state['content_id']}
        receipt=send(p,8,commit)
        assert receipt['status']=='ok' and receipt['result']['content_id']=='sha256:'+hashlib.sha256(candidate).hexdigest()
        assert send(p,8,commit)==receipt
        lookup=send(p,9,{'operation':'get_receipt','task_cap':cap,'commit_request_id':'8'})
        assert lookup['result']['content_id']==receipt['result']['content_id'] and lookup['result']['revision']=='1'
        maximum=send(p,18446744073709551615,{'operation':'get_task_state','task_cap':cap})
        assert maximum['request_id']=='18446744073709551615'
        bad_read=send(p,10,{'operation':'read_input','task_cap':cap,'resource':'resource:00000000000003e7'})
        assert bad_read['status']=='denied' and bad_read['result'] is None
        # Validate schema boundaries independently of Rust's request decoder.
        for value in ['0','01','18446744073709551616','1\n']:
            req={'schema_version':1,'request_id':value,'call':{'operation':'get_task_state','task_cap':cap}}
            assert not tools['get_task_state'].is_valid(req), value
        for value in ['', 'Zg', 'Zh==', b64(bytes(65537))]:
            req={'schema_version':1,'request_id':'11','call':{'operation':'stage_candidate','task_cap':cap,'bytes_base64':value}}
            assert not tools['stage_candidate'].is_valid(req), 'noncanonical/oversized base64 schema'
            bad=send(p,11,req['call'],False)
            assert bad['status']=='invalid' and bad['request_id'] is None and bad['result'] is None
        # Version/domain/path escape fields cannot become native dispatch.
        bad=send(p,12,{'operation':'read_input','task_cap':cap,'resource':b['resources'][0]['resource'],'domain_id':7},False)
        assert bad['status']=='invalid' and bad['result'] is None
        revoked=send(p,13,{'operation':'revoke_grant','policy_cap':b['policy_cap'],'task_cap':cap})
        assert revoked['status']=='ok'
        denied=send(p,14,{'operation':'get_task_state','task_cap':cap})
        assert denied['status']=='denied' and denied['result'] is None
        stop(p); p=None
        p,b=start(fixture,store)
        renewed=send(p,15,{'operation':'request_grant','policy_cap':b['policy_cap'],'task_id':b['task_id'],
          'resource':b['resources'][0]['resource'],'rights':['commit'],'lifetime_ms':60000})
        commit['task_cap']=renewed['result']['task_cap']
        assert send(p,8,commit)==receipt
        stop(p); p=None
        assert json.loads(candidate)==schema and json.loads(candidate)['label']==original_label
        p,b=start(fixture,directory/'oversized-store')
        p.stdin.write(b' '*131073+b'\n'); p.stdin.flush(); p.stdin.close()
        assert p.wait(timeout=8)==1
        assert p.stdout.read(1)==b''
        p.stdout.close(); p.stderr.close(); p=None
    finally:
        if p is not None:
            p.kill(); p.wait(timeout=8)
            for stream in [p.stdin,p.stdout,p.stderr]:
                if not stream.closed: stream.close()

(root/'tool-contract.json').write_bytes(contract_bytes+b'\n')
(root/'transcript.json').write_text(json.dumps(transcript,indent=2)+'\n')
report=json.loads((root/'report.json').read_bytes())
report.update({'cli_checked':True,'request_response_schemas_checked':True,
  'tool_contract_sha256':hashlib.sha256(contract_bytes).hexdigest(),'tool_contract_bytes':len(contract_bytes),
  'transcript_sha256':hashlib.sha256((root/'transcript.json').read_bytes()).hexdigest(),
  'adapter_binary_sha256':hashlib.sha256(binary.read_bytes()).hexdigest(),
  'worker_sha256':hashlib.sha256(worker.read_bytes()).hexdigest(),
  'source_revision':subprocess.check_output(['git','rev-parse','HEAD'],text=True).strip(),
  'cargo_lock_sha256':hashlib.sha256(Path('Cargo.lock').read_bytes()).hexdigest()})
source={p:hashlib.sha256(Path(p).read_bytes()).hexdigest()
        for p in subprocess.check_output(['git','ls-files','-co','--exclude-standard','-z']).decode().split('\0')
        if p and Path(p).is_file()}
report['source_files_sha256']=hashlib.sha256(json.dumps(source,sort_keys=True).encode()).hexdigest()
(root/'source-files.json').write_text(json.dumps(source,indent=2)+'\n')
(root/'report.json').write_text(json.dumps(report,indent=2)+'\n')
print('JSON executable: useful task, published schemas, bounds, revocation and restart retry passed')
