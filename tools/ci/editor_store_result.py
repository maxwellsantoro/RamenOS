#!/usr/bin/env python3
"""Readonly host Store Foundry evidence consumer; no handler imports or IO authority.

The root controller supplies the independently frozen source manifest and pure
codec executable pins. Artifact paths never choose executable or lifecycle authority.
"""
import argparse
import collections
import hashlib
import json
import os
from pathlib import Path
import re
import select
import subprocess
import sys
import time
import stat
import struct
import unittest

U64 = (1 << 64) - 1
U32 = (1 << 32) - 1
CASE = [
    'ui1_1_revoke_paused_key_frame_and_save',
    'ui1_1_stalled_session_leaves_other_session_usable',
    'ui1_1_clock_capacity_and_counter_limits_fail_closed',
    'ui1_1_store_commit_reopen_preserves_prior_blob',
    'ui1_1_store_same_base_conflict_and_operation_reuse',
    'ui1_1_store_lost_reply_reconciles_without_mutation_replay',
    'ui1_1_store_recovery_validates_atomic_selection_and_receipt',
]
LEGS = [3, 1, 16, 5, 1, 4, 18]
REOPEN = {(CASE[2],0), (CASE[2],1), (CASE[3],0), (CASE[5],3)} | {(CASE[6],i) for i in range(4)}
FAILED_REOPEN = {(CASE[2],5)} | {(CASE[3],i) for i in range(1,4)} | {(CASE[6],i) for i in range(4,18)}
UI_INVENTORY = {(CASE[0],0):1,(CASE[0],1):1,(CASE[1],0):1,(CASE[2],0):0,(CASE[2],1):15}
IDENTITY = {'schema_version','run_nonce','case','leg','fixture_id','native_test_pid','test_process_start_identity'}
OLD, NEW = b'note=old\n', b'note=new\n'
FONT = '50ab6522c495f06a69590ccb6958458be15e02a2ca4c6b1024b052e826d21c64'
ARGS = None


class EvidenceDenied(ValueError):
    pass


def require(condition, reason):
    if not condition:
        raise EvidenceDenied(reason)


def sha(raw):
    return hashlib.sha256(raw).hexdigest()


def keys(value, expected):
    require(type(value) is dict and set(value) == set(expected), 'unknown/missing object fields')
    return value


def uint(value, maximum=U64, positive=False):
    require(type(value) is int and int(positive) <= value <= maximum, 'invalid native integer')
    return value


def boolean(value):
    require(type(value) is bool, 'invalid boolean')
    return value


def hash32(value):
    require(type(value) is list and len(value) == 32, 'invalid Hash32')
    for item in value:
        uint(item,255)
    return bytes(value)


def hexhash(value):
    require(type(value) is str and re.fullmatch('[0-9a-f]{64}',value) is not None, 'invalid SHA256')
    return value


def rows(value, maximum):
    require(type(value) is list and len(value) <= maximum, 'unbounded array')
    return value


def strict_json(raw):
    def pairs(items):
        result = {}
        for name,value in items:
            require(name not in result, 'duplicate JSON key')
            result[name] = value
        return result
    def noninteger(value):
        raise EvidenceDenied('float/nonfinite JSON')
    try:
        value = json.loads(raw.decode('utf8','strict'),object_pairs_hook=pairs,
                           parse_float=noninteger,parse_constant=noninteger)
    except (UnicodeDecodeError,json.JSONDecodeError) as error:
        raise EvidenceDenied('invalid/truncated JSON') from error
    def bounded(node, depth=0):
        require(depth <= 32, 'JSON depth')
        if type(node) is dict:
            for key,value in node.items():
                require(type(key) is str, 'nonstring key')
                bounded(value,depth+1)
        elif type(node) is list:
            for item in node:
                bounded(item,depth+1)
        elif type(node) is str:
            require(len(node.encode('utf8')) <= len(raw), 'string length')
        else:
            require(node is None or type(node) in (bool,int), 'JSON scalar')
    bounded(value)
    return value


class Tree:
    """Rootfd-relative nofollow access; never open a recorded original root path."""
    def __init__(self, path, excluded_roots=()):
        require(type(path) is str and len(path.encode()) <= 4096, 'root path bound')
        self.path = Path(path)
        self.fd = os.open(path,os.O_RDONLY|os.O_DIRECTORY|os.O_NOFOLLOW)
        self.inventory = {}
        self.total_bytes = 0
        self.directories = 1
        excluded=set(excluded_roots)
        skipped=set()
        entries = 0
        def walk(fd, prefix='', depth=0):
            nonlocal entries
            require(depth <= 4, 'evidence directory depth')
            with os.scandir(fd) as iterator:
                for entry in iterator:
                    entries += 1
                    require(entries <= 32768, 'directory entry count before retention')
                    name = entry.name
                    require(re.fullmatch('[A-Za-z0-9_.-]{1,128}',name) is not None and name not in ('.','..'), 'unsafe evidence name')
                    relative = prefix+name
                    info = os.stat(name,dir_fd=fd,follow_symlinks=False)
                    require(not stat.S_ISLNK(info.st_mode), 'evidence symlink')
                    if stat.S_ISDIR(info.st_mode):
                        child_identity=(info.st_dev,info.st_ino)
                        if child_identity in excluded:
                            require(child_identity not in skipped, 'duplicate excluded root identity')
                            check=os.open(name,os.O_RDONLY|os.O_DIRECTORY|os.O_NOFOLLOW,dir_fd=fd)
                            try:
                                live=os.fstat(check)
                                require((live.st_dev,live.st_ino)==child_identity,'excluded opened root changed')
                            finally: os.close(check)
                            skipped.add(child_identity)
                            continue
                        self.directories += 1
                        require(self.directories <= 96, 'directory bound')
                        child = os.open(name,os.O_RDONLY|os.O_DIRECTORY|os.O_NOFOLLOW,dir_fd=fd)
                        try:
                            live=os.fstat(child)
                            require((live.st_dev,live.st_ino)==(info.st_dev,info.st_ino),'directory changed before traversal')
                            walk(child,relative+'/',depth+1)
                        finally: os.close(child)
                    else:
                        require(stat.S_ISREG(info.st_mode), 'nonregular evidence')
                        require(0 <= info.st_size <= 16777216, 'individual evidence admission bound')
                        self.total_bytes += info.st_size
                        require(self.total_bytes <= 1073741824 and len(self.inventory) < 20436, 'aggregate evidence bound')
                        self.inventory[relative] = (info.st_dev,info.st_ino,info.st_size)
        try:
            walk(self.fd)
        except BaseException:
            os.close(self.fd)
            self.fd = None
            raise

    def close(self):
        if self.fd is not None:
            os.close(self.fd)
            self.fd = None

    def read(self, relative, cap):
        require(type(relative) is str and len(relative) <= 256 and relative in self.inventory, 'undeclared evidence file')
        expected = self.inventory[relative]
        require(0 <= expected[2] <= cap, 'file admission bound')
        parts = relative.split('/')
        parent = os.dup(self.fd)
        try:
            for part in parts[:-1]:
                child = os.open(part,os.O_RDONLY|os.O_DIRECTORY|os.O_NOFOLLOW,dir_fd=parent)
                os.close(parent); parent = child
            fd = os.open(parts[-1],os.O_RDONLY|os.O_NOFOLLOW,dir_fd=parent)
            try:
                before = os.fstat(fd)
                require(stat.S_ISREG(before.st_mode) and (before.st_dev,before.st_ino,before.st_size)==expected, 'file identity changed')
                data = bytearray()
                while len(data) < expected[2]:
                    block = os.read(fd,min(65536,expected[2]-len(data)))
                    require(bool(block), 'truncated evidence')
                    data.extend(block)
                require(not os.read(fd,1), 'extra evidence bytes')
                after = os.fstat(fd)
                require((after.st_dev,after.st_ino,after.st_size,after.st_mtime_ns)==(before.st_dev,before.st_ino,before.st_size,before.st_mtime_ns), 'file changed during read')
                return bytes(data)
            finally: os.close(fd)
        finally: os.close(parent)

    def json(self, relative, cap):
        raw = self.read(relative,cap)
        require(bool(raw), 'empty JSON evidence')
        return strict_json(raw)


def actor(value):
    keys(value,{'owner_id','session_id','session_generation','instance_id','instance_generation'})
    for item in value.values(): uint(item,positive=True)


def selected(value):
    keys(value,{'schema_version','owner_id','selected_object_id','selected_generation','revision','content_hash','byte_len','manifest_hash'})
    require(uint(value['schema_version'],U32)==1, 'selected version')
    for name in ('owner_id','selected_object_id','selected_generation','revision'): uint(value[name],positive=True)
    uint(value['byte_len'],4096)
    hash32(value['content_hash']); hash32(value['manifest_hash'])


def allocation(value):
    keys(value,{'schema_version','actor','selected_object_id','selected_generation','operation_id','expected_revision','expected_content_hash'})
    require(uint(value['schema_version'],U32)==1, 'allocation version'); actor(value['actor'])
    for name in ('selected_object_id','selected_generation','operation_id','expected_revision'): uint(value[name],positive=True)
    hash32(value['expected_content_hash'])


def binding(value):
    keys(value,{'schema_version','allocation','source'})
    require(uint(value['schema_version'],U32)==1, 'binding version'); allocation(value['allocation'])
    source = keys(value['source'],{'source_object_id','source_generation','byte_len','content_hash'})
    canonical_handle(source['source_object_id'],2)
    require(uint(source['source_generation'],U32,True)==source['source_object_id'] & U32, 'source generation')
    uint(source['byte_len'],4096); hash32(source['content_hash'])


def permit(value):
    keys(value,{'schema_version','binding','successor_revision','service_epoch','writer_id','admission_epoch'})
    require(uint(value['schema_version'],U32)==1, 'permit version'); binding(value['binding'])
    for name in ('successor_revision','service_epoch','writer_id','admission_epoch'): uint(value[name],positive=True)
    require(value['binding']['allocation']['expected_revision'] < U64 and value['successor_revision']==value['binding']['allocation']['expected_revision']+1,'permit successor')


RECEIPT_KEYS = ['schema_version','total_len','outcome','reserved','owner_id','session_id','session_generation','original_instance_id','original_instance_generation','selected_object_id','selected_generation','operation_id','expected_revision','result_revision','backend_service_epoch','committed_at_ms','expected_content_hash','source_content_hash']


def receipt(value):
    keys(value,RECEIPT_KEYS)
    require(value['schema_version']==1 and type(value['schema_version']) is int and uint(value['total_len'],U32)==176 and uint(value['reserved'],U32)==0,'receipt header')
    require(value['outcome'] in ('committed','definitive_noncommit'),'receipt outcome')
    for name in RECEIPT_KEYS[4:16]: uint(value[name],positive=name not in ('result_revision','committed_at_ms'))
    if value['outcome']=='committed': require(value['expected_revision']<U64 and value['result_revision']==value['expected_revision']+1,'receipt revision')
    else: require(value['result_revision']==value['committed_at_ms']==0,'noncommit outcome')
    hash32(value['expected_content_hash']); hash32(value['source_content_hash'])


def receipt_bytes(raw):
    require(len(raw)==176,'receipt actual length')
    values = list(struct.unpack_from('<IIII',raw)) + list(struct.unpack_from('<12Q',raw,16)) + [list(raw[112:144]),list(raw[144:176])]
    require(values[2] in (0,1),'receipt wire outcome'); values[2] = ('committed','definitive_noncommit')[values[2]]
    value = dict(zip(RECEIPT_KEYS,values)); receipt(value); return value


def text_bytes(raw):
    require(64 <= len(raw) <= 4160,'selected actual length')
    version,total,obj,revision = struct.unpack_from('<IIQQ',raw)
    digest = raw[24:56]; length,reserved = struct.unpack_from('<II',raw,56); body = raw[64:]
    require(version==1 and total==len(raw) and length==len(body) and reserved==0 and obj>0 and revision>0,'selected header')
    require(all(b in (9,10) or 32<=b<=126 for b in body) and hashlib.sha256(body).digest()==digest,'selected body/hash')
    return obj,revision,body,digest


def canonical_handle(raw, kind=None):
    uint(raw,positive=True)
    require(raw>>56 in (1,2,3) and (kind is None or raw>>56==kind) and not (raw>>48 & 255) and (raw>>32 & 65535)>0 and (raw & U32)>0,'canonical handle')
    return raw


WIRE_LENGTHS = {1:40,2:40,3:32,4:24,5:56,6:40,7:32,8:40}


def wire(value, replies=False):
    rows(value,88); require(len(value)==88,'wire length')
    raw = bytes(uint(v,255) for v in value)
    protocol,msg,handle,length = struct.unpack_from('<IIQI',raw)
    require(protocol==368 and msg in WIRE_LENGTHS and length==WIRE_LENGTHS[msg] and not any(raw[20+length:]),'canonical typed wire')
    if handle: canonical_handle(handle)
    if msg in (4,5): require(not any(raw[20+(20 if msg==4 else 52):20+length]),'wire reserved')
    require(msg % 2 == (0 if replies else 1),'wire direction')
    if replies:
        status_offset = {2:36,4:16,6:32,8:32}[msg]
        status = struct.unpack_from('<I',raw,20+status_offset)[0]
        require(status<=11,'wire status')
    return raw,msg


def operation(value):
    keys(value,{'allocation','allocated_service_epoch','binding','submitted_service_epoch','permit','closure','state','successor','receipt'})
    allocation(value['allocation']); uint(value['allocated_service_epoch'],positive=True)
    for name,check in [('binding',binding),('permit',permit),('successor',selected),('receipt',receipt)]:
        if value[name] is not None: check(value[name])
    if value['submitted_service_epoch'] is not None: uint(value['submitted_service_epoch'],positive=True)
    require(value['closure'] in ('none','deadline','revoked','expired','service_retired'),'operation closure')
    require(value['state'] in ('allocated','submitted','permitted','committed','noncommit'),'operation state')
    if value['binding'] is not None: require(value['binding']['allocation']==value['allocation'],'operation binding')
    if value['permit'] is not None: require(value['permit']['binding']==value['binding'],'operation permit')
    if value['receipt'] is not None:
        r,a = value['receipt'],value['allocation']
        require((r['operation_id'],r['selected_object_id'],r['selected_generation'],r['expected_revision'],r['expected_content_hash'])==(a['operation_id'],a['selected_object_id'],a['selected_generation'],a['expected_revision'],a['expected_content_hash']),'operation receipt binding')
        for rkey,akey in [('owner_id','owner_id'),('session_id','session_id'),('session_generation','session_generation'),('original_instance_id','instance_id'),('original_instance_generation','instance_generation')]:
            require(r[rkey]==a['actor'][akey],'receipt original actor')
        require(value['binding'] is not None and r['source_content_hash']==value['binding']['source']['content_hash'],'receipt original source')
    b,p,r,successor = (value[k] for k in ('binding','permit','receipt','successor'))
    state=value['state']
    require((b is None)==(value['submitted_service_epoch'] is None),'submitted epoch option binding')
    if b is not None:require(value['submitted_service_epoch']>=value['allocated_service_epoch'],'original submitted epoch')
    if p is not None:require(p['service_epoch']==value['submitted_service_epoch'],'permit original submitted epoch')
    if r is not None:require(r['backend_service_epoch']==value['submitted_service_epoch'],'receipt original backend epoch')
    if successor is not None:
        a=value['allocation']
        require((successor['owner_id'],successor['selected_object_id'],successor['selected_generation'])==(a['actor']['owner_id'],a['selected_object_id'],a['selected_generation']),'successor bound owner/object')
    if state=='allocated':require(b is p is r is successor is None and value['closure']=='none','allocated state body')
    elif state=='submitted':require(b is not None and p is r is successor is None,'submitted state body')
    elif state=='permitted':require(b is not None and p is not None and r is successor is None,'permitted state body')
    elif state=='committed':
        require(b is not None and p is not None and r is not None and successor is not None,'committed missing body')
        require(r['outcome']=='committed' and r['result_revision']==p['successor_revision']==successor['revision'],'committed outcome/successor')
        require(successor['content_hash']==b['source']['content_hash'] and successor['byte_len']==b['source']['byte_len'],'committed original source')
    else:
        require(successor is None,'noncommit successor')
        require((b is None and r is None and p is None) or (b is not None and r is not None and r['outcome']=='definitive_noncommit'),'noncommit state body')


def raw_barrier(value):
    keys(value,{'request_id','operation_id','entered','released','settled','permit_issued'})
    uint(value['request_id']); uint(value['operation_id'])
    for name in ('entered','released','settled','permit_issued'): boolean(value[name])


def fence_shape(value, pid):
    keys(value,{'schema_version','namespace_id','fence_id','profile_hash','semantic_clock_min','identity_counter_min','service_producers','object_workers','objects','live_ownership_after_join'})
    require(type(value['schema_version']) is int and value['schema_version']==1,'fence schema')
    for name in ('namespace_id','fence_id','identity_counter_min'): uint(value[name],positive=True)
    uint(value['semantic_clock_min']); hash32(value['profile_hash'])
    require(uint(value['live_ownership_after_join'],U32)==0,'fence live ownership')
    producers = rows(value['service_producers'],64)+rows(value['object_workers'],2)
    ids = set()
    for producer in producers:
        keys(producer,{'producer_id','producer_generation','kind','native_pid','rust_thread_id','selected_object_id','request_id','operation_id','completed_join','join_outcome','settled_barrier','live_ownership_after_join'})
        identity = (uint(producer['producer_id'],positive=True),uint(producer['producer_generation'],positive=True))
        require(identity not in ids,'duplicate held producer'); ids.add(identity)
        require(uint(producer['native_pid'],U32,True)==pid,'foreign producer process')
        thread = producer['rust_thread_id']
        require(type(thread) is str and 0<len(thread)<=64 and thread.isascii() and thread.isprintable(),'thread diagnostic')
        require(len(compact(producer))<=2048,'producer fence row cap')
        require(producer['kind'] in ('Dispatcher','Supervisor','ObjectIoWorker') and producer['join_outcome'] in ('Returned','Panicked'),'raw fence enum spelling')
        require(boolean(producer['completed_join']) and not boolean(producer['live_ownership_after_join']),'unfinished producer')
        for name in ('selected_object_id','request_id','operation_id'):
            if producer[name] is not None: uint(producer[name],positive=name=='selected_object_id')
        if producer['settled_barrier'] is not None:
            raw_barrier(producer['settled_barrier']); require(producer['settled_barrier']['settled'],'unsettled joined barrier')
            require(producer['settled_barrier']['request_id']==producer['request_id'] and producer['settled_barrier']['operation_id']==producer['operation_id'],'joined barrier binding')
    require(all(p['kind'] in ('Dispatcher','Supervisor') for p in value['service_producers']) and all(p['kind']=='ObjectIoWorker' for p in value['object_workers']),'producer kind partition')
    objects = rows(value['objects'],2); require(len(objects)==2,'fence object inventory')
    require([o['selected_object_id'] for o in objects]==[31,32],'fence object ordering')
    for obj in objects:
        require(len(compact(obj))<=524288,'object fence row cap')
        keys(obj,{'owner_id','selected_object_id','selected_generation','service_epoch_at_fence','reserved_successor_epoch','durable_journal_sequence','durable_journal_hash','operation_high_water_min','writer_high_water_min','admission_epoch_min','known_durable_operations','issued_runtime_permits','original_closures'})
        for name in ('owner_id','selected_object_id','selected_generation','service_epoch_at_fence','durable_journal_sequence','admission_epoch_min'): uint(obj[name],positive=True)
        for name in ('operation_high_water_min','writer_high_water_min'): uint(obj[name])
        hash32(obj['durable_journal_hash'])
        expected = None if obj['service_epoch_at_fence']==U64 else obj['service_epoch_at_fence']+1
        require(obj['reserved_successor_epoch']==expected,'proposed successor epoch')
        for row in rows(obj['known_durable_operations'],16): operation(row)
        for row in rows(obj['issued_runtime_permits'],16): permit(row)
        for row in rows(obj['original_closures'],16):
            keys(row,{'allocation','submitted_binding','original_request_id','original_producer_id','original_producer_generation','closure'})
            allocation(row['allocation'])
            if row['submitted_binding'] is not None:
                binding(row['submitted_binding']); require(row['submitted_binding']['allocation']==row['allocation'],'closure binding')
            for name in ('original_request_id','original_producer_id','original_producer_generation'): uint(row[name],positive=True)
            require(row['closure'] in ('none','deadline','revoked','expired','service_retired'),'closure enum')


def fence_ref(value, epoch):
    keys(value,{'producer_epoch','relative_file','byte_len','sha256'})
    require(uint(value['producer_epoch'],U32)==epoch and value['relative_file']==f'epoch-{epoch}-fence.json','fence ref epoch/path')
    uint(value['byte_len'],2097152,True); hexhash(value['sha256'])


def native_descriptor(value, canonical=False):
    keys(value,{'handle','kind','object_generation','byte_len'})
    h = keys(value['handle'],{'kind','index','generation'})
    uint(h['kind'],3); uint(h['index'],U32); uint(h['generation'])
    require(h['kind']==2 and value['kind'] in ('SelectedText','DraftSource','Receipt'),'native descriptor kind')
    uint(value['object_generation']); uint(value['byte_len'],4160)
    if canonical:
        uint(h['index'],65535,True); uint(h['generation'],U32,True)
        require(value['object_generation']==h['generation'],'canonical descriptor equality')


def packed_native(value):
    h = value['handle']
    return (h['kind']<<56)|((h['index']&65535)<<32)|(h['generation']&U32)

# Fixed repository runtime projection, never an artifact-selected contract path.
HERE = Path(__file__).resolve().parent
REPO = HERE.parent.parent
RUNTIME_CONTRACT_SHA = 'aa3cc795374a9bfda490adec57750d8258d4d3589155f8fba50a1010ce3848be'
CONTRACT_PINS = {
    'editor-store-trusted-source-context-frozen.json': 'a8aa6aabe9e143393525ca009d42947a57d09d6ccd71ceaee331a348d86aa33d',
    'editor-store-recording-composite-frozen.json': 'b15b668f9bc8e55298ceb344144d491e370d2d314102020ad0058a501cce064c',
    'editor-store-recording-shapes-proposal.json': '5440da66c12a8cfbb911e020593dba9bd8c55ae59bc63763d6a91fc47dbdfbf2',
    'editor-store-evidence-addendum-proposal-v4.json': '1758bf6c8381ed62f22accd4d7cf9074a595ca45552e2931f14378832be1e347',
}
CONTRACT_PROJECTIONS = {
    'editor-store-trusted-source-context-frozen.json': 'trusted_source_context',
    'editor-store-recording-composite-frozen.json': 'composite',
    'editor-store-recording-shapes-proposal.json': 'shapes',
    'editor-store-evidence-addendum-proposal-v4.json': 'v4',
}


def compact(value):
    return json.dumps(value,ensure_ascii=False,separators=(',',':')).encode('utf8')


def read_regular(path, cap, expected_sha=None):
    fd=os.open(path,os.O_RDONLY|os.O_NOFOLLOW|os.O_CLOEXEC)
    try:
        before=os.fstat(fd)
        require(stat.S_ISREG(before.st_mode) and 0<before.st_size<=cap,'pinned regular-file admission')
        raw=bytearray()
        while len(raw)<before.st_size:
            block=os.read(fd,min(65536,before.st_size-len(raw)))
            require(bool(block),'truncated pinned file');raw.extend(block)
        require(not os.read(fd,1),'pinned file grew')
        after=os.fstat(fd)
        require(file_identity(before)==file_identity(after),'pinned file changed')
        if expected_sha is not None:require(sha(raw)==hexhash(expected_sha),'pinned SHA mismatch')
        return bytes(raw)
    finally:os.close(fd)


def file_identity(info):
    return (info.st_dev,info.st_ino,info.st_size,info.st_mtime_ns,info.st_ctime_ns,info.st_mode)


def pinned_contract(name):
    require(name in CONTRACT_PROJECTIONS,'unknown fixed contract projection')
    document=strict_json(read_regular(REPO/'docs/contracts/editor-store-recording-v0.json',131072,RUNTIME_CONTRACT_SHA))
    keys(document,{'schema_version','scope','basis','evidence_contract_sha256','assertion_source_sha256','composite','shapes','v4','trusted_source_context','run_inputs','overrides','assertion_corrections'})
    require(type(document['schema_version']) is int and document['schema_version']==1,'runtime contract version')
    require(document['evidence_contract_sha256']==CONTRACT_PINS['editor-store-recording-composite-frozen.json'],'runtime evidence identity')
    require(document['assertion_source_sha256']=='cc13bc8a25dd9cb5f58e01a5c9fa4379aeb4fb389e948c0dd39d7e2ef859c388','runtime reviewed assertion identity')
    correction=document['assertion_corrections']
    keys(correction,{'schema_version','baseline_assertion_sha256','method','proposal_sha256','lookup_wire','rules'})
    require(type(correction['schema_version']) is int and correction['schema_version']==1,'assertion correction version')
    require(correction['baseline_assertion_sha256']==document['assertion_source_sha256'] and correction['method']=='test_original_reconciled_receipt_no_replay','assertion correction baseline/method')
    return document[CONTRACT_PROJECTIONS[name]]


def schema(value, spec, definitions):
    """Small closed subset used by the independently frozen shape definitions."""
    if '$ref' in spec:
        ref = spec['$ref']; require(ref.startswith('#/$defs/'),'external schema ref')
        return schema(value,definitions[ref[8:]],definitions)
    if 'oneOf' in spec:
        matches = 0
        for alternative in spec['oneOf']:
            try: schema(value,alternative,definitions)
            except EvidenceDenied: pass
            else: matches += 1
        require(matches==1,'ambiguous/invalid schema union'); return
    if 'const' in spec:
        require(type(value) is type(spec['const']) and value==spec['const'],'wrong literal')
    if 'enum' in spec:
        require(any(type(value) is type(x) and value==x for x in spec['enum']),'unknown enum')
    kind = spec.get('type')
    if kind=='object':
        keys(value,spec['required'])
        for k,v in value.items(): schema(v,spec['properties'][k],definitions)
    elif kind=='array':
        require(type(value) is list and spec.get('minItems',0)<=len(value)<=spec.get('maxItems',32768),'array bound')
        for v in value: schema(v,spec['items'],definitions)
    elif kind=='integer': require(type(value) is int and spec.get('minimum',0)<=value<=spec.get('maximum',U64),'native integer range')
    elif kind=='boolean': boolean(value)
    elif kind=='string':
        require(type(value) is str and spec.get('minLength',0)<=len(value.encode('utf8'))<=spec.get('maxLength',4096),'UTF8 string bound')
        if 'pattern' in spec: require(re.fullmatch(spec['pattern'],value) is not None,'string pattern')
    elif kind=='null': require(value is None,'expected null')
    else: require(kind is None,'unsupported source schema')


def shape(value, name, definitions):
    schema(value,definitions[name],definitions)


def trusted_sources(path, trusted_sha, definitions):
    """Only the explicit root-controller CLI path/hash selects this trusted input."""
    hexhash(trusted_sha)
    require(type(path) is str and 0<len(path.encode('utf8'))<=4096,'trusted source manifest path bound')
    fd=os.open(path,os.O_RDONLY|os.O_NOFOLLOW)
    try:
        before=os.fstat(fd)
        require(stat.S_ISREG(before.st_mode) and 0<before.st_size<=524288,'trusted source manifest admission')
        raw=bytearray()
        while len(raw)<before.st_size:
            block=os.read(fd,min(65536,before.st_size-len(raw)))
            require(bool(block),'truncated trusted source manifest');raw.extend(block)
        require(not os.read(fd,1),'extra trusted source manifest bytes')
        after=os.fstat(fd)
        require((before.st_dev,before.st_ino,before.st_size,before.st_mtime_ns)==(after.st_dev,after.st_ino,after.st_size,after.st_mtime_ns),'trusted source manifest changed')
        require(sha(raw)==trusted_sha,'independently frozen source manifest SHA mismatch')
        manifest=strict_json(bytes(raw));require(bool(rows(manifest,1024)),'empty trusted closure')
        names=[];serialized_rows=0
        for record in manifest:
            shape(record,'source_record',definitions)
            encoded=compact(field_order(record,'source_record',definitions))
            require(len(encoded)<=512,'trusted source row cap before retention')
            serialized_rows+=len(encoded);require(serialized_rows<=524288,'trusted source rows aggregate')
            names.append(record['relative_path'])
        require(names==sorted(set(names)),'trusted exact sorted unique closure')
        canonical=compact([field_order(r,'source_record',definitions) for r in manifest])
        require(canonical==raw,'trusted manifest canonical declared-order bytes')
        return manifest
    finally:os.close(fd)


def field_order(value, name, definitions):
    """Typed diagnostic serialization follows declaration order, not received JSON order."""
    spec = definitions[name]
    def order(v,s):
        if '$ref' in s: return order(v,definitions[s['$ref'][8:]])
        if s.get('type')=='object': return {k:order(v[k],a) for k,a in s['properties'].items()}
        if s.get('type')=='array': return [order(z,s['items']) for z in v]
        return v
    return order(value,spec)


IO_POINTS = ('candidate_intent','candidate_blob_write','candidate_blob_sync','candidate_manifest','candidate_owner','candidate_directory_sync','candidate_readback','journal_write','journal_sync','journal_rename','journal_directory_sync','journal_readback')


def io_event(value, adapted=False):
    names = ('object','operation','writer') if adapted else ('selected_object_id','operation_id','writer_id')
    keys(value,set(names)|{'sequence','point','outcome','artifact_hash'})
    uint(value[names[0]],positive=True); uint(value[names[1]],positive=adapted); uint(value[names[2]])
    uint(value['sequence'],positive=True)
    require(value['point'] in IO_POINTS and value['outcome'] in ('began','completed','simulated_failure','actual_failure'),'unknown IO enum')
    if value['artifact_hash'] is not None: hash32(value['artifact_hash'])
    require(len(compact(value))<=512,'IO row cap')


def public(value, definitions):
    keys(value,{'exchanges','objects','barriers','io_events','ledger'})
    for e in rows(value['exchanges'],256):
        keys(e,{'actor','endpoint_class','selected_object_id','request_wire','actual_reply_wire','reply_suppressed','operation_id','elapsed_us','observed_ms'})
        actor(e['actor']); uint(e['selected_object_id'],positive=True)
        require(e['endpoint_class'] in ('artifact','recovery_receipt'),'exchange class')
        q,m = wire(e['request_wire']); uint(e['operation_id']); uint(e['elapsed_us']); uint(e['observed_ms'])
        boolean(e['reply_suppressed']); require(e['actual_reply_wire'] is not None,'missing actual internal reply')
        r,n = wire(e['actual_reply_wire'],True)
        require(n==m+1 and q[:4]==r[:4] and q[20:28]==r[20:28],'exchange correlation')
        reply_redaction(r,q)
        expected_op=struct.unpack_from('<Q',q,52 if m==5 else 44)[0] if m in (5,7) else 0
        require(e['operation_id']==expected_op,'public original operation binding')
        require(len(compact(e))<=2048,'exchange row cap')
    objects = rows(value['objects'],2); require(len(objects)==2,'objects absent')
    require([o['selected']['selected_object_id'] for o in objects]==[31,32],'object order')
    for o in objects:
        keys(o,{'selected','operations','mutation_dispatch_count','permit_count','transition_count','journal_hash','journal_sequence','blob_hash','manifest_hash','ownership_hash','mutation_quarantined','pending_writer_id','service_epoch','reopen_witness'})
        selected(o['selected']); uint(o['service_epoch'],positive=True); uint(o['journal_sequence'],positive=True)
        for k in ('mutation_dispatch_count','permit_count','transition_count'): uint(o[k],U32)
        require(o['transition_count']<=o['permit_count']<=o['mutation_dispatch_count'],'counter order')
        for k in ('journal_hash','blob_hash','manifest_hash','ownership_hash'): hash32(o[k])
        boolean(o['mutation_quarantined'])
        if o['pending_writer_id'] is not None: uint(o['pending_writer_id'],positive=True)
        ids = set()
        for op in rows(o['operations'],16):
            operation(op); require(len(compact(op))<=8192,'operation row bound')
            oid = op['allocation']['operation_id']; require(oid not in ids,'duplicate op'); ids.add(oid)
            require(op['allocation']['selected_object_id']==o['selected']['selected_object_id'] and op['allocation']['selected_generation']==o['selected']['selected_generation'],'operation object')
        if o['reopen_witness'] is not None: shape(o['reopen_witness'],'reopen_witness',definitions)
        for op in o['operations']:
            require(op['allocated_service_epoch']<=o['service_epoch'],'allocation current service epoch')
            if op['submitted_service_epoch'] is not None:require(op['submitted_service_epoch']<=o['service_epoch'],'submitted epoch not rewritten')
        active=sum(op['state']=='permitted' or (op['state']=='submitted' and op['closure']=='none') for op in o['operations'])
        require(active<=1,'at most one admitted object writer')
        require(len(compact(o))<=147456,'object evidence row')
    for b in rows(value['barriers'],64): raw_barrier(b); require(len(compact(b))<=1024,'barrier cap')
    previous = 0
    for event in rows(value['io_events'],1024):
        io_event(event); require(event['sequence']>previous,'IO sequence'); previous=event['sequence']
    l = keys(value['ledger'],{'sessions','selected_objects','original_instances','endpoints','shared_objects','active_io_workers','active_supervisors','held_service_producers','operations_per_object','cas_entries_per_object','bytes_per_object'})
    for k,cap in [('sessions',2),('selected_objects',2),('original_instances',16),('endpoints',64),('shared_objects',32),('active_io_workers',2),('active_supervisors',64),('held_service_producers',64)]: uint(l[k],cap)
    for k,cap in [('operations_per_object',16),('cas_entries_per_object',64),('bytes_per_object',2097152)]:
        pairs=rows(l[k],2); require([r[0] for r in pairs]==[31,32],'ledger object ordering')
        for pair in pairs:
            require(type(pair) is list and len(pair)==2,'ledger pair'); uint(pair[0],positive=True); uint(pair[1],cap)
    require(len(compact(l))<=4096,'ledger serialized cap')


def reply_status(raw):
    msg=struct.unpack_from('<I',raw,4)[0]
    return struct.unpack_from('<I',raw,20+{2:36,4:16,6:32,8:32}[msg])[0]


def reply_redaction(raw, request=None):
    status = reply_status(raw); msg=struct.unpack_from('<I',raw,4)[0]
    if status==0: return
    payload = bytearray(raw[20:20+WIRE_LENGTHS[msg]])
    offset={2:36,4:16,6:32,8:32}[msg]
    payload[:8]=bytes(8); payload[offset:offset+4]=bytes(4)
    # Only Conflict current revision or Unknown original op has nonzero failure data.
    if msg==6 and status==7: payload[16:24]=bytes(8)
    if msg in (6,8) and status==10:
        operation_id=struct.unpack_from('<Q',raw,28)[0]
        require(operation_id>0,'Unknown missing original operation')
        if request is not None:require(operation_id==struct.unpack_from('<Q',request,52 if msg==6 else 44)[0],'Unknown original operation correlation')
        payload[8:16]=bytes(8)
    require(not any(payload),'unredacted denied reply')


def obs_type(v, description):
    if description.startswith('const'):
        token=description[5:].strip()
        expected={'true':True,'false':False,'null':None}.get(token,token)
        if token.isdigit(): expected=int(token)
        require(type(v) is type(expected) and v==expected,'observation literal')
    elif description.startswith('integer'):
        match=re.fullmatch(r'integer(\d+)\.\.(\d+)',description); require(match is not None,'source integer description')
        uint(v,int(match[2])); require(v>=int(match[1]),'observation integer range')
    elif description.startswith('positive_u64'): uint(v,positive=True)
    elif description=='Hash32': hash32(v)
    elif description=='Receipt': receipt(v)
    elif description=='Binding': binding(v)
    elif description=='Permit': permit(v)
    elif description=='FenceRef': fence_ref(v,0)
    elif description=='Counts':
        require(type(v) is list and len(v)==3,'counts array'); [uint(i,U32) for i in v]
    elif description=='RetainedIds':
        require(type(v) is list and len(v)==16 and v==sorted(set(v)),'retained ID inventory'); [uint(i,positive=True) for i in v]
    elif description=='StoreBarrier':
        keys(v,{'request','operation','entered','released','settled','permit'})
        uint(v['request'],positive=True); uint(v['operation'],positive=True)
        require(v['entered'] is True and v['released'] is False and v['settled'] is False,'observed entry barrier')
        boolean(v['permit']); require(len(compact(v))<=1024,'observed barrier cap')
    elif description=='OperationIoList':
        require(bool(rows(v,256)),'empty original IO'); [io_event(i,True) for i in v]
        require(len(compact(v))<=131072,'operation IO cap')
    else: raise EvidenceDenied('unknown frozen observation type')


class FixtureData:
    def __init__(self, data, case, leg):
        self.data,self.case,self.leg=data,case,leg
        self.prefix=f'{case}/store-{leg}/'
        self.summary=self.json('actual.json',2097152)
        keys(self.summary,IDENTITY|{'backend','execution','claim_flags','epoch_records','actual_files','observations','lease_manifest_sha256','negative_calls_manifest_sha256'})
        data.identity(self.summary,case,leg)
        require(self.summary['backend']=='real_host_cas' and self.summary['execution']=='in_process_store_owner','wrong Store scope')
        keys(self.summary['claim_flags'],{'host_cas','integrated_editor_save','target','device_flush','power_loss','containment'})
        require(self.summary['claim_flags']['host_cas'] is True and all(self.summary['claim_flags'][k] is False for k in self.summary['claim_flags'] if k!='host_cas'),'broader claim')
        self.calls=self.json('calls.json',2097152); shape(self.calls,'calls_manifest',data.defs); data.identity(self.calls,case,leg)
        self.leases=self.json('leases.json',262144); keys(self.leases,IDENTITY|{'records'});data.identity(self.leases,case,leg)
        self.negative=self.json('negative-calls.json',524288 if (case,leg)==(CASE[3],0) else 262144)
        keys(self.negative,IDENTITY|{'negative_calls','lease_denials','reopen_attempt','state_witnesses'});data.identity(self.negative,case,leg)
        self.cleanup=self.json('cleanup.json',1048576);shape(self.cleanup,'cleanup_manifest',data.defs);data.identity(self.cleanup,case,leg)
        for filename,key in [('leases.json','lease_manifest_sha256'),('negative-calls.json','negative_calls_manifest_sha256')]:
            require(sha(self.raw(filename,524288))==hexhash(self.summary[key]),'manifest raw hash')
        required=next(r['required_fields_exact'] for r in data.v4['store_observation_schemas']['exact48leg_inventory'] if (r['case'],r['leg'])==(case,leg))
        keys(self.summary['observations'],required)
        for key,description in required.items(): obs_type(self.summary['observations'][key],description)
        require(len(compact(self.summary['observations']))<=1048576,'observation cap')
    def raw(self,name,cap): return self.data.store.read(self.prefix+name,cap)
    def json(self,name,cap): return self.data.store.json(self.prefix+name,cap)
    def epochs(self):
        ordinals=[0,1] if (self.case,self.leg) in REOPEN else [0]
        records=rows(self.summary['epoch_records'],2)
        require([r['producer_epoch'] for r in records]==ordinals,'epoch inventory')
        for row in records:
            shape(row,'epoch_record',self.data.defs); epoch=row['producer_epoch']
            fence_ref(row['fence_ref'],epoch)
            fr=self.raw(row['fence_ref']['relative_file'],2097152)
            require(len(fr)==row['fence_ref']['byte_len'] and sha(fr)==row['fence_ref']['sha256'],'raw fence reference')
            f=strict_json(fr); fence_shape(f,self.data.pid)
            require(row['namespace_id']==f['namespace_id'] and row['fence_id']==f['fence_id'] and row['profile_hash']==f['profile_hash'],'epoch fence identity')
            bindings=[{'object_id':o['selected_object_id'],'selected_generation':o['selected_generation'],'service_epoch':o['service_epoch_at_fence']} for o in f['objects']]
            require(row['object_service_epochs']==bindings,'fence epoch objects')
            pair=[]
            for stage in ('before_quiesce','after_join'):
                ref=row[stage];require(ref['relative_file']==f'epoch-{epoch}-{stage.replace("_","-")}.json','snapshot filename')
                raw=self.raw(ref['relative_file'],2097152)
                require(len(raw)==ref['byte_len'] and sha(raw)==ref['sha256'],'snapshot raw digest')
                s=strict_json(raw)
                keys(s,IDENTITY|{'producer_epoch','snapshot_stage','namespace_id','profile_hash','fence_id','object_service_epochs','public_evidence','fence_ref'})
                self.data.identity(s,self.case,self.leg)
                require(s['producer_epoch']==epoch and type(s['producer_epoch']) is int and s['snapshot_stage']==stage,'snapshot stage')
                for key in ('namespace_id','profile_hash','fence_id','object_service_epochs','fence_ref'): require(s[key]==row[key],'snapshot identity/ref binding')
                public(s['public_evidence'],self.data.defs)
                require([{'object_id':o['selected']['selected_object_id'],'selected_generation':o['selected']['selected_generation'],'service_epoch':o['service_epoch']} for o in s['public_evidence']['objects']]==bindings,'public object epoch')
                pair.append(s['public_evidence'])
            yield epoch,row,f,fr,pair[0],pair[1]


class Data:
    def __init__(self,args):
        self.opened=[]
        try:
            self.composite=pinned_contract('editor-store-recording-composite-frozen.json')
            self.shapes=pinned_contract('editor-store-recording-shapes-proposal.json');self.defs=self.shapes['$defs']
            pinned_contract('editor-store-trusted-source-context-frozen.json')
            self.trusted_manifest=trusted_sources(args.source_manifest,args.source_manifest_sha256,self.defs)
            self.v4=pinned_contract('editor-store-evidence-addendum-proposal-v4.json')
            self.store=Tree(args.store_evidence);self.opened.append(self.store)
            self.ui=Tree(args.ui_evidence);self.opened.append(self.ui)
            p=Path(args.provenance)
            require(re.fullmatch('[A-Za-z0-9_.-]{1,128}',p.name) is not None,'provenance filename')
            excluded=[(os.fstat(t.fd).st_dev,os.fstat(t.fd).st_ino) for t in (self.store,self.ui)]
            self.runner=Tree(str(p.parent),excluded);self.opened.append(self.runner)
            require(not any(a.path.resolve()==b.path.resolve() or a.path.resolve() in b.path.resolve().parents or b.path.resolve() in a.path.resolve().parents for a,b in [(self.store,self.ui)]),'Store/UI roots overlap')
            require(all(self.runner.path.resolve()!=t.path.resolve() and t.path.resolve() not in self.runner.path.resolve().parents for t in (self.store,self.ui)),'runner cannot be within Store/UI subtree')
            require(sum(t.total_bytes for t in self.opened)<=1073741824 and sum(len(t.inventory)+t.directories for t in self.opened)<=32768 and sum(len(t.inventory) for t in self.opened)<=20436 and sum(t.directories for t in self.opened)<=96,'global Store+UI+runner admission')
            self.provenance=self.runner.json(p.name,2097152);shape(self.provenance,'runner_provenance',self.defs)
            self.pid=self.provenance['test_process']['native_pid'];self.birth=self.provenance['test_process']['start_identity'];self.nonce=self.provenance['run_nonce']
            require(self.provenance['sources']==self.trusted_manifest,'provenance differs from independently frozen exact source closure')
            require(self.provenance['accepted_source_manifest_sha256']==args.source_manifest_sha256,'provenance trusted source SHA mismatch')
            self.sources={r['relative_path']:r for r in self.trusted_manifest}
            require(len(self.sources)==len(self.provenance['sources']),'duplicate sources')
        except BaseException:
            self.close();raise
    def close(self):
        for tree in reversed(self.opened): tree.close()
        self.opened=[]
    def identity(self,v,case,leg):
        require(type(v['schema_version']) is int and v['schema_version']==1 and v['run_nonce']==self.nonce and v['case']==case and type(v['leg']) is int and v['leg']==leg,'identity mismatch')
        require(type(v['native_test_pid']) is int and v['native_test_pid']==self.pid and v['test_process_start_identity']==self.birth,'process identity mismatch')
        require(type(v['fixture_id']) is str and re.fullmatch('[0-9a-f]{32}',v['fixture_id']) is not None,'fixture ID')
    def fixtures(self):
        for case,n in zip(CASE,LEGS):
            for leg in range(n):yield FixtureData(self,case,leg)
    def fixture(self,case,leg):return FixtureData(self,case,leg)


def observations(fixture):return fixture.summary['observations']


def matching(fixture):
    records=fixture.calls['records']; require(len(records)<=512,'call cap')
    require([c['call_ordinal'] for c in records]==list(range(len(records))),'invocation ordinals')
    require(sorted(c['observed_completion_ordinal'] for c in records)==list(range(len(records))),'observed completion ordinals')
    require(all(n<=256 for n in collections.Counter(c['producer_epoch'] for c in records).values()),'calls epoch cap')
    require(len(compact({k:v for k,v in fixture.calls.items() if k!='records'}))<=4096,'calls header cap')
    for epoch,_,_,_,_,after in fixture.epochs():
        exchanges=after['exchanges'];used=set()
        for call in (c for c in records if c['producer_epoch']==epoch):
            require(len(compact(call))<=2048,'call row cap')
            shape(call,'call_record',fixture.data.defs)
            actor(call['actor'])
            if call['input_kind']=='canonical_request_wire':
                require(call['typed_envelope_or_null'] is None and call['request_wire_or_null'] is not None,'canonical input union')
                q,msg=wire(call['request_wire_or_null']);require(sha(q)==call['input_diagnostic_sha256'],'call raw input digest')
            else:
                require(call['request_wire_or_null'] is None and call['typed_envelope_or_null'] is not None,'typed input union')
                typed=call['typed_envelope_or_null'];shape(typed,'typed_envelope',fixture.data.defs)
                require(sha(compact(field_order(typed,'typed_envelope',fixture.data.defs)))==call['input_diagnostic_sha256'],'native input diagnostic digest')
                q=None
            result=call['actual_result'];index=call['matched_public_exchange_index_or_null']
            if result['kind']=='error':
                require(index is None,'error cannot invent public row');continue
            r,n=wire(result['returned_wire'],True);reply_redaction(r,q)
            require(q is not None,'typed-invalid input cannot fabricate successful canonical wire')
            require(n==msg+1 and q[20:28]==r[20:28],'caller reply correlation')
            require(index is not None and index<len(exchanges) and index not in used,'missing/repeated match representative')
            e=exchanges[index]; used.add(index)
            require(call['actor']==e['actor'] and call['endpoint_class']==e['endpoint_class'] and call['selected_object_id']==e['selected_object_id'] and list(q)==e['request_wire'],'actual passed issuer context/multiplicity')
            if e['reply_suppressed']:
                require(reply_status(r)==9,'suppressed caller result');reply_redaction(r)
            else:require(list(r)==e['actual_reply_wire'],'caller/internal exact response')
        require(used==set(range(len(exchanges))),'unmatched original public exchange')
    return records


def lease_records(fixture):
    records=rows(fixture.leases['records'],256)
    require([r['ordinal'] for r in records]==list(range(len(records))),'lease ordinals')
    require(len(compact({k:v for k,v in fixture.leases.items() if k!='records'}))<=4096,'lease header')
    calls=matching(fixture)
    fields={'ordinal','producer_epoch','call_ordinal','endpoint_class','actor','selected_object_id','kind','descriptor_raw_handle','descriptor_generation','descriptor_byte_len','actual_request_wire_sha256','actual_reply_wire_sha256','relative_file','actual_bytes','sha256'}
    for r in records:
        keys(r,fields);require(len(compact(r))<=1000,'lease row cap')
        uint(r['ordinal'],255);uint(r['producer_epoch'],1);uint(r['call_ordinal'],511);actor(r['actor'])
        require(r['call_ordinal']<len(calls),'lease call missing');c=calls[r['call_ordinal']]
        require(c['producer_epoch']==r['producer_epoch'] and c['actor']==r['actor'] and c['endpoint_class']==r['endpoint_class'] and c['selected_object_id']==r['selected_object_id'],'lease actual issuer/call binding')
        require(c['actual_result']['kind']=='ok_envelope' and c['matched_public_exchange_index_or_null'] is not None,'lease from actual Ok caller only')
        q,msg=wire(c['request_wire_or_null']);response,n=wire(c['actual_result']['returned_wire'],True)
        require(reply_status(response)==0,'lease from failed caller/suppression')
        require(sha(q)==hexhash(r['actual_request_wire_sha256']) and sha(response)==hexhash(r['actual_reply_wire_sha256']),'lease actual frame digest')
        handle=canonical_handle(r['descriptor_raw_handle'],2)
        require(uint(r['descriptor_generation'],U32,True)==(handle&U32),'descriptor packed generation')
        require(r['relative_file']==f'lease-{r["ordinal"]}.bin','lease file naming')
        raw=fixture.raw(r['relative_file'],4160)
        require(len(raw)==r['actual_bytes']==r['descriptor_byte_len'] and sha(raw)==hexhash(r['sha256']),'actual copied bytes')
        if r['kind']=='selected_text':
            require(msg==1 and r['endpoint_class']=='artifact','text grant route')
            revision,shm,generation=struct.unpack_from('<QQQ',response,28);length=struct.unpack_from('<I',response,52)[0]
            obj,rev,body,digest=text_bytes(raw)
            require(shm==handle and generation==r['descriptor_generation'] and length==len(raw) and revision==rev and obj==r['selected_object_id'],'read reply/header exact binding')
        else:
            require(r['kind']=='receipt' and msg in (5,7) and r['endpoint_class'] in ('artifact','recovery_receipt'),'receipt route')
            rr=receipt_bytes(raw);shm=struct.unpack_from('<Q',response,44)[0];length=struct.unpack_from('<I',response,56)[0]
            require(shm==handle and length==176 and rr['selected_object_id']==r['selected_object_id'],'receipt descriptor/object')
            if msg==5:require(rr['outcome']=='committed','original CommitOk outcome')
            operation_id=struct.unpack_from('<Q',response,28)[0]
            require(struct.unpack_from('<Q',response,36)[0]==rr['result_revision'],'receipt reply revision')
            require(operation_id==rr['operation_id'] and struct.unpack_from('<Q',q,52 if msg==5 else 44)[0]==rr['operation_id'],'receipt actual operation')
            for a,b in [('owner_id','owner_id'),('session_id','session_id'),('session_generation','session_generation')]:require(rr[a]==r['actor'][b],'receipt owner/session')
        yield r,raw


def state_witnesses(f):
    witnesses=rows(f.negative['state_witnesses'],5)
    expected=[]
    if (f.case,f.leg)==(CASE[3],0): expected+=['selectedtext_before_allocation','draft_after_invalid_submissions','receipt_after_commit']
    if (f.case,f.leg) in REOPEN|FAILED_REOPEN: expected+=['before_reopen','after_reopen']
    require([s['phase'] for s in witnesses]==expected,'complete state phase inventory')
    for s in witnesses:
        shape(s,'state_witness',f.data.defs)
        cap=40960 if s['phase'] not in ('before_reopen','after_reopen') else 122880
        require(len(compact(s))<=cap,'state phase cap')
        body=s['state']; require(sha(compact(field_order(body,'state_body',f.data.defs)))==s['state_sha256'],'typed complete state digest')
        require(body['run_nonce']==f.data.nonce and body['fixture_id']==f.summary['fixture_id'],'state same fixture')
        require(body['root_device']==f.cleanup['root_device'] and body['root_inode']==f.cleanup['root_inode'],'outer TempDir state identity')
        require(body['namespace_id']==f.summary['epoch_records'][0]['namespace_id'],'state same reopened namespace')
        paths=[r['relative_path'] for r in body['files']]
        require(paths==sorted(set(paths),key=lambda p:p.encode('utf8')),'state complete sorted unique owner-relative inventory')
        for r in body['files']:
            require(not Path(r['relative_path']).is_absolute() and all(p not in ('','.', '..') for p in r['relative_path'].split('/')) and '\x00' not in r['relative_path'],'state owner child path')
            require(len(compact(r))<=512,'state row cap')
        require([r['object_id'] for r in body['objects']]==[31,32],'state objects')
        for o in body['objects']:require(o['transitions']<=o['permits']<=o['dispatches'],'state mutation counters')
        expected_epoch=1 if s['phase']=='after_reopen' and (f.case,f.leg) in REOPEN else 0
        require(s['producer_epoch']==expected_epoch and type(s['producer_epoch']) is int,'state owner epoch')
    if (f.case,f.leg)==(CASE[3],0):
        require(sum(len(s['state']['files']) for s in witnesses[:3])<=160,'lease phases file rows cap')
        require(sum(len(compact(s)) for s in witnesses[:3])<=122880,'lease phase combined cap')
        require(len({s['state_sha256'] for s in witnesses[:3]})==3,'actual advancing lease phases not aliased')
    if expected[-2:]==['before_reopen','after_reopen']:
        require(sum(len(compact(s)) for s in witnesses[-2:])<=245760,'two reopen phase bound')
    return {s['phase']:s for s in witnesses}


def cleanup_files(f):
    records=f.cleanup['emitted_files'];require([r['relative_file'] for r in records]==sorted(set(r['relative_file'] for r in records)),'cleanup sorted exact files')
    actual={path[len(f.prefix):] for path in f.data.store.inventory if path.startswith(f.prefix)}
    require(actual=={r['relative_file'] for r in records}|{'cleanup.json'},'cleanup exact emitted file inventory')
    require(len(records)<=426 and sum(len(compact(r)) for r in records)<=131072,'cleanup emitted row totals')
    for r in records:
        require(len(compact(r))<=192,'emitted row cap')
        raw=f.raw(r['relative_file'],2097152)
        require(len(raw)==r['byte_len'] and sha(raw)==r['sha256'],'cleanup raw file SHA/length')


def get_ui(data,case,index):
    prefix=case+'/'
    raw=data.ui.read(prefix+f'registry-{index}.json',1048576);record=strict_json(raw)
    keys(record,{'schema_version','case','registry_index','backend','execution','font_sha256','api_sha256','test_sha256','support_sha256','exchanges','frames','observations','mutation_dispatch_count','permit_count','transition_count','ledger'})
    require(type(record['schema_version']) is int and record['schema_version']==1 and record['case']==case and type(record['registry_index']) is int and record['registry_index']==index,'UI record identity')
    require(record['backend']=='volatile' and record['execution']=='in_process','UI scope')
    for key,pin in [('font_sha256',FONT),('api_sha256','b8ae6420a6e7ea611c5553528ad1de87d06bdc6014d346c2a26b7c7f6c3625fe'),('test_sha256','28d9415115b669bec0e2395f331b6d762b4a67db29745779c75920fbd9e7adc0'),('support_sha256','6d0d0101dc8f65806b22a28fb98baa0d0c00003c9d334b23b41ecd32cfaa80df')]:require(record[key]==pin,'UI reused frozen source pin')
    notes=rows(record['observations'],4)
    require(all(type(n) is dict and set(n)=={'name','value'} for n in notes),'UI note fields')
    require(len(set(n['name'] for n in notes))==len(notes),'duplicate UI note')
    expected=next(r['required_note_values'] for r in data.v4['ui_a_observation_schemas']['exact5registry_inventory'] if (r['case'],r['registry_index'])==(case,index))
    note_map={n['name']:n['value'] for n in notes};require(set(note_map)==set(expected),'UI exact note inventory')
    for name,fields in expected.items():
        v=keys(note_map[name],fields)
        for k,desc in fields.items():
            if desc.startswith('const'):obs_type(v[k],desc)
            elif desc=='bool':boolean(v[k])
            elif desc.startswith('positive'):uint(v[k],U32 if 'u32' in desc else U64,True)
            elif desc.startswith('u64'):uint(v[k])
            elif k in ('run_nonce','ui_fixture_id'):require(type(v[k]) is str and re.fullmatch('[0-9a-f]{32}',v[k]) is not None,'UI nonce')
            elif k=='test_process_start_identity':require(type(v[k]) is str and len(v[k])<=128 and v[k].isascii(),'UI process birth')
            elif k=='case':require(v[k]==case,'UI note case')
            elif k=='registry_index':require(type(v[k]) is int and v[k]==index,'UI note index')
            else:hexhash(v[k])
        require(len(compact(v))<=4096,'UI note bytes')
    b=note_map['b_run_binding']
    require(b['run_nonce']==data.nonce and b['native_test_pid']==data.pid and b['test_process_start_identity']==data.birth,'actual executing Store process UI binding')
    require(b['executing_store_test_sha256']==data.sources['services/store_service/tests/editor_store.rs']['sha256'] and b['executing_test_binary_sha256']==data.provenance['test_process']['test_binary']['sha256'],'UI executing Store binary/source identity')
    for e in rows(record['exchanges'],256):
        keys(e,{'request_hex','reply_hex','request_canonical'});boolean(e['request_canonical'])
        require(type(e['request_hex']) is str and re.fullmatch('[0-9a-f]{176}',e['request_hex']) is not None and type(e['reply_hex']) is str and re.fullmatch('[0-9a-f]{176}',e['reply_hex']) is not None,'UI raw88hex')
        q=bytes.fromhex(e['request_hex']);r=bytes.fromhex(e['reply_hex'])
        ui_exchange(q,r,e['request_canonical'])
        require(not any(r[84:]) and struct.unpack_from('<I',r,16)[0]<=64 and not any(r[20+struct.unpack_from('<I',r,16)[0]:84]),'UI canonical reply pad/tail')
    require(len(record['frames'])==UI_INVENTORY[(case,index)],'UI frames count')
    for i,fr in enumerate(record['frames']):
        keys(fr,{'file','frame_kind','session_id','instance_id','focus_epoch','sequence','sha256','crop_sha256'})
        require(fr['file']==f'registry-{index}-frame-{i}.bgra','frame filename')
        for k in ('session_id','instance_id'):uint(fr[k],positive=True)
        uint(fr['focus_epoch']);uint(fr['sequence'])
        require((fr['frame_kind']=='app' and fr['focus_epoch']>0 and fr['sequence']>0) or (fr['frame_kind']=='recovery' and fr['focus_epoch']==fr['sequence']==0),'frame origin/sentinel')
        pixels=data.ui.read(prefix+fr['file'],640*568*4);require(len(pixels)==640*568*4 and sha(pixels)==hexhash(fr['sha256']),'actual frame file')
        crop=pixels[48*640*4:528*640*4]
        require(sha(crop)==hexhash(fr['crop_sha256']),'actual owned app crop')
        require(all(pixels[j+3]==255 for j in range(0,len(pixels),4)),'BGRA alpha')
        if fr['frame_kind']=='recovery':require(crop==b'\xff'*(640*480*4),'terminal recovery app area blank')
    for k in ('mutation_dispatch_count','permit_count','transition_count'):uint(record[k],U32)
    require(record['transition_count']<=record['permit_count']<=record['mutation_dispatch_count'],'UI mutation order')
    l=keys(record['ledger'],{'sessions','selected_objects','instances','endpoints','shared_objects','queued_keys','operations_per_object'})
    for k,cap in [('sessions',2),('selected_objects',2),('instances',16),('endpoints',64),('shared_objects',32),('queued_keys',128)]:uint(l[k],cap)
    for pair in rows(l['operations_per_object'],2):
        require(type(pair) is list and len(pair)==2,'UI operation ledger');uint(pair[0],positive=True);uint(pair[1],16)
    return raw,record,note_map


UI_LAYOUTS = {
    (352, 1): (64, {'request_id': (0, 8), 'session_id': (8, 8), 'session_generation': (16, 8), 'application_hash': (24, 32), 'requested_rights': (56, 4), 'reserved': (60, 4)}),
    (352, 2): (48, {'request_id': (0, 8), 'plan_id': (8, 8), 'preview_revision': (16, 8), 'preview_shm': (24, 8), 'preview_len': (32, 4), 'status': (36, 4), 'granted_rights': (40, 4), 'reserved': (44, 4)}),
    (352, 3): (32, {'request_id': (0, 8), 'session_id': (8, 8), 'session_generation': (16, 8), 'plan_id': (24, 8)}),
    (352, 4): (16, {'request_id': (0, 8), 'status': (8, 4), 'reserved': (12, 4)}),
    (352, 5): (40, {'request_id': (0, 8), 'session_id': (8, 8), 'session_generation': (16, 8), 'plan_id': (24, 8), 'preview_revision': (32, 8)}),
    (352, 6): (48, {'request_id': (0, 8), 'session_generation': (8, 8), 'instance_id': (16, 8), 'instance_generation': (24, 8), 'grants_shm': (32, 8), 'grants_len': (40, 4), 'status': (44, 4)}),
    (352, 7): (40, {'request_id': (0, 8), 'session_id': (8, 8), 'session_generation': (16, 8), 'instance_id': (24, 8), 'instance_generation': (32, 8)}),
    (352, 8): (48, {'request_id': (0, 8), 'instance_id': (8, 8), 'instance_generation': (16, 8), 'focus_epoch': (24, 8), 'service_epoch': (32, 8), 'state': (40, 4), 'status': (44, 4)}),
    (352, 9): (40, {'request_id': (0, 8), 'session_id': (8, 8), 'session_generation': (16, 8), 'instance_id': (24, 8), 'instance_generation': (32, 8)}),
    (352, 10): (16, {'request_id': (0, 8), 'status': (8, 4), 'reserved': (12, 4)}),
    (352, 11): (40, {'request_id': (0, 8), 'session_id': (8, 8), 'session_generation': (16, 8), 'instance_id': (24, 8), 'instance_generation': (32, 8)}),
    (352, 12): (16, {'request_id': (0, 8), 'status': (8, 4), 'reserved': (12, 4)}),
    (352, 13): (40, {'request_id': (0, 8), 'session_id': (8, 8), 'session_generation': (16, 8), 'instance_id': (24, 8), 'instance_generation': (32, 8)}),
    (352, 14): (48, {'request_id': (0, 8), 'plan_id': (8, 8), 'preview_revision': (16, 8), 'preview_shm': (24, 8), 'preview_len': (32, 4), 'status': (36, 4), 'granted_rights': (40, 4), 'reserved': (44, 4)}),
    (352, 15): (40, {'request_id': (0, 8), 'session_id': (8, 8), 'session_generation': (16, 8), 'instance_id': (24, 8), 'instance_generation': (32, 8)}),
    (352, 16): (32, {'request_id': (0, 8), 'focus_epoch': (8, 8), 'service_epoch': (16, 8), 'status': (24, 4), 'reserved': (28, 4)}),
    (352, 17): (48, {'session_id': (0, 8), 'session_generation': (8, 8), 'instance_id': (16, 8), 'instance_generation': (24, 8), 'grants_shm': (32, 8), 'grants_len': (40, 4), 'status': (44, 4)}),
    (368, 1): (40, {'request_id': (0, 8), 'session_id': (8, 8), 'session_generation': (16, 8), 'instance_id': (24, 8), 'instance_generation': (32, 8)}),
    (368, 2): (40, {'request_id': (0, 8), 'revision': (8, 8), 'data_shm': (16, 8), 'object_generation': (24, 8), 'byte_len': (32, 4), 'status': (36, 4)}),
    (368, 3): (32, {'request_id': (0, 8), 'session_id': (8, 8), 'session_generation': (16, 8), 'expected_revision': (24, 8)}),
    (368, 4): (24, {'request_id': (0, 8), 'operation_id': (8, 8), 'status': (16, 4), 'reserved': (20, 4)}),
    (368, 5): (56, {'request_id': (0, 8), 'session_id': (8, 8), 'session_generation': (16, 8), 'expected_revision': (24, 8), 'operation_id': (32, 8), 'source_shm': (40, 8), 'byte_len': (48, 4), 'reserved': (52, 4)}),
    (368, 6): (40, {'request_id': (0, 8), 'operation_id': (8, 8), 'revision': (16, 8), 'receipt_shm': (24, 8), 'status': (32, 4), 'receipt_len': (36, 4)}),
    (368, 7): (32, {'request_id': (0, 8), 'session_id': (8, 8), 'session_generation': (16, 8), 'operation_id': (24, 8)}),
    (368, 8): (40, {'request_id': (0, 8), 'operation_id': (8, 8), 'revision': (16, 8), 'receipt_shm': (24, 8), 'status': (32, 4), 'receipt_len': (36, 4)}),
    (802, 1): (32, {'request_id': (0, 8), 'session_id': (8, 8), 'session_generation': (16, 8), 'device_generation': (24, 8)}),
    (802, 2): (24, {'request_id': (0, 8), 'queue_generation': (8, 8), 'status': (16, 4), 'reserved': (20, 4)}),
    (802, 3): (48, {'session_id': (0, 8), 'session_generation': (8, 8), 'queue_generation': (16, 8), 'sequence': (24, 8), 'usage': (32, 4), 'phase': (36, 4), 'modifiers': (40, 4), 'reserved': (44, 4)}),
    (802, 4): (16, {'sequence': (0, 8), 'status': (8, 4), 'reserved': (12, 4)}),
    (832, 1): (56, {'request_id': (0, 8), 'session_id': (8, 8), 'session_generation': (16, 8), 'instance_id': (24, 8), 'instance_generation': (32, 8), 'surface_id': (40, 8), 'surface_generation': (48, 8)}),
    (832, 2): (32, {'request_id': (0, 8), 'focus_epoch': (8, 8), 'service_epoch': (16, 8), 'status': (24, 4), 'reserved': (28, 4)}),
    (832, 3): (48, {'request_id': (0, 8), 'session_id': (8, 8), 'session_generation': (16, 8), 'instance_id': (24, 8), 'instance_generation': (32, 8), 'focus_epoch': (40, 8)}),
    (832, 4): (40, {'request_id': (0, 8), 'sequence': (8, 8), 'focus_epoch': (16, 8), 'usage': (24, 4), 'phase': (28, 4), 'modifiers': (32, 4), 'status': (36, 4)}),
    (833, 1): (56, {'request_id': (0, 8), 'session_id': (8, 8), 'session_generation': (16, 8), 'instance_id': (24, 8), 'instance_generation': (32, 8), 'width': (40, 4), 'height': (44, 4), 'format': (48, 4), 'reserved': (52, 4)}),
    (833, 2): (56, {'request_id': (0, 8), 'surface_id': (8, 8), 'surface_generation': (16, 8), 'buffer0_shm': (24, 8), 'buffer1_shm': (32, 8), 'status': (40, 4), 'stride': (44, 4), 'format': (48, 4), 'reserved': (52, 4)}),
    (833, 3): (32, {'request_id': (0, 8), 'surface_id': (8, 8), 'surface_generation': (16, 8), 'buffer_index': (24, 4), 'reserved': (28, 4)}),
    (833, 4): (32, {'request_id': (0, 8), 'mapping_generation': (8, 8), 'buffer_shm': (16, 8), 'status': (24, 4), 'byte_len': (28, 4)}),
    (833, 5): (48, {'request_id': (0, 8), 'surface_id': (8, 8), 'surface_generation': (16, 8), 'mapping_generation': (24, 8), 'sequence': (32, 8), 'buffer_index': (40, 4), 'reserved': (44, 4)}),
    (833, 6): (24, {'request_id': (0, 8), 'sequence': (8, 8), 'status': (16, 4), 'reserved': (20, 4)}),
    (833, 7): (32, {'request_id': (0, 8), 'surface_id': (8, 8), 'surface_generation': (16, 8), 'sequence': (24, 8)}),
    (833, 8): (24, {'request_id': (0, 8), 'sequence': (8, 8), 'status': (16, 4), 'reserved': (20, 4)}),
    (833, 9): (24, {'request_id': (0, 8), 'surface_id': (8, 8), 'surface_generation': (16, 8)}),
    (833, 10): (16, {'request_id': (0, 8), 'status': (8, 4), 'reserved': (12, 4)}),
}

def ui_frame(raw):
    require(len(raw)==88 and not any(raw[84:]),'UI88/outer pad')
    protocol,msg,handle,length=struct.unpack_from('<IIQI',raw)
    require((protocol,msg) in UI_LAYOUTS,'unsupported UI layout')
    expected,fields=UI_LAYOUTS[(protocol,msg)]
    require(length==expected and not any(raw[20+length:84]),'UI exact length/tail')
    if handle:canonical_handle(handle)
    for name,(offset,size) in fields.items():
        if name=='reserved':require(not any(raw[20+offset:20+offset+size]),'UI reserved')
    return protocol,msg,fields


def ui_exchange(q,r,claimed_canonical):
    try:qp,qm,qfields=ui_frame(q)
    except EvidenceDenied:
        require(claimed_canonical is False,'false canonical classification')
        qp,qm=struct.unpack_from('<II',q);qfields=None
    else:require(claimed_canonical is True,'false noncanonical classification')
    rp,rm,fields=ui_frame(r)
    require('status' in fields and rm%2==0,'UI reply direction')
    status_offset=fields['status'][0];status=struct.unpack_from('<I',r,20+status_offset)[0]
    require(status<=11,'UI status')
    if qfields is not None:
        require(qm%2==1 and rp==qp and rm==qm+1,'UI paired direction/protocol')
        correlation=q[44:52] if (qp,qm)==(802,3) else q[20:28]
        require(r[20:28]==correlation,'UI actual correlation')
    if status:
        extra=set()
        if status==7:
            require(qfields is not None and (qp,qm,rp,rm)==(368,5,368,6),'Conflict scope');extra.update(range(16,24))
        if status==10:
            require(qfields is not None and qp==rp==368 and (qm,rm) in ((5,6),(7,8)),'Unknown scope')
            operation_id=struct.unpack_from('<Q',r,28)[0]
            require(operation_id>0 and operation_id==struct.unpack_from('<Q',q,52 if qm==5 else 44)[0],'Unknown own operation');extra.update(range(8,16))
        require(not any(byte for i,byte in enumerate(r[20:84]) if not (i<8 or status_offset<=i<status_offset+4 or i in extra)),'UI failure redaction')
    return status


class RecordingContractTests(unittest.TestCase):
    def setUp(self):
        require(ARGS is not None,'required actual evidence CLI inputs missing')
        self.data=Data(ARGS);self.addCleanup(self.data.close)

    def test_case_leg_inventory(self):
        expected={f'{case}/store-{leg}/actual.json' for case,n in zip(CASE,LEGS) for leg in range(n)}
        actual={p for p in self.data.store.inventory if p.endswith('/actual.json')}
        self.assertEqual(actual,expected)
        ids=[]
        for f in self.data.fixtures():
            ids.append(f.summary['fixture_id'])
            for manifest in (f.calls,f.leases,f.negative,f.cleanup):self.assertEqual(manifest['fixture_id'],f.summary['fixture_id'])
            self.assertEqual(f.cleanup['fixture_filesystem']['fixture_id'],f.summary['fixture_id'])
        self.assertEqual(len(ids),48);self.assertEqual(len(set(ids)),48)
        self.assertEqual({p for p in self.data.ui.inventory if re.search(r'/registry-\d+[.]json$',p)}, {f'{c}/registry-{i}.json' for c,i in UI_INVENTORY})

    def test_epoch_before_after_inventory(self):
        snapshots,fences,epochs=set(),set(),0
        for f in self.data.fixtures():
            for epoch,row,_,_,_,_ in f.epochs():
                epochs+=1
                for stage in ('before_quiesce','after_join'):snapshots.add(f.prefix+row[stage]['relative_file'])
                fences.add(f.prefix+row['fence_ref']['relative_file'])
            all_files={p for p in self.data.store.inventory if p.startswith(f.prefix) and '/epoch-' in p}
            wanted={p for p in snapshots|fences if p.startswith(f.prefix)}
            self.assertEqual(all_files,wanted)
        self.assertEqual(epochs,56);self.assertEqual(len(snapshots),112);self.assertEqual(len(fences),56)

    def test_original_epoch_trace_preserved(self):
        for f in self.data.fixtures():
            traces=[]
            for epoch,_,_,_,before,after in f.epochs():
                for key in ('exchanges','io_events'):
                    self.assertEqual(after[key][:len(before[key])],before[key],f'{f.prefix} original {key} retained')
                for b in before['barriers']:
                    same=[a for a in after['barriers'] if (a['request_id'],a['operation_id'])==(b['request_id'],b['operation_id'])]
                    self.assertEqual(len(same),1)
                    for k in ('entered','released','settled','permit_issued'):
                        self.assertFalse(b[k] and not same[0][k],'barrier history regressed')
                for old,new in zip(before['objects'],after['objects']):
                    retained={o['allocation']['operation_id']:o for o in new['operations']}
                    for o in old['operations']:
                        self.assertIn(o['allocation']['operation_id'],retained)
                        self.assertEqual(o['allocation'],retained[o['allocation']['operation_id']]['allocation'])
                        if o['binding'] is not None:self.assertEqual(o['binding'],retained[o['allocation']['operation_id']]['binding'])
                        if o['permit'] is not None:self.assertEqual(o['permit'],retained[o['allocation']['operation_id']]['permit'])
                        if o['receipt'] is not None:self.assertEqual(o['receipt'],retained[o['allocation']['operation_id']]['receipt'])
                traces.append(after)
            matching(f)
            if len(traces)==2:
                self.assertTrue(any(c['producer_epoch']==0 for c in f.calls['records']))
                self.assertTrue(any(c['producer_epoch']==1 for c in f.calls['records']))
                self.assertFalse(any(struct.unpack_from('<I',bytes(e['request_wire']),4)[0]==5 for e in traces[1]['exchanges']),'reopen is observation, not recommit')

    def test_raw_fence_digest_join_bindings(self):
        for f in self.data.fixtures():
            for epoch,row,fence,raw,_,after in f.epochs():
                self.assertEqual(row['fence_ref']['sha256'],sha(raw))
                self.assertEqual(fence['live_ownership_after_join'],0)
                self.assertEqual(after['ledger']['active_io_workers'],0)
                self.assertEqual(after['ledger']['active_supervisors'],0)
                for p in fence['service_producers']+fence['object_workers']:
                    self.assertTrue(p['completed_join']);self.assertFalse(p['live_ownership_after_join']);self.assertEqual(p['native_pid'],self.data.pid)
                for w,o in zip(fence['objects'],after['objects']):
                    self.assertEqual((w['durable_journal_sequence'],w['durable_journal_hash']),(o['journal_sequence'],o['journal_hash']))
                    # Runtime retained operations can exceed acknowledged durable subset.
                    byid={op['allocation']['operation_id']:op for op in o['operations']}
                    containing=(w['owner_id'],w['selected_object_id'],w['selected_generation'])
                    self.assertEqual(containing,(o['selected']['owner_id'],o['selected']['selected_object_id'],o['selected']['selected_generation']))
                    for op in w['known_durable_operations']:
                        allocation=op['allocation'];self.assertIn(allocation['operation_id'],byid)
                        runtime=byid[allocation['operation_id']]
                        self.assertEqual((allocation['actor']['owner_id'],allocation['selected_object_id'],allocation['selected_generation']),containing)
                        self.assertEqual(allocation,runtime['allocation'])
                        self.assertEqual(op['allocated_service_epoch'],runtime['allocated_service_epoch'])
                        # A durable subset may be earlier than runtime; preserve every observed tuple.
                        for field in ('binding','permit','receipt','successor'):
                            if op[field] is not None:self.assertEqual(op[field],runtime[field],f'durable original {field} changed')
                        if op['submitted_service_epoch'] is not None:self.assertEqual(op['submitted_service_epoch'],runtime['submitted_service_epoch'])
                    for p in w['issued_runtime_permits']:
                        allocation=p['binding']['allocation']
                        self.assertEqual((allocation['actor']['owner_id'],allocation['selected_object_id'],allocation['selected_generation']),containing)
                        self.assertIn(allocation['operation_id'],byid)
                        op=byid[allocation['operation_id']];self.assertEqual(p,op['permit'])
                    retained_producers={p['producer_id']:p for p in fence['service_producers']+fence['object_workers']}
                    for closure in w['original_closures']:
                        allocation=closure['allocation'];operation_id=allocation['operation_id']
                        self.assertEqual((allocation['actor']['owner_id'],allocation['selected_object_id'],allocation['selected_generation']),containing)
                        self.assertIn(operation_id,byid);runtime=byid[operation_id]
                        self.assertEqual(allocation,runtime['allocation'])
                        self.assertEqual(closure['submitted_binding'],runtime['binding'])
                        self.assertEqual(closure['closure'],runtime['closure'])
                        request_id=closure['original_request_id']
                        candidates=[]
                        for exchange in after['exchanges']:
                            q=bytes(exchange['request_wire'])
                            if (exchange['selected_object_id']==w['selected_object_id'] and exchange['actor']==allocation['actor']
                                and exchange['endpoint_class']=='artifact' and exchange['operation_id']==operation_id
                                and struct.unpack_from('<I',q,4)[0]==5 and struct.unpack_from('<Q',q,20)[0]==request_id
                                and struct.unpack_from('<Q',q,52)[0]==operation_id):
                                self.assertEqual(struct.unpack_from('<QQQ',q,28),(allocation['actor']['session_id'],allocation['actor']['session_generation'],allocation['expected_revision']))
                                if closure['submitted_binding'] is not None:
                                    source=closure['submitted_binding']['source']
                                    self.assertEqual(struct.unpack_from('<Q',q,60)[0],source['source_object_id'])
                                    self.assertEqual(struct.unpack_from('<I',q,68)[0],source['byte_len']+64)
                                candidates.append(exchange)
                        self.assertTrue(candidates,'closure has no actual original object/request/operation exchange')
                        retained=retained_producers.get(closure['original_producer_id'])
                        if retained is not None:
                            self.assertEqual(retained['producer_generation'],closure['original_producer_generation'])
                            self.assertEqual((retained['selected_object_id'],retained['request_id'],retained['operation_id']),(w['selected_object_id'],request_id,operation_id))
                        # Already joined historical producers need not remain in the fence.

    def test_actual_authorized_text_receipt_bytes(self):
        text_count=receipt_count=0
        for f in self.data.fixtures():
            epochs={epoch:after for epoch,_,_,_,_,after in f.epochs()}
            for r,raw in lease_records(f):
                if r['kind']=='selected_text':
                    obj,revision,body,digest=text_bytes(raw);text_count+=1
                    self.assertIn(obj,(31,32));self.assertEqual(hashlib.sha256(body).digest(),digest)
                else:
                    receipt_count+=1;rr=receipt_bytes(raw)
                    owner=next(o for o in epochs[r['producer_epoch']]['objects'] if o['selected']['selected_object_id']==rr['selected_object_id'])
                    op=next((o for o in owner['operations'] if o['allocation']['operation_id']==rr['operation_id']),None)
                    self.assertIsNotNone(op);self.assertEqual(op['receipt'],rr)
        self.assertGreaterEqual(text_count,6);self.assertGreater(receipt_count,0)

    def test_lease_call_epoch_identity(self):
        for f in self.data.fixtures():
            records=list(lease_records(f))
            self.assertEqual({r['relative_file'] for r,_ in records},{p[len(f.prefix):] for p in self.data.store.inventory if p.startswith(f.prefix) and re.search(r'/lease-\d+[.]bin$',p)})
            self.assertLessEqual(len(records),256)
            for r,_ in records:
                call=f.calls['records'][r['call_ordinal']]
                self.assertEqual(r['actor'],call['actor'])
                self.assertEqual(r['producer_epoch'],call['producer_epoch'])
                self.assertEqual(r['endpoint_class'],call['endpoint_class'])
            if (f.case,f.leg) in REOPEN:self.assertTrue(any(r['producer_epoch']==1 for r,_ in records),'actual new-owner authorized observation missing')

    def test_seven_malformed_input_actual_errors(self):
        names=['raw_decode_reserved_handle_byte14','raw_decode_outer_pad_byte84','typed_dispatch_payload_len_minus1','typed_dispatch_payload_tail_byte63','typed_dispatch_zero_handle_generation','typed_dispatch_protocol369','typed_dispatch_reply_direction_msg2']
        statuses=[2,2,2,2,2,3,3]
        for f in self.data.fixtures():
            records=rows(f.negative['negative_calls'],7)
            if (f.case,f.leg)!=(CASE[3],0):self.assertEqual(records,[]);continue
            self.assertEqual(len(records),7)
            states=state_witnesses(f);digest=states['selectedtext_before_allocation']['state_sha256']
            typed_seen=[]
            for ordinal,(r,name,status) in enumerate(zip(records,names,statuses)):
                keys(r,{'ordinal','producer_epoch','stage','input_kind','raw_wire_or_null','typed_envelope_or_null','input_diagnostic_sha256','actual_returned_error_status','actor_or_null','endpoint_class_or_null','selected_object_id_or_null','before_state_sha256','after_state_sha256'})
                self.assertEqual(uint(r['ordinal'],6),ordinal);self.assertEqual(uint(r['producer_epoch'],1),0)
                self.assertEqual(uint(r['actual_returned_error_status'],11),status)
                self.assertEqual(r['before_state_sha256'],digest);self.assertEqual(r['after_state_sha256'],digest)
                self.assertLessEqual(len(compact(r)),4096)
                if ordinal<2:
                    self.assertEqual(r['stage'],'raw_decoder');self.assertEqual(r['input_kind'],'raw_wire')
                    self.assertIsNone(r['typed_envelope_or_null']);raw=bytes(uint(v,255) for v in rows(r['raw_wire_or_null'],88));self.assertEqual(len(raw),88)
                    self.assertEqual(sha(raw),hexhash(r['input_diagnostic_sha256']))
                    self.assertNotEqual(raw[14] if ordinal==0 else raw[84],0)
                    self.assertIsNone(r['actor_or_null']);self.assertIsNone(r['endpoint_class_or_null']);self.assertIsNone(r['selected_object_id_or_null'])
                    with self.assertRaises(EvidenceDenied):wire(list(raw))
                else:
                    self.assertEqual(r['stage'],'typed_dispatch');self.assertEqual(r['input_kind'],'typed_envelope')
                    self.assertIsNone(r['raw_wire_or_null']);t=r['typed_envelope_or_null'];shape(t,'typed_envelope',self.data.defs)
                    self.assertEqual(sha(compact(field_order(t,'typed_envelope',self.data.defs))),hexhash(r['input_diagnostic_sha256']))
                    actor(r['actor_or_null']);self.assertEqual(r['endpoint_class_or_null'],'artifact');self.assertEqual(r['selected_object_id_or_null'],31)
                    if ordinal==2:self.assertEqual(t['payload_len'],39)
                    elif ordinal==3:self.assertNotEqual(t['payload'][63],0)
                    elif ordinal==4:self.assertEqual(t['handle']['generation'],0)
                    elif ordinal==5:self.assertEqual(t['protocol'],369)
                    else:self.assertEqual(t['msg_type'],2)
                    compatible=[c for c in f.calls['records'] if c['input_kind']=='native_typed_envelope' and c['typed_envelope_or_null']==t and c['actor']==r['actor_or_null'] and c['endpoint_class']==r['endpoint_class_or_null'] and c['selected_object_id']==r['selected_object_id_or_null']]
                    self.assertEqual(len(compatible),1);self.assertEqual(compatible[0]['actual_result'],{'kind':'error','error_status':status});self.assertIsNone(compatible[0]['matched_public_exchange_index_or_null']);typed_seen.append(compatible[0]['call_ordinal'])
            self.assertEqual(len(set(typed_seen)),5)

    def test_canonical_descriptor_24_denials_three_states(self):
        for f in self.data.fixtures():
            denials=rows(f.negative['lease_denials'],24)
            if (f.case,f.leg)!=(CASE[3],0):self.assertEqual(denials,[]);continue
            self.assertEqual(len(denials),24);states=state_witnesses(f);invocations=set()
            for ordinal,r in enumerate(denials):
                keys(r,{'ordinal','producer_epoch','recorded_lease_call_ordinal','method','actor','endpoint_class','selected_object_id','canonical_descriptor','attempted_descriptor','actual_returned_error_status','before_state_sha256','after_state_sha256','state_phase'})
                self.assertEqual(uint(r['ordinal'],23),ordinal);self.assertEqual(uint(r['producer_epoch'],1),0);self.assertEqual(uint(r['actual_returned_error_status'],11),1)
                invocation=uint(r['recorded_lease_call_ordinal'],U32);self.assertNotIn(invocation,invocations);invocations.add(invocation)
                kind=['SelectedText','DraftSource','Receipt'][ordinal//8];variant=(ordinal%8)//2
                self.assertEqual(r['method'],['read_lease','write_lease'][ordinal%2]);actor(r['actor'])
                self.assertEqual(r['endpoint_class'],'artifact');self.assertEqual(r['selected_object_id'],31)
                original=r['canonical_descriptor'];changed=r['attempted_descriptor'];native_descriptor(original,True);native_descriptor(changed)
                self.assertEqual(original['kind'],kind);expected=strict_json(compact(original))
                if variant==0:expected['handle']['index']+=65536
                elif variant in (1,2):
                    expected['handle']['generation']+=4294967296
                    if variant==2:expected['object_generation']=expected['handle']['generation']
                else:expected['object_generation']+=1
                self.assertEqual(changed,expected)
                if variant<3:self.assertEqual(packed_native(changed),packed_native(original),'real truncating alias')
                with self.assertRaises(EvidenceDenied):native_descriptor(changed,True)
                phase=['selectedtext_before_allocation','draft_after_invalid_submissions','receipt_after_commit'][ordinal//8]
                self.assertEqual(r['state_phase'],phase);self.assertEqual(r['before_state_sha256'],states[phase]['state_sha256']);self.assertEqual(r['after_state_sha256'],r['before_state_sha256'])
                self.assertLessEqual(len(compact(r)),2048)
            copied=list(lease_records(f))
            text=[raw for r,raw in copied if r['producer_epoch']==0 and r['kind']=='selected_text' and text_bytes(raw)[:2]==(31,1)]
            receipts=[raw for r,raw in copied if r['producer_epoch']==0 and r['kind']=='receipt']
            self.assertGreaterEqual(len(text),2);self.assertEqual(text[0],text[-1]);self.assertEqual(text_bytes(text[0])[2],OLD)
            self.assertGreaterEqual(len(receipts),2);self.assertEqual(receipts[0],receipts[-1]);rr=receipt_bytes(receipts[0]);self.assertEqual(rr['source_content_hash'],list(hashlib.sha256(NEW).digest()));self.assertEqual(rr['result_revision'],2)
            before=states['selectedtext_before_allocation']['state']['objects'][0];after=states['receipt_after_commit']['state']['objects'][0]
            self.assertEqual((before['permits'],before['transitions']),(0,0));self.assertEqual((after['permits'],after['transitions']),(1,1))

    def test_original_reconciled_receipt_no_replay(self):
        for leg in range(4):
            f=self.data.fixture(CASE[5],leg);o=observations(f);records=matching(f)
            epoch0=next(after for epoch,_,_,_,_,after in f.epochs() if epoch==0)
            originals=[op for op in epoch0['objects'][0]['operations'] if op['receipt'] is not None]
            if leg!=3:self.assertTrue(originals)
            if leg in (1,2):
                rr=o['reconciled_receipt'];self.assertEqual(rr['operation_id'],o['known_operation'])
                self.assertEqual(rr['outcome'],'committed' if leg==2 else 'definitive_noncommit')
                commits=[e for e in epoch0['exchanges'] if struct.unpack_from('<I',bytes(e['request_wire']),4)[0]==5 and struct.unpack_from('<Q',bytes(e['request_wire']),52)[0]==rr['operation_id']]
                self.assertEqual(len(commits),1);self.assertTrue(commits[0]['reply_suppressed'])
                call=next(c for c in records if c['matched_public_exchange_index_or_null']==epoch0['exchanges'].index(commits[0]) and c['producer_epoch']==0)
                self.assertEqual(reply_status(bytes(call['actual_result']['returned_wire'])),9)
                self.assertTrue(any(receipt_bytes(raw)==rr for r,raw in lease_records(f) if r['kind']=='receipt'))
            elif leg==0:
                rr=o['timeout_noncommit'];self.assertEqual(rr['outcome'],'definitive_noncommit')
                self.assertNotEqual(rr['operation_id'],o['fresh_receipt']['operation_id']);self.assertEqual(o['fresh_receipt']['result_revision'],2)
                self.assertEqual(o['original_io_before_release'],o['original_io_after_release']);self.assertTrue(o['original_io_before_release'])
                actual=[e for e in epoch0['io_events'] if e['selected_object_id']==31 and e['operation_id']==rr['operation_id']]
                adapted=[{'object':e['selected_object_id'],'operation':e['operation_id'],'writer':e['writer_id'],'sequence':e['sequence'],'point':e['point'],'outcome':e['outcome'],'artifact_hash':e['artifact_hash']} for e in actual]
                self.assertEqual(adapted,o['original_io_after_release'])
            else:
                rr=o['reconciled_receipt'];self.assertEqual(rr['outcome'],'committed');self.assertEqual(rr['result_revision'],2)
                self.assertEqual(o['original_permit']['binding'],o['original_binding'])
                old=[op for op in epoch0['objects'][0]['operations'] if op['allocation']['operation_id']==rr['operation_id']]
                self.assertEqual(len(old),1);old=old[0]
                self.assertEqual(old['state'],'permitted');self.assertIsNone(old['receipt']);self.assertIsNone(old['successor'])
                self.assertEqual(old['binding'],o['original_binding']);self.assertEqual(old['permit'],o['original_permit'])
                epoch1=next((before,after) for epoch,_,_,_,before,after in f.epochs() if epoch==1)
                recovered=[op for op in epoch1[1]['objects'][0]['operations'] if op['allocation']['operation_id']==rr['operation_id']]
                self.assertEqual(len(recovered),1);recovered=recovered[0]
                self.assertEqual(recovered['state'],'committed');self.assertEqual(recovered['receipt'],rr)
                for key in ('allocation','allocated_service_epoch','binding','submitted_service_epoch','permit','closure'):
                    self.assertEqual(recovered[key],old[key])
                commits=[e for e in epoch0['exchanges'] if struct.unpack_from('<I',bytes(e['request_wire']),4)[0]==5 and e['operation_id']==rr['operation_id']]
                self.assertEqual(len(commits),1)
                for state in epoch1:
                    self.assertFalse(any(struct.unpack_from('<I',bytes(e['request_wire']),4)[0]==5 and e['operation_id']==rr['operation_id'] for e in state['exchanges']))
                    self.assertTrue(any(struct.unpack_from('<I',bytes(e['request_wire']),4)[0]==7 and e['endpoint_class']=='recovery_receipt' and e['operation_id']==rr['operation_id'] for e in state['exchanges']))
                    for key in ('mutation_dispatch_count','permit_count','transition_count'):
                        self.assertEqual(state['objects'][0][key],epoch0['objects'][0][key])
                events=[e for e in epoch0['io_events'] if e['operation_id']==rr['operation_id']]
                failures=[e for e in events if e['point']=='journal_directory_sync' and e['outcome']=='simulated_failure']
                self.assertEqual(len(failures),1);self.assertEqual(failures[0]['sequence'],o['simulated_failure_sequence'])
                self.assertTrue(any(e['point']=='journal_rename' and e['outcome']=='completed' and e['sequence']<failures[0]['sequence'] for e in events))
                self.assertTrue(any(r['producer_epoch']==1 and r['kind']=='receipt' and receipt_bytes(raw)==rr for r,raw in lease_records(f)))
        race=self.data.fixture(CASE[4],0);o=observations(race)
        self.assertEqual(o['original_receipt'],o['duplicate_receipt']);self.assertEqual(o['original_receipt'],o['recovery_receipt'])
        self.assertNotEqual(o['original_receipt']['operation_id'],o['fresh_receipt']['operation_id']);self.assertEqual(o['fresh_receipt']['result_revision'],3)
        final=list(race.epochs())[-1][-1]['objects'][0];self.assertEqual((final['permit_count'],final['transition_count']),(2,2))

    def test_reopen_phase_states_and_failures(self):
        attempts=successes=failures=0
        for f in self.data.fixtures():
            states=state_witnesses(f);a=f.negative['reopen_attempt']
            if (f.case,f.leg) not in REOPEN|FAILED_REOPEN:self.assertIsNone(a);continue
            attempts+=1;shape(a,'reopen_attempt',self.data.defs)
            old=list(f.epochs())[0];_,row,fence,raw,_,_=old
            self.assertEqual(a['before_fence_ref'],row['fence_ref']);self.assertEqual(a['before_files_state_sha256'],states['before_reopen']['state_sha256']);self.assertEqual(a['after_files_state_sha256'],states['after_reopen']['state_sha256'])
            for s in states.values():self.assertEqual(s['state']['namespace_id'],fence['namespace_id'])
            if (f.case,f.leg) in FAILED_REOPEN:
                failures+=1;self.assertFalse(a['success']);self.assertEqual(states['before_reopen']['state'],states['after_reopen']['state'])
                self.assertEqual(a['before_files_state_sha256'],a['after_files_state_sha256']);self.assertEqual(a['returned_fence_byte_len_or_null'],len(raw));self.assertEqual(a['returned_fence_sha256_or_null'],sha(raw));self.assertIsNone(a['actual_new_owner_witness_or_null'])
                self.assertIn(a['actual_error_status_or_null'],(5,) if (f.case,f.leg)==(CASE[2],5) else (6,10))
            else:
                successes+=1;self.assertTrue(a['success']);self.assertIsNone(a['actual_error_status_or_null']);self.assertIsNone(a['returned_fence_byte_len_or_null']);self.assertIsNone(a['returned_fence_sha256_or_null'])
                witnesses=a['actual_new_owner_witness_or_null'];self.assertEqual([w['selected_object_id'] for w in witnesses],[31,32])
                new=list(f.epochs())[1];self.assertEqual(new[2]['namespace_id'],fence['namespace_id'])
                for w,prior,obj in zip(witnesses,fence['objects'],new[-1]['objects']):
                    self.assertEqual(w,obj['reopen_witness']);self.assertEqual(w['fence_snapshot_hash'],list(hashlib.sha256(raw).digest()));self.assertEqual(w['fence_id'],fence['fence_id']);self.assertEqual(w['prior_service_epoch'],prior['service_epoch_at_fence']);self.assertEqual(w['actual_service_epoch'],prior['reserved_successor_epoch']);self.assertGreater(w['actual_service_epoch'],w['prior_service_epoch'])
                if f.case==CASE[6]:
                    phase=observations(f)['phase_index'];self.assertEqual(phase,f.leg)
                    receipt=observations(f)['recovered_receipt'];self.assertEqual(receipt['outcome'],'definitive_noncommit' if phase<2 else 'committed')
                    self.assertEqual(observations(f)['selection_revision'],1 if phase<2 else 2)
        self.assertEqual((attempts,successes,failures),(26,8,18))

    def test_explicit_owned_root_cleanup(self):
        fs=self.data.provenance['fixture_filesystems'];self.assertEqual(len({r['fixture_id'] for r in fs}),48)
        for f in self.data.fixtures():
            cleanup_files(f);c=f.cleanup;self.assertEqual(c['live_store_ownership'],0);self.assertTrue(c['fixture_owned_holders_consumed']);self.assertEqual(c['tempdir_close'],{'result':'ok'})
            a=c['root_absence'];self.assertEqual(a['probe'],'symlink_metadata');self.assertTrue(a['after_tempdir_close']);self.assertEqual(a['result'],'not_found');self.assertEqual(a['io_error_kind_or_null'],'NotFound');self.assertEqual(a['raw_os_error_or_null'],2);self.assertIsNone(a['observed_device_or_null']);self.assertIsNone(a['observed_inode_or_null'])
            epochs=list(f.epochs());self.assertEqual(c['epoch_terminal_fence_refs'],[r[1]['fence_ref'] for r in epochs])
            pending={(r['producer_epoch'],r['call_ordinal']):r for r in f.calls['records'] if r['caller_kind']=='pending_thread'}
            joined={}
            for row in c['external_caller_joins']:
                self.assertLessEqual(len(compact(row)),512);key=(row['producer_epoch'],row['call_ordinal']);self.assertNotIn(key,joined);self.assertIn(key,pending);joined[key]=row
                self.assertTrue(row['completed_join']);self.assertEqual(row['join_outcome'],'Returned');self.assertEqual(row['native_test_pid'],self.data.pid);self.assertEqual(row['rust_thread_id'],pending[key]['caller_rust_thread_id'])
            self.assertEqual(set(joined),set(pending));self.assertLessEqual(sum(len(compact(r)) for r in c['external_caller_joins']),262144)
            carried=c['fixture_filesystem'];shape(carried,'fixture_filesystem',self.data.defs);self.assertLessEqual(len(compact(carried)),16384)
            self.assertEqual(carried,next(r for r in fs if r['fixture_id']==c['fixture_id']));self.assertTrue(carried['observed_while_root_live'])
            self.assertEqual((carried['root_path'],carried['root_device'],carried['root_inode'],carried['native_test_pid']),(c['root_path'],c['root_device'],c['root_inode'],self.data.pid))
            method=carried['observation'];system=self.data.provenance['os_profile']['system']
            self.assertEqual(method['method'],'linux_mountinfo' if system=='Linux' else 'macos_statfs');self.assertFalse(method['mount_flags']['read_only'])
            self.assertTrue(Path(c['root_path']).is_absolute());self.assertTrue(Path(c['root_path']).is_relative_to(method['mount_point']))
            if system=='Linux':self.assertEqual((method['device_major'],method['device_minor']),(os.major(c['root_device']),os.minor(c['root_device'])))

    def test_six_healthy_and_epochmax_controls(self):
        healthy_ids=[]
        for index in range(6):
            f=self.data.fixture(CASE[2],10+index);o=observations(f);self.assertEqual(o['positive_for_counter_index'],index);self.assertEqual(o['healthy_actual_read_revision'],1);self.assertEqual(o['healthy_actual_read_content_hash'],list(hashlib.sha256(OLD).digest()));healthy_ids.append(f.summary['fixture_id'])
            self.assertTrue(any(r['kind']=='selected_text' and text_bytes(raw)[:3]==(31,1,OLD) for r,raw in lease_records(f)))
            self.assertNotEqual(f.summary['fixture_id'],self.data.fixture(CASE[2],3+index).summary['fixture_id'])
        self.assertEqual(len(set(healthy_ids)),6)
        f=self.data.fixture(CASE[2],5);o=observations(f);epoch=list(f.epochs())[0];obj=epoch[2]['objects'][0]
        self.assertEqual(obj['service_epoch_at_fence'],U64);self.assertIsNone(obj['reserved_successor_epoch']);self.assertEqual(o['epoch_max_fence'],epoch[1]['fence_ref']);self.assertEqual(o['epoch_max_reopen_status'],5)
        self.assertEqual(f.negative['reopen_attempt']['actual_error_status_or_null'],5);self.assertEqual(len(f.summary['epoch_records']),1)
        capacity=self.data.fixture(CASE[2],0);o=observations(capacity);old=list(capacity.epochs())[0][-1]['objects'][0];new=list(capacity.epochs())[1][-1]['objects'][0]
        ids=o['retained_ids'];self.assertEqual(sorted(r['allocation']['operation_id'] for r in old['operations']),ids);self.assertEqual(sorted(r['allocation']['operation_id'] for r in new['operations']),ids);self.assertEqual(len(ids),16)
        self.assertTrue(any(reply_status(bytes(e['actual_reply_wire']))==5 for e in list(capacity.epochs())[1][-1]['exchanges']))

    def test_volatile_UI_run_font_frame_provenance(self):
        ids=[];frames=0;font=source_font()
        for case,index in UI_INVENTORY:
            _,r,notes=get_ui(self.data,case,index);ids.append(notes['b_run_binding']['ui_fixture_id'])
            for f in r['frames']:
                frames+=1;pixels=self.data.ui.read(case+'/'+f['file'],640*568*4)
                expected=ascii_crop(b'other\na' if case==CASE[1] else OLD,font)
                self.assertEqual(pixels[48*640*4:528*640*4],expected,'independent fixed corpus raster and cursor')
                self.assertTrue(chrome_label(pixels,font,b'VOLATILE / IN-PROCESS'),'actual compositor-owned volatile label')
            if case==CASE[0]:
                entered=notes['volatile_before_effect_entered'];settled=notes['volatile_before_effect_settled']
                self.assertTrue(entered['entered']);self.assertFalse(entered['settled']);self.assertFalse(entered['permit_issued']);self.assertTrue(settled['settled']);self.assertFalse(settled['permit_issued'])
                self.assertEqual((entered['request_id'],entered['operation_id']),(settled['request_id'],settled['operation_id']))
                self.assertEqual(entered['operation_id'],0)
        self.assertEqual((len(ids),len(set(ids)),frames),(5,5,18))
        self.assertFalse(set(ids)&{f.summary['fixture_id'] for f in self.data.fixtures()},'UI fixture identities do not confer Store authority')

    def test_queue_reset_frame_terminal(self):
        _,q,notes=get_ui(self.data,CASE[2],0);b=notes['backend_boundary']
        self.assertEqual(b['fresh_successor_sequence'],b['overflow_sequence']+1);self.assertLess(b['overflow_sequence'],U64)
        exchanges=[(bytes.fromhex(e['request_hex']),bytes.fromhex(e['reply_hex'])) for e in q['exchanges']]
        produce=[(a,r) for a,r in exchanges if struct.unpack_from('<II',a)==(802,3)]
        overflow=[(a,r) for a,r in produce if struct.unpack_from('<Q',a,44)[0]==b['overflow_sequence']]
        self.assertEqual(len(overflow),1);self.assertEqual(struct.unpack_from('<I',overflow[0][1],28)[0],0)
        self.assertTrue(any(struct.unpack_from('<I',r,28)[0]==6 for _,r in produce),'actual producer NotReady before Reset consumption')
        polls=[(a,r) for a,r in exchanges if struct.unpack_from('<II',a)==(832,3)]
        resets=[r for _,r in polls if struct.unpack_from('<Q',r,28)[0]==b['overflow_sequence'] and struct.unpack_from('<I',r,48)[0]==3]
        self.assertEqual(len(resets),1);self.assertEqual(struct.unpack_from('<III',resets[0],44),(0,3,0));self.assertEqual(struct.unpack_from('<I',resets[0],56)[0],0)
        self.assertTrue(any(struct.unpack_from('<Q',r,28)[0]==b['fresh_successor_sequence'] and struct.unpack_from('<II',r,44)==(4,1) and struct.unpack_from('<I',r,56)[0]==0 for _,r in polls),'actual successor delivery')
        raw,frame,notes=get_ui(self.data,CASE[2],1);self.assertEqual(len(frame['frames']),15)
        terminal_path=CASE[2]+'/frame-terminal-1.json';terminal=self.data.ui.json(terminal_path,4096)
        keys(terminal,{'schema_version','case','registry_index','prior_record_sha256','compose_terminal_statuses','post_exhaustion_evidence_status','intentional_expected_failure','integrated_store_save'})
        self.assertEqual(type(terminal['schema_version']),int);self.assertEqual(terminal['schema_version'],1);self.assertEqual(terminal['case'],CASE[2]);self.assertEqual(type(terminal['registry_index']),int);self.assertEqual(terminal['registry_index'],1)
        self.assertEqual(terminal['prior_record_sha256'],sha(raw));self.assertEqual(terminal['compose_terminal_statuses'],[5,5]);self.assertTrue(all(type(v) is int for v in terminal['compose_terminal_statuses']));self.assertEqual(type(terminal['post_exhaustion_evidence_status']),int);self.assertEqual(terminal['post_exhaustion_evidence_status'],5);self.assertIs(terminal['intentional_expected_failure'],True);self.assertIs(terminal['integrated_store_save'],False)
        self.assertEqual({p for p in self.data.ui.inventory if 'terminal' in p},{terminal_path})
        self.assertEqual(notes['backend_boundary']['actual_composed_limit'],notes['backend_boundary']['initial_consumed_frame']+len(frame['frames']))

    def test_file_row_aggregate_bounds(self):
        # These negative controls must reject exact malformed data, never arbitrary exceptions.
        for malformed in (b'',b'{',b'{"x":1,"x":2}',b'{"x":1.0}',b'{"x":NaN}'):
            with self.assertRaises(EvidenceDenied):strict_json(malformed)
        with self.assertRaises(EvidenceDenied):uint(True)
        with self.assertRaises(EvidenceDenied):keys({'unexpected':1},set())
        totals=collections.Counter();expected_store=set();expected_ui=set()
        for f in self.data.fixtures():
            files=rows(f.summary['actual_files'],160);paths=[];size=0
            for i,r in enumerate(files):
                shape(r,'copied_file_row',self.data.defs);self.assertLessEqual(len(compact(r)),512)
                self.assertEqual(r['evidence_file'],f'actual-file-{i}.bin');paths.append(r['relative_path']);self.assertFalse(Path(r['relative_path']).is_absolute());self.assertTrue(all(p not in ('','.', '..') for p in r['relative_path'].split('/')))
                raw=f.raw(r['evidence_file'],131072);self.assertEqual(len(raw),r['bytes']);self.assertEqual(hashlib.sha256(raw).digest(),hash32(r['sha256']));size+=len(raw)
                if r['kind']=='symlink':self.assertIn((f.case,f.leg),FAILED_REOPEN);self.assertTrue(raw)
                # Healthy selection journals remain exact typed, canonical and chain-valid.
                if r['relative_path'].endswith('/selection.json') and (f.case,f.leg) not in FAILED_REOPEN:
                    journal_bytes(raw)
            self.assertEqual(paths,sorted(set(paths),key=lambda p:p.encode('utf8')));self.assertLessEqual(size,4194304)
            cleanup_files(f);expected_store|={f.prefix+r['relative_file'] for r in f.cleanup['emitted_files']}|{f.prefix+'cleanup.json'}
            for path in (p for p in self.data.store.inventory if p.startswith(f.prefix)):
                name=path[len(f.prefix):];n=self.data.store.inventory[path][2]
                if name.startswith('actual-file-'):component,cap='actual_store_files',131072
                elif name.startswith('lease-'):component,cap='actual_lease_buffers',4160
                elif name.startswith('epoch-') and name.endswith('-fence.json'):component,cap='epoch_full_fence_json',2097152
                elif name.startswith('epoch-'):component,cap='epoch_snapshot_json',2097152
                else:
                    component,cap={'actual.json':('store_summary_json',2097152),'calls.json':('calls_manifests',2097152),'leases.json':('lease_manifests',262144),'negative-calls.json':('negative_and_reopen_manifests',524288 if (f.case,f.leg)==(CASE[3],0) else 262144),'cleanup.json':('cleanup_manifests',1048576)}[name]
                self.assertLessEqual(n,cap);totals[component]+=n
        self.assertEqual(set(self.data.store.inventory),expected_store)
        for case,index in UI_INVENTORY:
            _,r,_=get_ui(self.data,case,index);expected_ui.add(case+f'/registry-{index}.json')
            for frame in r['frames']:expected_ui.add(case+'/'+frame['file'])
        expected_ui.add(CASE[2]+'/frame-terminal-1.json');self.assertEqual(set(self.data.ui.inventory),expected_ui)
        for path,(_,_,n) in self.data.ui.inventory.items():totals['ui_a_full_frames' if path.endswith('.bgra') else 'ui_a_frame_terminal' if 'terminal' in path else 'ui_a_registry_json']+=n
        caps=dict(self.data.v4['aggregate_bounds']['component_byte_ceilings']);caps['calls_manifests']=100663296;caps['negative_and_reopen_manifests']+=262144
        for k,n in totals.items():self.assertLessEqual(n,caps[k],k)
        self.assertLessEqual(sum(t.total_bytes for t in self.data.opened),1073741824);self.assertLessEqual(sum(len(t.inventory) for t in self.data.opened),20436);self.assertLessEqual(sum(t.directories for t in self.data.opened),96)
        # Runner artifacts cannot hide arbitrary large extras beyond their two declared components.
        log_names={self.data.provenance[k]['relative_file'] for k in ('test_log','test_list_log')}
        log_bytes=sum(self.data.runner.inventory[n][2] for n in log_names)
        other_bytes=sum(n for p,(_,_,n) in self.data.runner.inventory.items() if p not in log_names)
        self.assertLessEqual(log_bytes,16777216);self.assertLessEqual(other_bytes,4194304);self.assertLessEqual(len(self.data.runner.inventory),36)

    def test_source_nonce_owned_process_binding(self):
        p=self.data.provenance;process=p['test_process'];sources=p['sources']
        self.assertEqual(p['evidence_contract_sha256'],CONTRACT_PINS['editor-store-recording-composite-frozen.json'])
        self.assertEqual([r['relative_path'] for r in sources],sorted(self.data.sources));self.assertLessEqual(len(sources),1024)
        self.assertTrue(all(len(compact(r))<=512 for r in sources));self.assertLessEqual(sum(len(compact(r)) for r in sources),524288)
        self.assertEqual(sources,self.data.trusted_manifest)
        typed=[field_order(r,'source_record',self.data.defs) for r in self.data.trusted_manifest]
        self.assertEqual(sha(compact(typed)),ARGS.source_manifest_sha256)
        self.assertEqual(p['accepted_source_manifest_sha256'],ARGS.source_manifest_sha256)
        mandatory={'Cargo.toml','Cargo.lock','rust-toolchain.toml','services/store_service/tests/editor_store.rs','services/store_service/tests/support/editor_store.rs','services/store_service/src/editor_store/mod.rs','services/store_service/src/editor_store/types.rs','services/store_service/src/editor_store/codec.rs','services/store_service/src/editor_store/storage.rs','services/store_service/examples/check_editor_store_evidence.rs','tools/ci/editor_store_result.py','tools/ci/foundry_desktop_editor_store_ui1_1b.sh','docs/DESKTOP_EDITOR_STORE_API_V0.md','docs/plans/editor-store-transaction-v0.md','artifact_store_schema/src/editor_save.rs','artifact_store_schema/tests/editor_save.rs','services/desktop/tests/support/editor_host_assertions.rs','services/desktop/tests/editor_host.rs','services/desktop/tests/fixtures/ascii8x16_v0.bin','services/desktop/tests/fixtures/ascii8x16_v0.golden.json'}
        self.assertTrue(mandatory<=set(self.data.sources),'independently frozen actual dependency closure missing mandatory inputs')
        unchanged={r['path']:r['sha256'] for r in self.data.composite['baseline_sources'] if r['path'] in ('docs/DESKTOP_EDITOR_STORE_API_V0.md','docs/plans/editor-store-transaction-v0.md','artifact_store_schema/src/editor_save.rs','artifact_store_schema/tests/editor_save.rs','services/desktop/tests/support/editor_host_assertions.rs','services/desktop/tests/editor_host.rs','services/desktop/tests/fixtures/ascii8x16_v0.bin','services/desktop/tests/fixtures/ascii8x16_v0.golden.json')}
        for path,pin in unchanged.items():self.assertEqual(self.data.sources[path]['sha256'],pin)
        self.assertTrue(process['spawn_observed']);self.assertTrue(process['wait_reaped']);self.assertEqual(type(process['exit_code_or_null']),int);self.assertEqual(process['exit_code_or_null'],0);self.assertIsNone(process['term_signal_or_null']);self.assertGreaterEqual(process['exit_monotonic_ns'],process['start_monotonic_ns'])
        self.assertIn('--test-threads=1',process['argv']);self.assertFalse(any(x in process['argv'] for x in ('--ignored','--include-ignored','--list')))
        self.assertEqual(p['test_inventory'],CASE)
        system=p['os_profile']['system'];self.assertIn(system,('Linux','Darwin'));self.assertEqual(process['start_identity_method'],'linux_proc_starttime_boot_id' if system=='Linux' else 'macos_proc_starttime')
        for key in ('rustc_profile','cargo_profile'):
            profile=p[key];self.assertEqual(profile['exit_code'],0);self.assertEqual(sha(profile['stdout'].encode('utf8')),profile['stdout_sha256'])
        self.assertIn(p['rustc_profile']['release'],p['rustc_profile']['stdout']);self.assertIn(p['rustc_profile']['host'],p['rustc_profile']['stdout']);self.assertTrue(p['cargo_profile']['stdout'].startswith('cargo '))
        self.assertLessEqual(sum(len(compact(r)) for r in p['fixture_filesystems']),786432)
        for k,flag in p['claims'].items():self.assertIs(flag,k in ('host_cas','directory_sync_observed'))
        completed_dirsync=False
        for f in self.data.fixtures():
            for _,_,_,_,_,after in f.epochs():completed_dirsync|=any(e['point'] in ('candidate_directory_sync','journal_directory_sync') and e['outcome']=='completed' for e in after['io_events'])
        self.assertTrue(completed_dirsync,'actual named filesystem directory sync completion missing')
        logs={}
        for key in ('test_log','test_list_log'):
            ref=p[key];raw=self.data.runner.read(ref['relative_file'],16777216);self.assertEqual(len(raw),ref['byte_len']);self.assertEqual(sha(raw),ref['sha256']);logs[key]=raw.decode('utf8','strict')
        listed=re.findall(r'^(\w+): test$',logs['test_list_log'],re.MULTILINE);self.assertEqual(sorted(listed),sorted(CASE));self.assertIn('7 tests, 0 benchmarks',logs['test_list_log'])
        tested=re.findall(r'^test (\w+) \.\.\. (\w+)$',logs['test_log'],re.MULTILINE);self.assertEqual(sorted(tested),sorted((c,'ok') for c in CASE));self.assertRegex(logs['test_log'],r'test result: ok[.] 7 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out;')


# Independent fixture raster; only this fixed source path is read, never an artifact path.
def source_font():
    path=HERE.parent.parent/'services/desktop/tests/fixtures/ascii8x16_v0.bin'
    fd=os.open(path,os.O_RDONLY|os.O_NOFOLLOW)
    try:
        info=os.fstat(fd);require(stat.S_ISREG(info.st_mode) and info.st_size==1536,'pinned font size')
        raw=os.read(fd,1537);require(len(raw)==1536 and sha(raw)==FONT,'pinned font bytes');return raw
    finally:os.close(fd)


def ascii_crop(text,font):
    pixels=bytearray(b'\xff'*(640*480*4));row=column=0
    for c in text:
        if c==10:row+=1;column=0;continue
        require(32<=c<=126,'fixed raster corpus')
        for y in range(16):
            bits=font[(c-32)*16+y]
            for x in range(8):
                if bits & (128>>x):
                    offset=((row*16+y)*640+column*8+x)*4;pixels[offset:offset+4]=b'\x00\x00\x00\xff'
        column+=1
    for y in range(row*16,row*16+16):
        for x in range(column*8,column*8+2):
            offset=(y*640+x)*4;pixels[offset:offset+4]=b'\x00\x00\x00\xff'
    return bytes(pixels)


def chrome_label(pixels,font,label):
    for y in (0,8,16,24,32,528,536,544,552):
        for x in range(0,641-len(label)*8,8):
            good=True
            for i,c in enumerate(label):
                for dy in range(16):
                    for dx in range(8):
                        offset=((y+dy)*640+x+i*8+dx)*4
                        want=b'\x00\x00\x00\xff' if font[(c-32)*16+dy] & (128>>dx) else b'\xe0\xe0\xe0\xff'
                        if pixels[offset:offset+4]!=want:good=False;break
                    if not good:break
                if not good:break
            if good:return True
    return False


# Independent declaration-order map from the frozen pure Rust structs.
PURE_FIELDS = {
    'actor': ['owner_id','session_id','session_generation','instance_id','instance_generation'],
    'selected': ['schema_version','owner_id','selected_object_id','selected_generation','revision','content_hash','byte_len','manifest_hash'],
    'allocation': ['schema_version','actor','selected_object_id','selected_generation','operation_id','expected_revision','expected_content_hash'],
    'source': ['source_object_id','source_generation','byte_len','content_hash'],
    'binding': ['schema_version','allocation','source'],
    'permit': ['schema_version','binding','successor_revision','service_epoch','writer_id','admission_epoch'],
    'receipt': RECEIPT_KEYS,
    'record': ['allocation','allocated_service_epoch','binding','submitted_service_epoch','permit','closure','state','successor','receipt'],
    'journal': ['schema_version','owner_id','selected_object_id','selected_generation','signing_policy_hash','initial','current','service_epoch','operation_high_water','writer_high_water','transition_sequence','prior_journal_hash','operations'],
}
PURE_CHILDREN = {
    'allocation': {'actor':'actor'},
    'binding': {'allocation':'allocation','source':'source'},
    'permit': {'binding':'binding'},
    'record': {'allocation':'allocation','binding':'binding','permit':'permit','successor':'selected','receipt':'receipt'},
    'journal': {'initial':'selected','current':'selected','operations':'record_array'},
}


def ordered_pure(value, kind):
    if kind=='record_array':
        return [ordered_pure(record,'record') for record in rows(value,16)]
    keys(value,PURE_FIELDS[kind])
    result={}
    for name in PURE_FIELDS[kind]:
        nested=PURE_CHILDREN.get(kind,{}).get(name)
        result[name]=ordered_pure(value[name],nested) if nested is not None and value[name] is not None else value[name]
    return result


def journal_bytes(raw):
    j=strict_json(raw)
    fields=['schema_version','owner_id','selected_object_id','selected_generation','signing_policy_hash','initial','current','service_epoch','operation_high_water','writer_high_water','transition_sequence','prior_journal_hash','operations']
    keys(j,fields);require(type(j['schema_version']) is int and j['schema_version']==1,'journal schema')
    require(compact(ordered_pure(j,'journal'))==raw,'complete nested typed canonical journal bytes')
    for key in ('owner_id','selected_object_id','selected_generation','service_epoch'):uint(j[key],positive=True)
    for key in ('operation_high_water','writer_high_water'):uint(j[key])
    uint(j['transition_sequence'],positive=True)
    hash32(j['signing_policy_hash']);hash32(j['prior_journal_hash']);selected(j['initial']);selected(j['current'])
    for selected_record in (j['initial'],j['current']):require((selected_record['owner_id'],selected_record['selected_object_id'],selected_record['selected_generation'])==(j['owner_id'],j['selected_object_id'],j['selected_generation']),'journal selected identity')
    current=j['initial'];previous=0;active=0;writers=set()
    for op in rows(j['operations'],16):
        operation(op);require(op['allocated_service_epoch']<=j['service_epoch'],'journal allocation epoch')
        if op['submitted_service_epoch'] is not None:require(op['submitted_service_epoch']<=j['service_epoch'],'journal submitted epoch')
        a=op['allocation'];require(a['operation_id']>previous and a['operation_id']<=j['operation_high_water'],'journal op highwater/order');previous=a['operation_id']
        require((a['actor']['owner_id'],a['selected_object_id'],a['selected_generation'])==(j['owner_id'],j['selected_object_id'],j['selected_generation']),'journal op owner')
        if op['permit'] is not None:
            writer=op['permit']['writer_id'];require(writer<=j['writer_high_water'] and writer not in writers,'journal writer identity');writers.add(writer)
        if op['state']=='committed':
            require(a['expected_revision']==current['revision'] and a['expected_content_hash']==current['content_hash'],'journal selected chain');current=op['successor']
        if op['state']=='permitted' or (op['state']=='submitted' and op['closure']=='none'):active+=1
    require(active<=1 and current==j['current'],'journal terminal chain')
    return j


CODEC_MAX_CALLS = 19968  #48*(256 authorized lease rows+160 copied file rows).
CODEC_PHASE_SECONDS = 120.0
CODEC_CHILD_SECONDS = 2.0
CODEC_CLEANUP_RESERVE_SECONDS = 0.25
CODEC_INPUT_CAP = 131072
CODEC_STDOUT_CAP = 4096
CODEC_STDERR_CAP = 256
CODEC_BINARY_CAP = 268435456
RESULT_CAP = 65536
SEMANTIC_CASES = (
    'test_case_leg_inventory', 'test_epoch_before_after_inventory',
    'test_original_epoch_trace_preserved', 'test_raw_fence_digest_join_bindings',
    'test_actual_authorized_text_receipt_bytes', 'test_lease_call_epoch_identity',
    'test_seven_malformed_input_actual_errors',
    'test_canonical_descriptor_24_denials_three_states',
    'test_original_reconciled_receipt_no_replay',
    'test_reopen_phase_states_and_failures', 'test_explicit_owned_root_cleanup',
    'test_six_healthy_and_epochmax_controls',
    'test_volatile_UI_run_font_frame_provenance', 'test_queue_reset_frame_terminal',
    'test_file_row_aggregate_bounds', 'test_source_nonce_owned_process_binding',
)


def bounded_result_json(value, cap):
    raw=bytearray()
    for piece in json.JSONEncoder(ensure_ascii=False,separators=(',',':'),allow_nan=False).iterencode(value):
        block=piece.encode('utf8')
        require(len(raw)+len(block)+1<=cap,'result bound before growth')
        raw.extend(block)
    raw.extend(b'\n')
    return bytes(raw)


class PinnedCodec:
    """Root-selected trusted executable; one actually owned child at a time.

    Path/inode/hash checks do not establish isolation from a malicious host
    owner changing an executable between checks. No fixture-selected path/PID
    is opened, executed or signalled.
    """
    def __init__(self,path,digest):
        require(type(path) is str and 0<len(path.encode())<=4096,'codec path bound')
        require(Path(path).is_absolute(),'codec must be explicit absolute trusted path')
        self.path=path;self.digest=hexhash(digest);self.fd=None
        try:
            self.fd=os.open(path,os.O_RDONLY|os.O_NOFOLLOW|os.O_CLOEXEC)
            self.identity=file_identity(os.fstat(self.fd))
            self.check(full_hash=True)
        except BaseException:
            self.close();raise
        self.started=time.monotonic();self.deadline=self.started+CODEC_PHASE_SECONDS
        self.counts=collections.Counter();self.calls=0;self.reaped=0
        self.statuses=collections.Counter();self.transcript=hashlib.sha256()
        self.first_pid=None;self.last_pid=None

    def close(self):
        if self.fd is not None:os.close(self.fd);self.fd=None

    def check(self,full_hash=False):
        info=os.fstat(self.fd)
        require(stat.S_ISREG(info.st_mode) and info.st_mode&0o111 and 0<info.st_size<=CODEC_BINARY_CAP,'codec executable admission')
        require(file_identity(info)==self.identity,'held codec identity changed')
        path_info=os.stat(self.path,follow_symlinks=False)
        require(file_identity(path_info)==self.identity,'selected codec path changed')
        if full_hash:
            os.lseek(self.fd,0,os.SEEK_SET);digest=hashlib.sha256();count=0
            while count<info.st_size:
                block=os.read(self.fd,min(65536,info.st_size-count))
                require(bool(block),'truncated codec binary');count+=len(block);digest.update(block)
            require(not os.read(self.fd,1),'codec binary grew')
            require(file_identity(os.fstat(self.fd))==self.identity,'codec changed while hashed')
            require(digest.hexdigest()==self.digest,'root-selected codec SHA mismatch')

    def run(self,kind,raw,expected):
        require(kind in ('text','receipt','journal'),'fixed codec kind')
        require(type(raw) is bytes and 0<len(raw)<=CODEC_INPUT_CAP,'codec input pre-growth bound')
        require(self.calls<CODEC_MAX_CALLS,'codec invocation cap before spawn')
        require(time.monotonic()+2*CODEC_CLEANUP_RESERVE_SECONDS<self.deadline,'codec phase lacks cleanup budget')
        self.check()
        child_deadline=min(self.deadline,time.monotonic()+CODEC_CHILD_SECONDS)
        process=None;held=False;accepted=False;out=bytearray();err=bytearray();offset=0
        out_eof=err_eof=False;status=None
        def reap():
            nonlocal held,status
            if held:
                observed=process.poll()  #waitpid on this actual Popen child only.
                if observed is not None:
                    held=False  #Retire signalling identity BEFORE diagnostics.
                    status=observed
            return status
        def cleanup():
            nonlocal held
            if not held:return
            reap()
            if held:process.kill()  #Popen also polls before sending to its owned PID.
            while held and time.monotonic()<child_deadline:
                reap()
                if held:time.sleep(min(0.005,max(0,child_deadline-time.monotonic())))
            require(not held,'codec actual reap uncertain')
        try:
            process=subprocess.Popen([self.path,kind],stdin=subprocess.PIPE,stdout=subprocess.PIPE,
                                     stderr=subprocess.PIPE,bufsize=0,close_fds=True)
            held=True;pid=process.pid
            uint(pid,(1<<31)-1,True)
            for stream in (process.stdin,process.stdout,process.stderr):os.set_blocking(stream.fileno(),False)
            while held or not out_eof or not err_eof:
                require(time.monotonic()<child_deadline-CODEC_CLEANUP_RESERVE_SECONDS,'codec deadline')
                reads=[]
                if not out_eof:reads.append(process.stdout.fileno())
                if not err_eof:reads.append(process.stderr.fileno())
                writes=[] if process.stdin.closed else [process.stdin.fileno()]
                ready,writable,_=select.select(reads,writes,[],min(0.01,max(0,child_deadline-CODEC_CLEANUP_RESERVE_SECONDS-time.monotonic())))
                if writable:
                    try:wrote=os.write(process.stdin.fileno(),raw[offset:offset+65536])
                    except BlockingIOError:wrote=0
                    require(wrote>=0,'codec write status');offset+=wrote
                    if offset==len(raw):process.stdin.close()
                for fd in ready:
                    cap,retained=(CODEC_STDOUT_CAP,len(out)) if fd==process.stdout.fileno() else (CODEC_STDERR_CAP,len(err))
                    try:block=os.read(fd,min(4096,cap+1-retained))
                    except BlockingIOError:continue
                    if fd==process.stdout.fileno():
                        if not block:out_eof=True
                        else:require(len(out)+len(block)<=CODEC_STDOUT_CAP,'codec stdout cap before growth');out.extend(block)
                    else:
                        if not block:err_eof=True
                        else:require(len(err)+len(block)<=CODEC_STDERR_CAP,'codec stderr cap before growth');err.extend(block)
                reap()
            require(not held and status==0,'codec nonzero native status')
            require(offset==len(raw),'codec did not consume complete bounded input')
            require(not err,'codec successful stderr must be empty')
            require(out.endswith(b'\n') and out.count(b'\n')==1,'codec exact one-line diagnostic')
            diagnostic=strict_json(bytes(out));keys(diagnostic,{'schema_version','kind','byte_len','sha256','summary'})
            require(type(diagnostic['schema_version']) is int and diagnostic['schema_version']==1,'codec diagnostic version')
            require(diagnostic['kind']==kind and type(diagnostic['byte_len']) is int and diagnostic['byte_len']==len(raw),'codec kind/length binding')
            require(hexhash(diagnostic['sha256'])==sha(raw),'codec full raw SHA binding')
            keys(diagnostic['summary'],expected)
            #Strict JSON already rejects floats; native expected scalar types
            #also prevent bool==integer equivalence in this exact comparison.
            for name,value in expected.items():
                require(type(diagnostic['summary'][name]) is type(value) and diagnostic['summary'][name]==value,'codec exact summary binding')
            self.check()
            observation={'ordinal':self.calls,'kind':kind,'native_pid':pid,'native_returncode':status,
                         'actually_reaped':not held,'input_bytes':len(raw),'input_sha256':sha(raw),
                         'stdout_bytes':len(out),'stdout_sha256':sha(out),'stderr_bytes':len(err),
                         'stderr_sha256':sha(err)}
            canonical=compact(observation)
            require(len(canonical)<=512,'codec transcript row cap')
            require(time.monotonic()<child_deadline,'codec final acceptance deadline')
            self.transcript.update(struct.pack('<I',len(canonical)));self.transcript.update(canonical)
            self.calls+=1;self.reaped+=1;self.counts[kind]+=1;self.statuses[str(status)]+=1
            if self.first_pid is None:self.first_pid=pid
            self.last_pid=pid;accepted=True
        finally:
            try:
                if process is not None:cleanup()
            finally:
                if process is not None:
                    if not accepted:
                        failure={'kind':kind,'actual_native_pid':process.pid,'actual_native_returncode':status,
                                 'actually_reaped':not held,'input_sha256':sha(raw),
                                 'stdout_bytes':len(out),'stdout_sha256':sha(out),
                                 'stderr_bytes':len(err),'stderr_sha256':sha(err),
                                 'status':'INCOMPLETE_CODEC_INVOCATION'}
                        sys.stderr.write(bounded_result_json(failure,1024).decode('utf8'))
                    for stream in (process.stdin,process.stdout,process.stderr):
                        if stream is not None:stream.close()

    def summary(self):
        self.check(full_hash=True)
        require(self.calls>0 and self.calls==self.reaped and self.statuses=={'0':self.calls},'codec count/native status binding')
        require(time.monotonic()<=self.deadline,'codec whole phase deadline')
        return {'binary_sha256':self.digest,'binary_bytes':self.identity[2],
                'invocation_count':self.calls,'kind_counts':{k:self.counts[k] for k in ('text','receipt','journal')},
                'native_returncode_counts':dict(self.statuses),'actual_reaped_count':self.reaped,
                'first_actual_pid':self.first_pid,'last_actual_pid':self.last_pid,
                'transcript_sha256':self.transcript.hexdigest(),
                'transcript_scope':'ordered length-framed canonical actual invocation observations; retained digest, not per-call payload export',
                'elapsed_ms':int((time.monotonic()-self.started)*1000),
                'limits':{'invocations':CODEC_MAX_CALLS,'phase_ms':120000,'child_ms':2000,
                          'cleanup_reserve_ms':250,'input_bytes':CODEC_INPUT_CAP,
                          'stdout_bytes':CODEC_STDOUT_CAP,'stderr_bytes':CODEC_STDERR_CAP},
                'claim':'pure decoding of captured bytes only; maximum admissible inventory completion is not guaranteed within deadline'}


def codec_expected(kind,raw):
    if kind=='text':
        obj,rev,body,digest=text_bytes(raw)
        return {'selected_object_id':obj,'revision':rev,'body_byte_len':len(body),'content_hash':digest.hex()}
    if kind=='receipt':
        value=receipt_bytes(raw)
        return {k:(hash32(value[k]).hex() if k in ('expected_content_hash','source_content_hash') else value[k])
                for k in RECEIPT_KEYS if k not in ('schema_version','total_len','reserved')}
    journal=journal_bytes(raw)
    expected={k:journal[k] for k in ('owner_id','selected_object_id','selected_generation','service_epoch',
                                    'operation_high_water','writer_high_water','transition_sequence')}
    expected.update(revision=journal['current']['revision'],operation_count=len(journal['operations']),
                    last_operation_id=journal['operations'][-1]['allocation']['operation_id'] if journal['operations'] else None,
                    content_hash=hash32(journal['current']['content_hash']).hex())
    return expected


def decode_captured(data,codec):
    wanted=collections.Counter()
    for fixture in data.fixtures():
        for record,raw in lease_records(fixture):
            kind='text' if record['kind']=='selected_text' else 'receipt'
            wanted[kind]+=1;codec.run(kind,raw,codec_expected(kind,raw))
        for row in rows(fixture.summary['actual_files'],160):
            if row['kind']=='regular' and row['relative_path'].endswith('/selection.json') and (fixture.case,fixture.leg) not in FAILED_REOPEN:
                raw=fixture.raw(row['evidence_file'],131072)
                wanted['journal']+=1;codec.run('journal',raw,codec_expected('journal',raw))
    require(codec.counts==wanted,'every captured eligible payload decoded with multiplicity')
    require(wanted['text']>0 and wanted['receipt']>0 and wanted['journal']>0,'codec positive kind inventory')
    return codec.summary()


def publish_result(data,args,codec_summary):
    raw=bounded_result_json({'schema_version':1,'gate':'foundry-desktop-editor-store-ui1-1b','status':'PASS',
        'behavior_cases':CASE,'behavior_case_count':7,'semantic_cases':list(SEMANTIC_CASES),'semantic_case_count':16,
        'runtime_contract_sha256':RUNTIME_CONTRACT_SHA,
        'evidence_contract_sha256':CONTRACT_PINS['editor-store-recording-composite-frozen.json'],
        'source_manifest_sha256':args.source_manifest_sha256,'source_count':len(data.trusted_manifest),
        'run_nonce':data.nonce,'actual_test_pid':data.pid,'test_process_start_identity':data.birth,
        'codec':codec_summary,
        'scope':'default-off host Store CAS fixture evidence and reused volatile UI-a controls',
        'claims':{'host_cas':True,'pure_codec':True,'integrated_editor_save':False,'target':False,
                  'device_flush':False,'power_loss':False,'containment':False,'runtime_isolation':False}},RESULT_CAP)
    require(sum(t.total_bytes for t in data.opened)+len(raw)<=1073741824,'result global bytes admission')
    require(len(data.runner.inventory)<36 and len(data.runner.inventory)+1+sum(len(t.inventory) for t in (data.store,data.ui))<=20436,'result global regular-file admission')
    log_names={data.provenance[k]['relative_file'] for k in ('test_log','test_list_log')}
    other_bytes=sum(n for p,(_,_,n) in data.runner.inventory.items() if p not in log_names)
    require(other_bytes+len(raw)<=4194304,'result runner component admission')
    fd=os.open('result.json',os.O_WRONLY|os.O_CREAT|os.O_EXCL|os.O_NOFOLLOW|os.O_CLOEXEC,0o600,dir_fd=data.runner.fd)
    try:
        offset=0
        while offset<len(raw):
            count=os.write(fd,raw[offset:]);require(count>0,'result write status');offset+=count
        os.fsync(fd)
    finally:os.close(fd)


def main():
    global ARGS
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--evidence',required=True)
    parser.add_argument('--source-manifest',required=True)
    parser.add_argument('--source-manifest-sha256',required=True)
    parser.add_argument('--codec-binary',required=True)
    parser.add_argument('--codec-binary-sha256',required=True)
    ARGS=parser.parse_args()  #Reject arbitrary unittest filters and unknown args.
    require(type(ARGS.evidence) is str and 0<len(ARGS.evidence.encode())<=4096,'evidence root bound')
    root=Path(ARGS.evidence)
    ARGS.store_evidence=str(root/'cases');ARGS.ui_evidence=str(root/'ui-a');ARGS.provenance=str(root/'provenance.json')
    actual=tuple(k for k in RecordingContractTests.__dict__ if k.startswith('test_'))
    require(actual==SEMANTIC_CASES,'exact reviewed semantic method inventory')
    data=None;codec=None
    try:
        data=Data(ARGS)
        require(data.sources.get('docs/contracts/editor-store-recording-v0.json',{}).get('sha256')==RUNTIME_CONTRACT_SHA,'trusted closure must include the fixed runtime contract')
        own_source=read_regular(HERE/'editor_store_result.py',262144)
        require(data.sources.get('tools/ci/editor_store_result.py',{}).get('sha256')==sha(own_source),'trusted closure must bind the actual consumer source')
        require('result.json' not in data.runner.inventory,'existing result cannot be replayed/overwritten')
        #Check both independent contexts before semantic test execution.
        codec=PinnedCodec(ARGS.codec_binary,ARGS.codec_binary_sha256)
        suite=unittest.TestSuite(RecordingContractTests(name) for name in SEMANTIC_CASES)
        result=unittest.TextTestRunner(verbosity=2).run(suite)
        require(result.testsRun==16 and result.wasSuccessful() and not result.failures and not result.errors
                and not result.skipped and not result.expectedFailures and not result.unexpectedSuccesses,'sixteen actual semantic methods must succeed')
        #The decode phase begins AFTER the sixteen semantic methods. No reset
        #occurs between invocations or their cleanup.
        codec.started=time.monotonic();codec.deadline=codec.started+CODEC_PHASE_SECONDS
        codec_summary=decode_captured(data,codec)
        publish_result(data,ARGS,codec_summary)
        return 0
    finally:
        if codec is not None:codec.close()
        if data is not None:data.close()


if __name__=='__main__':
    try:sys.exit(main())
    except (EvidenceDenied,OSError,ValueError,subprocess.SubprocessError):
        #No partial/replayed result is promoted. Root retains native nonzero
        #status and its own bounded execution logs/cleanup observations.
        print('editor Store evidence: INCOMPLETE or failed validation',file=sys.stderr)
        sys.exit(1)
