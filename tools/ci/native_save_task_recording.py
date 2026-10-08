#!/usr/bin/env python3
"""Source-only NativeSave exported-data reader successor; held predecessor is immutable.

This module does not run tests, spawn children, signal artifact PIDs, open artifact
root paths, grant authority, or infer successful behavior from JSON markers.
The coordinator owns process/source admission and the eventual trusted inventory.
This entry is a source-only integration candidate. It is not connected to an
outer gate; runtime admission and the trusted verifier remain root-owned. Component validators
below operate on bounded copied data, not service authority or lifecycle handles.
"""
from collections import Counter
import hashlib
import json
import os
import stat
import re
import struct
import time

# Historical creation pins only; final acceptance still requires root-frozen inputs.
DTO_SCHEMA_SHA256 = 'f73ae7df8ef548a27139b96c7c324fccbb8e3e840814902990e81d817f88f03b'
SHAPE_SHA256 = 'b4737b69423ed320f91ddfea54eff923bd8a73658d6a3036f36ff8d57662717c'
CONTRACT_SHA256 = '241bda5a96f79e806f100dc72e411e2efb97dea51ff02f0717f82a2000e4cf9c'
FONT_SHA256 = '50ab6522c495f06a69590ccb6958458be15e02a2ca4c6b1024b052e826d21c64'
U64_MAX = (1 << 64) - 1
FORBIDDEN_KEYS = frozenset(('pass_marker', 'pass', 'accepted', 'runtime_pass'))


class Denied(ValueError):
    """Bounded data diagnostic, never an authority or execution verdict."""


def require(condition, reason):
    if not condition:
        raise Denied(reason)


def digest(data):
    require(type(data) is bytes, 'digest input type')
    return hashlib.sha256(data).hexdigest()


def integer(value, maximum=U64_MAX, minimum=0):
    require(type(value) is int and minimum <= value <= maximum, 'native integer bounds')
    return value


def exact_keys(value, names):
    require(type(value) is dict and set(value) == set(names), 'closed record keys')


def hex_bytes(value, maximum, exact=None):
    require(type(value) is str and len(value) <= maximum * 2
            and len(value) % 2 == 0 and re.fullmatch('[0-9a-f]*', value) is not None,
            'bounded lowercase hex')
    if exact is not None:
        require(len(value) == exact * 2, 'exact hex length')
    return bytes.fromhex(value)


def bounded_json(data, maximum):
    require(type(data) is bytes and 0 < len(data) <= maximum, 'JSON byte bounds')
    # Bound lexical nesting and element starts before the allocating JSON parser.
    depth = 0
    quoted = False
    escaped = False
    starts = 0
    for byte in data:
        if quoted:
            if escaped:
                escaped = False
            elif byte == 92:
                escaped = True
            elif byte == 34:
                quoted = False
            continue
        if byte == 34:
            quoted = True
            starts += 1
        elif byte in (91, 123):
            depth += 1
            starts += 1
            require(depth <= 64, 'JSON nesting bound')
        elif byte in (93, 125):
            depth -= 1
            require(depth >= 0, 'JSON nesting mismatch')
        elif byte in (44, 58):
            starts += 1
        require(starts <= 4_194_304, 'JSON lexical allocation bound')
    require(not quoted and depth == 0, 'JSON lexical completion')

    def pairs(rows):
        result = {}
        for key, value in rows:
            require(key not in result and key not in FORBIDDEN_KEYS, 'duplicate/forbidden JSON key')
            result[key] = value
        return result

    def forbidden_number(_):
        raise Denied('noninteger JSON number')

    try:
        return json.loads(data.decode('utf8', errors='strict'), object_pairs_hook=pairs,
                          parse_float=forbidden_number, parse_constant=forbidden_number)
    except (UnicodeError, json.JSONDecodeError, RecursionError) as error:
        raise Denied('invalid bounded JSON') from error


def validate_shape(value, spec, definitions, depth=0):
    """Closed trusted JSON-schema subset used by the admitted46 DTO definitions.

    Schema is coordinator-selected source input, never loaded from a record $ref.
    Unknown executable schema features fail closed; native bool is not integer.
    """
    require(depth <= 64 and type(spec) is dict, 'trusted schema depth/type')
    if 'not' in spec:
        try:
            validate_shape(value, spec['not'], definitions, depth + 1)
        except Denied:
            pass
        else:
            raise Denied('forbidden schema value')
    if '$ref' in spec:
        ref = spec['$ref']
        require(type(ref) is str and ref.startswith('#/$defs/')
                and ref[8:] in definitions, 'local trusted schema reference')
        return validate_shape(value, definitions[ref[8:]], definitions, depth + 1)
    if 'anyOf' in spec:
        alternatives = spec['anyOf']
        require(type(alternatives) is list and 0 < len(alternatives) <= 4, 'schema alternative bound')
        for alternative in alternatives:
            try:
                validate_shape(value, alternative, definitions, depth + 1)
                return
            except Denied:
                continue
        raise Denied('no admitted schema alternative')
    if 'const' in spec:
        require(type(value) is type(spec['const']) and value == spec['const'], 'schema constant')
    if 'enum' in spec:
        require(any(type(value) is type(item) and value == item for item in spec['enum']),
                'closed schema enum')
    kind = spec.get('type')
    if kind == 'null':
        require(value is None, 'schema null')
    elif kind == 'boolean':
        require(type(value) is bool, 'schema boolean')
    elif kind == 'integer':
        integer(value, spec.get('maximum', U64_MAX), spec.get('minimum', 0))
    elif kind == 'string':
        require(type(value) is str and spec.get('minLength', 0) <= len(value)
                <= spec.get('maxLength', 4096), 'schema string length')
        if 'x-maxUtf8Bytes' in spec:
            require(len(value.encode('utf8', errors='strict')) <= spec['x-maxUtf8Bytes'],
                    'schema UTF8 bytes')
        if 'pattern' in spec:
            require(re.fullmatch(spec['pattern'], value) is not None, 'schema string pattern')
    elif kind == 'array':
        require(type(value) is list and spec.get('minItems', 0) <= len(value)
                <= spec.get('maxItems', 0), 'schema array bounds')
        prefix = spec.get('prefixItems')
        if prefix is not None:
            require(len(value) == len(prefix) and spec.get('items') is False, 'fixed tuple arity')
            for item, item_spec in zip(value, prefix):
                validate_shape(item, item_spec, definitions, depth + 1)
        else:
            require(type(spec.get('items')) is dict, 'array item schema')
            for item in value:
                validate_shape(item, spec['items'], definitions, depth + 1)
    elif kind == 'object':
        properties = spec.get('properties', {})
        require(spec.get('additionalProperties') is False
                and set(spec.get('required', [])) == set(properties), 'closed trusted object schema')
        exact_keys(value, properties)
        for key, item_spec in properties.items():
            validate_shape(value[key], item_spec, definitions, depth + 1)
    elif kind is None:
        require('const' in spec or 'enum' in spec, 'missing schema type')
    else:
        raise Denied('unsupported trusted schema type')


def decode_handle(packed, expected_kind=None, invalid_allowed=False, zero_index_allowed=False):
    integer(packed)
    if packed == 0 and invalid_allowed:
        return (0, 0, 0)
    kind, reserved = packed >> 56, (packed >> 48) & 255
    index, generation = (packed >> 32) & 65535, packed & 0xffffffff
    require(reserved == 0 and kind in (1, 2, 3)
            and (index > 0 or zero_index_allowed) and generation > 0,
            'canonical packed handle')
    require(expected_kind is None or kind == expected_kind, 'packed handle kind')
    return kind, index, generation


# Exact current initialized editor wire layouts. Artifact9/10 is the additive
# frozen Save observation contract; existing43 messages are preserved.
WIRE_LAYOUTS = {
    802: ((32, None, None), (24, 20, 16), (48, 44, None), (16, 12, 8)),
    832: ((56, None, None), (32, 28, 24), (48, None, None), (40, None, 36)),
    833: ((56, 52, None), (56, 52, 40), (32, 28, None), (32, None, 24),
          (48, 44, None), (24, 20, 16), (32, None, None), (24, 20, 16),
          (24, None, None), (16, 12, 8)),
    352: ((64, 60, None), (48, 44, 36), (32, None, None), (16, 12, 8),
          (40, None, None), (48, None, 44), (40, None, None), (48, None, 44),
          (40, None, None), (16, 12, 8), (40, None, None), (16, 12, 8),
          (40, None, None), (48, 44, 36), (40, None, None), (32, 28, 24),
          (48, None, 44)),
    368: ((40, None, None), (40, None, 36), (32, None, None), (24, 20, 16),
          (56, 52, None), (40, None, 32), (32, None, None), (40, None, 32),
          (32, None, None), (32, 28, 24)),
}


def u32(data, offset):
    require(type(data) is bytes and 0 <= offset <= len(data) - 4, 'u32 wire offset')
    return struct.unpack_from('<I', data, offset)[0]


def u64(data, offset):
    require(type(data) is bytes and 0 <= offset <= len(data) - 8, 'u64 wire offset')
    return struct.unpack_from('<Q', data, offset)[0]


def decode_wire88(data):
    require(type(data) is bytes and len(data) == 88, 'Envelope exact88')
    protocol, message, handle, payload_len = struct.unpack_from('<IIQI', data)
    require(protocol in WIRE_LAYOUTS and 1 <= message <= len(WIRE_LAYOUTS[protocol]),
            'registered editor protocol/message')
    length, reserved, status_offset = WIRE_LAYOUTS[protocol][message - 1]
    require(payload_len == length and not any(data[20 + length:84])
            and not any(data[84:]), 'initialized Envelope bytes')
    payload = data[20:20 + length]
    require(reserved is None or u32(payload, reserved) == 0, 'message reserved zero')
    decode_handle(handle, invalid_allowed=True)
    status = None if status_offset is None else u32(payload, status_offset)
    require(status is None or status <= 11, 'closed status value')
    return {'protocol': protocol, 'message': message, 'handle': handle,
            'payload': payload, 'request_id': u64(payload, 0), 'status': status}


def exchange_pair(request, reply, *, native_error_redaction=False):
    q, r = decode_wire88(request), decode_wire88(reply)
    require(q['message'] % 2 == 1 and q['message'] != 17
            and r['protocol'] == q['protocol'] and r['message'] == q['message'] + 1,
            'actual request/reply pair')
    correlation = u64(q['payload'], 24) if (q['protocol'], q['message']) == (802, 3) else q['request_id']
    require(correlation == r['request_id'], 'actual exact-layout reply correlation')
    if native_error_redaction:
        require(r['handle'] == 0, 'all native replies use INVALID header handle')
    if native_error_redaction and r['status'] not in (None, 0, 10):
        require(r['handle'] == 0, 'native denial INVALID reply handle')
        # Leave correlation and the exact status/reserved fields intact, deny
        # any logical operation/descriptor/data fields on a definitive error.
        _, _, offset = WIRE_LAYOUTS[r['protocol']][r['message'] - 1]
        masked = bytearray(r['payload'])
        masked[:8] = bytes(8)
        masked[offset:offset + 4] = bytes(4)
        # Conflict is the frozen first-Commit CAS exception: current selected
        # revision survives, while operation/receipt and every other field redact.
        if r['protocol'] == 368 and r['message'] == 6 and r['status'] == 7:
            require(u64(r['payload'], 16) > 0, 'Conflict actual current revision')
            masked[16:24] = bytes(8)
        require(not any(masked), 'native definitive error redaction')
    return q, r


def decode_text64(data):
    require(type(data) is bytes and 64 <= len(data) <= 4160, 'Text64 bounds')
    version, total = struct.unpack_from('<II', data)
    object_id, revision = struct.unpack_from('<QQ', data, 8)
    body_len, reserved = struct.unpack_from('<II', data, 56)
    require(version == 1 and total == len(data) and object_id > 0 and revision > 0
            and body_len == len(data) - 64 and reserved == 0, 'Text64 header')
    body = data[64:]
    require(all(byte in (9, 10) or 32 <= byte <= 126 for byte in body), 'Text64 ASCII')
    require(data[24:56].hex() == digest(body), 'Text64 actual content hash')
    return {'selected_object_id': object_id, 'revision': revision,
            'content_hash': data[24:56].hex(), 'byte_len': body_len, 'body': body}


def validate_actor(actor):
    exact_keys(actor, ('owner_id', 'session_id', 'session_generation',
                       'instance_id', 'instance_generation'))
    for value in actor.values():
        integer(value, minimum=1)


def validate_allocation(allocation):
    require(allocation['schema_version'] == 1, 'allocation version')
    validate_actor(allocation['actor'])
    for name in ('selected_object_id', 'selected_generation', 'operation_id', 'expected_revision'):
        integer(allocation[name], minimum=1)
    hex_bytes(allocation['expected_content_hash'], 32, 32)


def validate_binding(binding, *, native_source=False):
    require(binding['schema_version'] == 1, 'binding version')
    validate_allocation(binding['allocation'])
    source = binding['source']
    integer(source['source_object_id'], minimum=1)
    integer(source['source_generation'], minimum=1)
    integer(source['byte_len'], 4096)
    hex_bytes(source['content_hash'], 32, 32)
    if native_source:
        require(decode_handle(source['source_object_id'], 2)[2] == source['source_generation'],
                'native source descriptor generation')


def decode_receipt176(data):
    require(type(data) is bytes and len(data) == 176, 'Receipt exact176')
    version, total, outcome, reserved = struct.unpack_from('<IIII', data)
    require(version == 1 and total == 176 and outcome in (0, 1) and reserved == 0,
            'Receipt header/state')
    names = ('owner_id', 'session_id', 'session_generation', 'original_instance_id',
             'original_instance_generation', 'selected_object_id', 'selected_generation',
             'operation_id', 'expected_revision', 'result_revision',
             'backend_service_epoch', 'committed_at_ms')
    result = dict(zip(names, struct.unpack_from('<12Q', data, 16)))
    result.update(schema_version=1, total_len=176, reserved=0,
                  outcome=('committed' if outcome == 0 else 'definitive_noncommit'),
                  expected_content_hash=data[112:144].hex(), source_content_hash=data[144:].hex())
    for name in names[:9] + ('backend_service_epoch',):
        integer(result[name], minimum=1)
    if outcome == 0:
        require(result['expected_revision'] < U64_MAX
                and result['result_revision'] == result['expected_revision'] + 1,
                'Receipt committed successor')
    else:
        require(result['result_revision'] == 0 and result['committed_at_ms'] == 0,
                'Receipt definitive noncommit')
    return result


def receipt_binding(receipt, binding, original_epoch):
    validate_binding(binding)
    a, source = binding['allocation'], binding['source']
    actor = a['actor']
    pairs = {'owner_id': actor['owner_id'], 'session_id': actor['session_id'],
             'session_generation': actor['session_generation'],
             'original_instance_id': actor['instance_id'],
             'original_instance_generation': actor['instance_generation'],
             'selected_object_id': a['selected_object_id'],
             'selected_generation': a['selected_generation'], 'operation_id': a['operation_id'],
             'expected_revision': a['expected_revision'],
             'expected_content_hash': a['expected_content_hash'],
             'source_content_hash': source['content_hash'], 'backend_service_epoch': original_epoch}
    integer(original_epoch, minimum=1)
    require(all(receipt[name] == value for name, value in pairs.items()), 'complete original receipt join')
    require(receipt['schema_version'] == 1 and receipt['total_len'] == 176
            and receipt['reserved'] == 0, 'retained receipt header')


def selected_validate(selected, identity=None):
    require(selected['schema_version'] == 1, 'selection version')
    for name in ('owner_id', 'selected_object_id', 'selected_generation', 'revision'):
        integer(selected[name], minimum=1)
    integer(selected['byte_len'], 4096)
    hex_bytes(selected['content_hash'], 32, 32)
    hex_bytes(selected['manifest_hash'], 32, 32)
    if identity is not None:
        require(all(selected[name] == identity[name] for name in
                    ('owner_id', 'selected_object_id', 'selected_generation')), 'selection object binding')


def journal_validate(journal):
    """Semantic translation of the existing pure canonical journal contract.

    Input has already passed the exact DTO schema; this does not authenticate the
    manifest, establish Store admission, or certify filesystem durability.
    """
    require(journal['schema_version'] == 1, 'journal version')
    for name in ('owner_id', 'selected_object_id', 'selected_generation',
                 'service_epoch', 'transition_sequence'):
        integer(journal[name], minimum=1)
    for name in ('operation_high_water', 'writer_high_water'):
        integer(journal[name])
    hex_bytes(journal['signing_policy_hash'], 32, 32)
    hex_bytes(journal['prior_journal_hash'], 32, 32)
    selected_validate(journal['initial'], journal)
    selected_validate(journal['current'], journal)
    operations = journal['operations']
    require(type(operations) is list and len(operations) <= 16, 'journal operation bound')
    current = journal['initial']
    previous_id, active = 0, 0
    writers = set()
    for row in operations:
        allocation = row['allocation']
        validate_allocation(allocation)
        require(allocation['actor']['owner_id'] == journal['owner_id']
                and allocation['selected_object_id'] == journal['selected_object_id']
                and allocation['selected_generation'] == journal['selected_generation'], 'journal actor/object')
        operation_id = allocation['operation_id']
        require(previous_id < operation_id <= journal['operation_high_water'], 'journal ordered IDs')
        previous_id = operation_id
        allocated_epoch = integer(row['allocated_service_epoch'], journal['service_epoch'], 1)
        binding, submitted, permit, receipt, successor = (
            row['binding'], row['submitted_service_epoch'], row['permit'], row['receipt'], row['successor'])
        require((binding is None) == (submitted is None), 'submitted binding/epoch pair')
        if binding is not None:
            validate_binding(binding)
            require(binding['allocation'] == allocation, 'submitted allocation equality')
            integer(submitted, journal['service_epoch'], allocated_epoch)
        if permit is not None:
            require(permit['schema_version'] == 1 and permit['binding'] == binding,
                    'immutable permit binding')
            validate_binding(permit['binding'])
            require(allocation['expected_revision'] < U64_MAX
                    and permit['successor_revision'] == allocation['expected_revision'] + 1,
                    'permit successor')
            require(permit['service_epoch'] == submitted and submitted is not None, 'permit original epoch')
            writer = integer(permit['writer_id'], journal['writer_high_water'], 1)
            require(writer not in writers, 'one retained writer identity')
            writers.add(writer)
            integer(permit['admission_epoch'], minimum=1)
        if receipt is not None:
            require(binding is not None, 'receipt without binding')
            receipt_binding(receipt, binding, submitted)
            # Check the same receipt state constraints as the176 codec, even
            # before a particular raw copied receipt is joined to this row.
            if receipt['outcome'] == 'committed':
                require(receipt['expected_revision'] < U64_MAX and
                        receipt['result_revision'] == receipt['expected_revision'] + 1,
                        'retained receipt successor')
            else:
                require(receipt['outcome'] == 'definitive_noncommit'
                        and receipt['result_revision'] == 0 and receipt['committed_at_ms'] == 0,
                        'retained receipt noncommit')
        if successor is not None:
            selected_validate(successor, journal)
        state = row['state']
        if state == 'allocated':
            require(all(value is None for value in (binding, permit, receipt, successor))
                    and row['closure'] == 'none', 'allocated canonical state')
        elif state == 'submitted':
            require(binding is not None and all(value is None for value in (permit, receipt, successor)),
                    'submitted canonical state')
            active += int(row['closure'] == 'none')
        elif state == 'permitted':
            require(binding is not None and permit is not None and receipt is None and successor is None,
                    'permitted canonical state')
            active += 1
        elif state == 'committed':
            require(permit is not None and receipt is not None and successor is not None,
                    'committed canonical state')
            require(receipt['outcome'] == 'committed'
                    and receipt['result_revision'] == permit['successor_revision']
                    and successor['revision'] == permit['successor_revision']
                    and successor['content_hash'] == binding['source']['content_hash']
                    and successor['byte_len'] == binding['source']['byte_len'], 'committed full successor')
            require(allocation['expected_revision'] == current['revision']
                    and allocation['expected_content_hash'] == current['content_hash'], 'actual CAS chain')
            current = successor
        elif state == 'noncommit':
            require(successor is None, 'noncommit no successor')
            require((binding is None and receipt is None and permit is None)
                    or (binding is not None and receipt is not None
                        and receipt['outcome'] == 'definitive_noncommit'), 'noncommit canonical state')
        else:
            raise Denied('unknown journal operation state')
    require(active <= 1 and current == journal['current'], 'journal active/current chain')


def canonical_journal(data, definitions):
    parsed = bounded_json(data, 131072)
    # Native raw journal Hash32 uses integer arrays. Convert ONLY those fields
    # according to trusted typed schemas; object declaration order is preserved.
    def convert(value, spec):
        if '$ref' in spec:
            name = spec['$ref'][8:]
            if name == 'Hash32':
                require(type(value) is list and len(value) == 32, 'native Hash32 integer array')
                return bytes(integer(item, 255) for item in value).hex()
            return convert(value, definitions[name])
        if 'anyOf' in spec:
            if value is None:
                return None
            nonnull = [part for part in spec['anyOf'] if part.get('type') != 'null']
            require(len(nonnull) == 1, 'native optional schema')
            return convert(value, nonnull[0])
        if spec.get('type') == 'object':
            properties = spec['properties']
            require(type(value) is dict and list(value) == list(properties), 'native declaration field order')
            return {key: convert(value[key], part) for key, part in properties.items()}
        if spec.get('type') == 'array':
            require(type(value) is list and len(value) <= spec.get('maxItems', 0), 'native nested array bound')
            return [convert(item, spec['items']) for item in value]
        return value
    normalized = convert(parsed, definitions['EditorSelectionJournalV0'])
    validate_shape(normalized, definitions['EditorSelectionJournalV0'], definitions)
    journal_validate(normalized)
    require(json.dumps(parsed, separators=(',', ':'), ensure_ascii=False).encode('utf8') == data,
            'native compact canonical journal bytes')
    return normalized


def chrome248(data, binding=None, receipt_data=None, original_epoch=None):
    require(type(data) is bytes and len(data) == 248, 'Save chrome exact248')
    version, total, state, reserved = struct.unpack_from('<IIII', data)
    require(version == 2 and total == 248 and state <= 4 and reserved == 0
            and u32(data, 244) == 0, 'Save chrome header/state')
    names = ('owner_id', 'session_id', 'session_generation', 'instance_id', 'instance_generation',
             'object_id', 'object_generation', 'current_epoch', 'original_epoch', 'operation_id',
             'source_handle', 'source_generation', 'expected_revision', 'result_revision',
             'attachment_version', 'status_version')
    out = dict(zip(names, struct.unpack_from('<16Q', data, 16)))
    out.update(state=state, source_content_hash=data[144:176].hex(),
               expected_content_hash=data[176:208].hex(), receipt_sha256=data[208:240].hex(),
               source_body_len=u32(data, 240))
    for name in names[:8] + ('attachment_version', 'status_version'):
        integer(out[name], minimum=1)
    if state in (0, 4):
        require(not any(data[80:128]) and not any(data[144:244]), 'idle/unavailable forbidden original fields')
        require(binding is None and receipt_data is None, 'idle/unavailable no original association')
        return out
    for name in ('original_epoch', 'operation_id', 'expected_revision'):
        integer(out[name], minimum=1)
    integer(out['source_body_len'], 4096)
    require(decode_handle(out['source_handle'], 2)[2] == out['source_generation'], 'chrome source generation')
    if state == 1:
        require(out['result_revision'] == 0 and not any(data[208:240]) and receipt_data is None,
                'Commit Unknown no receipt/successor')
    elif state == 2:
        require(out['expected_revision'] < U64_MAX
                and out['result_revision'] == out['expected_revision'] + 1, 'chrome committed successor')
    else:
        require(out['result_revision'] == 0, 'chrome noncommit no successor')
    if binding is not None:
        validate_binding(binding, native_source=True)
        a, source = binding['allocation'], binding['source']
        require({name: out[name] for name in ('owner_id', 'session_id', 'session_generation',
                                            'instance_id', 'instance_generation')} == a['actor'],
                'chrome original actor')
        expected = {'object_id': a['selected_object_id'], 'object_generation': a['selected_generation'],
                    'operation_id': a['operation_id'], 'expected_revision': a['expected_revision'],
                    'expected_content_hash': a['expected_content_hash'],
                    'source_handle': source['source_object_id'], 'source_generation': source['source_generation'],
                    'source_body_len': source['byte_len'], 'source_content_hash': source['content_hash'],
                    'original_epoch': original_epoch}
        require(all(out[key] == value for key, value in expected.items()), 'chrome full original binding')
    if receipt_data is not None:
        require(binding is not None and state in (2, 3), 'chrome receipt association')
        receipt = decode_receipt176(receipt_data)
        receipt_binding(receipt, binding, original_epoch)
        require(receipt['outcome'] == ('committed' if state == 2 else 'definitive_noncommit')
                and receipt['result_revision'] == out['result_revision']
                and digest(receipt_data) == out['receipt_sha256'], 'chrome exact copied receipt digest/outcome')
    return out


def frame_crops(bgra):
    require(type(bgra) is bytes and len(bgra) == 640 * 568 * 4, 'composed BGRA exact shape')
    app = bgra[48 * 640 * 4:528 * 640 * 4]
    protected = bgra[:48 * 640 * 4] + bgra[528 * 640 * 4:]
    return digest(app), digest(protected)


def protected_raster(bgra, label, banner, font):
    """Independent entire protected raster, including explicit empty banner."""
    require(type(font) is bytes and len(font) == 1536 and digest(font) == FONT_SHA256,
            'pinned actual font bytes')
    require(label in ('LOADED', 'UNSAVED', 'SAVING', 'SAVED', 'CONFLICT', 'UNAVAILABLE') and
            banner in ('', 'SAVE UNKNOWN', 'ALLOCATION UNKNOWN', 'SAVE UNKNOWN | ALLOCATION UNKNOWN'),
            'closed source/protected publication literals')
    require(type(bgra) is bytes and len(bgra) == 640 * 568 * 4, 'protected frame shape')
    expected = bytearray(bytes((224, 224, 224, 255)) * (640 * 88))
    def pixel(x, y, value):
        row = y if y < 48 else y - 480
        require(0 <= x < 640 and (0 <= y < 48 or 528 <= y < 568), 'protected raster fixed bounds')
        offset = (row * 640 + x) * 4
        expected[offset:offset + 4] = bytes(value)
    for y in range(8, 24):
        for x in range(8, 24):
            pixel(x, y, (0, 0, 0, 255) if x in (8, 23) or y in (8, 23) else (0, 192, 0, 255))
    for text, x, y in (('HOST STORE', 40, 8), (banner, 8, 24), (label, 8, 536)):
        for index, char in enumerate(text):
            for dy in range(16):
                for dx in range(8):
                    bit = font[(ord(char) - 32) * 16 + dy] & (128 >> dx)
                    pixel(x + index * 8 + dx, y + dy,
                          (0, 0, 0, 255) if bit else (224, 224, 224, 255))
    require(bgra[:48 * 2560] + bgra[528 * 2560:] == bytes(expected),
            'complete protected title/marker/banner/footer/background exact raster')


def validate_fence(snapshot):
    require(snapshot['schema_version'] == 1, 'fence schema')
    base = snapshot['base']
    require(base['schema_version'] == 1 and base['live_ownership_after_join'] == 0,
            'actual fence no live ownership')
    require(len(base['objects']) <= 3 and len(base['object_workers']) <= 2
            and len(base['service_producers']) + len(base['object_workers']) <= 64,
            'combined actual fence roster cap')
    producer_ids = set()
    for producer in base['service_producers'] + base['object_workers']:
        producer_id = integer(producer['producer_id'], minimum=1)
        require(producer_id not in producer_ids and producer['producer_generation'] == 1
                and producer['completed_join'] is True and producer['live_ownership_after_join'] is False,
                'actual unique completed producer fence')
        producer_ids.add(producer_id)
        integer(producer['native_pid'], 0xffffffff, 1)
        require(type(producer['rust_thread_id']) is str
                and 0 < len(producer['rust_thread_id'].encode('utf8')) <= 64,
                'actual held handle thread metadata')
        if producer['settled_barrier'] is not None:
            require(producer['settled_barrier']['settled'] is True, 'fence settled owned barrier')
    object_ids = set()
    for obj in base['objects']:
        object_id = integer(obj['selected_object_id'], minimum=1)
        require(object_id not in object_ids, 'unique fence object')
        object_ids.add(object_id)
        require(len(obj['known_durable_operations']) <= 16
                and len(obj['issued_runtime_permits']) <= 16
                and len(obj['original_closures']) <= 16, 'fence retained witnesses bound')
        successor = obj['reserved_successor_epoch']
        require(successor is None or obj['service_epoch_at_fence'] < U64_MAX
                and successor == obj['service_epoch_at_fence'] + 1, 'fence reserved versus actual epoch')
    require(len(snapshot['original_tickets']) <= 96, 'fence real original ticket bound')
    keys = set()
    for ticket in snapshot['original_tickets']:
        wire = decode_wire88(hex_bytes(ticket['original_request'], 88, 88))
        require(wire['protocol'] == 368 and wire['message'] in (3, 5), 'fence original mutation kind')
        require(ticket['kind'] == ('Allocate' if wire['message'] == 3 else 'Commit'),
                'fence original kind binding')
        key = (ticket['origin_id'], wire['request_id'])
        require(key not in keys, 'fence exact original execution identity')
        keys.add(key)
        validate_actor(ticket['actor'])
        require(u64(wire['payload'], 8) == ticket['actor']['session_id']
                and u64(wire['payload'], 16) == ticket['actor']['session_generation'],
                'fence original actor correlation')



def descriptor_handle(descriptor, expected_kind, byte_len):
    require(descriptor['kind'] == expected_kind and descriptor['handle_kind'] == 2,
            'actual captured descriptor class')
    index = integer(descriptor['handle_index'], 65535, 1)
    generation = integer(descriptor['handle_generation'], 0xffffffff, 1)
    require(descriptor['object_generation'] == generation
            and descriptor['byte_len'] == byte_len, 'actual captured descriptor generation/length')
    return (2 << 56) | (index << 32) | generation


def actor_wire(actor, wire, instance=False):
    validate_actor(actor)
    require(u64(wire['payload'], 8) == actor['session_id']
            and u64(wire['payload'], 16) == actor['session_generation'],
            'actual wire actor session')
    if instance:
        require(u64(wire['payload'], 24) == actor['instance_id']
                and u64(wire['payload'], 32) == actor['instance_generation'],
                'actual wire actor instance')


def chrome_read248(data):
    # Read controls remain the original schema1 two-state codec, never silently
    # normalized to Save schema2 or to a current/committed mutation authority.
    require(type(data) is bytes and len(data) == 248, 'Read chrome exact248')
    require(u32(data, 0) == 1 and u32(data, 4) == 248 and u32(data, 8) in (0, 4)
            and u32(data, 12) == 0 and u32(data, 244) == 0, 'Read chrome header/state')
    names = ('owner_id', 'session_id', 'session_generation', 'instance_id',
             'instance_generation', 'object_id', 'object_generation', 'current_epoch')
    out = {name: u64(data, 16 + i * 8) for i, name in enumerate(names)}
    out.update(state=u32(data, 8), attachment_version=u64(data, 128),
               status_version=u64(data, 136))
    require(all(value > 0 for key, value in out.items() if key != 'state'),
            'Read chrome current positive identity')
    require(not any(data[80:128]) and not any(data[144:244]), 'Read chrome no original outcome')
    return out


def grant464(data, expected_phase, expected_schema):
    require(type(data) is bytes and len(data) == 464, 'grant exact464')
    require(expected_schema in (2, 3) and expected_phase in ('Preview', 'Active'),
            'trusted grant scope/phase')
    require(u32(data, 0) == expected_schema and u32(data, 4) == 464
            and u32(data, 8) == 63 and u32(data, 12) == 4 and u32(data, 236) == 0,
            'grant header/category/count/reserved')
    names = ('session_id', 'session_generation', 'instance_id', 'instance_generation',
             'object_id', 'object_generation', 'selected_revision', 'policy_revision',
             'plan_id', 'preview_revision', 'expires_at_ms', 'desktop_epoch')
    out = dict(zip(names, struct.unpack_from('<12Q', data, 16)))
    require(all(value > 0 for name, value in out.items()
                if name not in ('instance_id', 'instance_generation')), 'grant logical positive identity')
    preview = out['instance_id'] == 0 and out['instance_generation'] == 0
    active = out['instance_id'] > 0 and out['instance_generation'] > 0
    require((preview and expected_phase == 'Preview') or (active and expected_phase == 'Active'),
            'grant actual phase')
    require(struct.unpack_from('<8I', data, 208) == (4096, 640, 480, 2560, 1, 64, 1, 0),
            'grant fixed limits')
    out.update(application_hash=data[112:144].hex(), manifest_hash=data[144:176].hex(),
               selected_content_hash=data[176:208].hex())
    records = []
    for i in range(4):
        offset = 240 + i * 56
        handle, resource, generation, epoch, expiry = struct.unpack_from('<5Q', data, offset)
        cls, rights, protocol, reserved = struct.unpack_from('<4I', data, offset + 40)
        require((cls, rights, protocol, reserved) ==
                (i + 1, (1, 1, 15, 7 if expected_schema == 3 else 1)[i], 1, 0),
                'grant exact classes/rights/protocol/reserved')
        if preview:
            require(handle == 0, 'preview no executable handles')
        else:
            decode_handle(handle, 1, zero_index_allowed=True)
        if i < 2:
            require(resource == out['instance_id'] and generation == out['instance_generation'],
                    'grant self/focus resource')
        elif i == 2:
            require((resource == 0 and generation == 0) if preview else
                    (resource > 0 and generation > 0), 'grant surface resource')
        else:
            require(resource == out['object_id'] and generation == out['object_generation'],
                    'grant actual Store object resource')
        require(epoch == out['desktop_epoch'] if i < 3 else epoch > 0, 'grant service epoch')
        require(expiry == out['expires_at_ms'], 'grant original instance expiry')
        records.append({'handle': handle, 'resource_id': resource,
                        'resource_generation': generation, 'epoch': epoch, 'expiry': expiry})
    if active and expected_schema == 3:
        require(len({row['handle'] for row in records}) == 4, 'Save grant six-pair handle uniqueness')
    out['records'] = records
    return out


def captures_validate(batches, objects_by_core, raw_bytes, initial_by_object):
    """Crosslink actual full copies/freeze bytes; Core ordinal is not backend epoch.

    objects_by_core is supplied only from the matching actual admitted Store
    snapshots. raw_bytes holds previously bounded, hashed, root-relative files.
    It never reads a service root path or reencodes ObjectEvidence into data.
    """
    require(type(batches) is list and len(batches) <= 4, 'actual Core capture batches bound')
    core_ids = set()
    validated = []
    for batch in batches:
        core = integer(batch['epoch_index'], 3)
        require(core not in core_ids and core in objects_by_core, 'capture actual Core ordinal')
        core_ids.add(core)
        rows = batch['captures']
        require(type(rows) is list and len(rows) <= 256, 'complete Core capture256 bound')
        capture_ids = set()
        for row in rows:
            capture_id = integer(row['capture_id'], minimum=1)
            require(capture_id not in capture_ids, 'distinct actual Core capture identity')
            capture_ids.add(capture_id)
            validate_actor(row['actor'])
            integer(row['original_backend_epoch'], minimum=1)
            obj = next((obj for obj in objects_by_core[core]
                        if obj['selected']['selected_object_id'] == row['object_id']), None)
            require(obj is not None and obj['selected']['selected_generation'] == row['object_generation']
                    and obj['selected']['owner_id'] == row['actor']['owner_id'],
                    'capture actual owner/object generation')
            q = decode_wire88(hex_bytes(row['original_request'], 88, 88))
            cq = decode_wire88(hex_bytes(row['capture_request'], 88, 88))
            require(q['protocol'] == cq['protocol'] == 368, 'capture real Artifact controls')
            actor_wire(row['actor'], q, instance=q['message'] == 1)
            actor_wire(row['actor'], cq, instance=cq['message'] == 1)
            ref = row['raw']
            require(ref['file'] in raw_bytes, 'actual raw capture present')
            data = raw_bytes[ref['file']]
            require(len(data) == ref['byte_len'] and digest(data) == ref['sha256'], 'actual raw capture hash')
            if row['kind'] == 'SelectedTextCopy':
                require(ref['kind'] == 'text64' and row['operation_id'] is None
                        and row['intent_id'] is None and row['original_reply'] is not None
                        and row['capture_reply'] is not None and q['message'] == cq['message'] == 1
                        and row['original_request'] == row['capture_request']
                        and row['original_reply'] == row['capture_reply'], 'Selected actual Read original')
                _, r = exchange_pair(hex_bytes(row['original_request'], 88, 88),
                                     hex_bytes(row['original_reply'], 88, 88), native_error_redaction=True)
                header = decode_text64(data)
                handle = descriptor_handle(row['descriptor'], 'SelectedText', len(data))
                require(r['status'] == 0 and u64(r['payload'], 16) == handle
                        and u32(r['payload'], 32) == len(data) and r['handle'] == 0
                        and header['selected_object_id'] == row['object_id']
                        and u64(r['payload'], 8) == header['revision']
                        and u64(r['payload'], 24) == row['descriptor']['object_generation'],
                        'full Selected actual reply/descriptor/header hash')
                require(row['object_id'] in initial_by_object, 'trusted actual genesis inventory present')
                initial = initial_by_object[row['object_id']]
                require(initial['owner_id'] == row['actor']['owner_id']
                        and initial['generation'] == row['object_generation'], 'trusted genesis owner/object')
                selections = [{'revision': initial['revision'], 'content_hash': initial['body_sha256'],
                               'byte_len': initial['body_len']}, obj['selected']] + [operation['successor'] for operation in obj['operations']
                                                 if operation['successor'] is not None]
                require(any(selected['revision'] == header['revision']
                            and selected['content_hash'] == header['content_hash']
                            and selected['byte_len'] == header['byte_len'] for selected in selections),
                        'selected capture retained actual selection lineage')
                validated.append((core, row, header, None))
                continue
            require(q['message'] == 5, 'source/receipt actual original Commit')
            op = integer(row['operation_id'], minimum=1)
            integer(row['intent_id'], minimum=1)
            operation = next((operation for operation in obj['operations']
                              if operation['allocation']['operation_id'] == op), None)
            require(operation is not None and operation['binding'] is not None, 'capture actual bound operation')
            binding = operation['binding']
            validate_binding(binding, native_source=True)
            a, source = binding['allocation'], binding['source']
            require(a['actor'] == row['actor'] and operation['submitted_service_epoch'] ==
                    row['original_backend_epoch'] and u64(q['payload'], 32) == op
                    and u64(q['payload'], 24) == a['expected_revision']
                    and u64(q['payload'], 40) == source['source_object_id']
                    and u32(q['payload'], 48) == 64 + source['byte_len'], 'capture full actual original binding')
            if row['kind'] == 'DraftSourceFreeze':
                require(ref['kind'] == 'text64' and row['original_request'] == row['capture_request']
                        and row['original_reply'] is None and row['capture_reply'] is None,
                        'actual source freeze before reply')
                header = decode_text64(data)
                handle = descriptor_handle(row['descriptor'], 'DraftSource', len(data))
                require(handle == source['source_object_id'] and header['selected_object_id'] == row['object_id']
                        and header['revision'] == a['expected_revision']
                        and header['byte_len'] == source['byte_len']
                        and header['content_hash'] == source['content_hash'], 'full sealed source64 binding')
                validated.append((core, row, header, binding))
            else:
                require(row['kind'] == 'ReceiptCopy' and ref['kind'] == 'receipt176'
                        and row['capture_reply'] is not None, 'actual receipt full copy')
                _, r = exchange_pair(hex_bytes(row['capture_request'], 88, 88),
                                     hex_bytes(row['capture_reply'], 88, 88), native_error_redaction=True)
                require(cq['message'] in (5, 7) and u64(cq['payload'], 32 if cq['message'] == 5 else 24) == op,
                        'receipt original or fresh same-operation lookup')
                if cq['message'] == 5:
                    require(row['original_request'] == row['capture_request'], 'direct receipt original request')
                else:
                    require(cq['request_id'] != q['request_id'], 'fresh receipt lookup correlation')
                receipt = decode_receipt176(data)
                receipt_binding(receipt, binding, row['original_backend_epoch'])
                handle = descriptor_handle(row['descriptor'], 'Receipt', len(data))
                require(r['status'] == 0 and u64(r['payload'], 8) == op
                        and u64(r['payload'], 24) == handle
                        and u64(r['payload'], 16) == receipt['result_revision']
                        and u32(r['payload'], 36) == 176, 'actual receipt reply descriptor/result')
                require(operation['receipt'] == receipt, 'raw176 equals retained actual receipt')
                validated.append((core, row, receipt, binding))
    return validated



def frame_validate(frame, raw_bytes, font, bound_original=None):
    """Validate one actual frame publication and its supplied copied diagnostics.

    bound_original, when needed, is a verified (Binding, original_epoch, copied
    Receipt176-or-None) association from the same actual Core capture lineage.
    These values are data consistency checks, never source/publisher authority.
    """
    ref = frame['raw']
    require(ref['file'] in raw_bytes, 'frame actual raw present')
    bgra = raw_bytes[ref['file']]
    require(len(bgra) == ref['byte_len'] and digest(bgra) == ref['sha256'], 'frame actual raw length/hash')
    if ref['kind'] == 'app_frame':
        require(len(bgra) == 640 * 480 * 4 and digest(bgra) == frame['app_crop_sha256'],
                'actual app frame full crop')
        require(frame['chrome'] is None and frame['expected_protected_label'] is None
                and frame['expected_unknown_banner'] is None, 'app-only no protected publication claim')
    else:
        require(ref['kind'] == 'composed_frame', 'actual composed frame kind')
        app_hash, protected_hash = frame_crops(bgra)
        require((app_hash, protected_hash) ==
                (frame['app_crop_sha256'], frame['protected_crop_sha256']), 'same composed publication crops')
        if frame['expected_protected_label'] is not None:
            require(frame['expected_unknown_banner'] is not None, 'source exact banner includes absence')
            protected_raster(bgra, frame['expected_protected_label'], frame['expected_unknown_banner'], font)
    draft, join, chrome = frame['draft'], frame['frame_join'], frame['chrome']
    if draft is not None:
        validate_actor(draft['actor'])
        text = hex_bytes(draft['bytes'], 4096)
        require(all(byte in (9, 10) or 32 <= byte <= 126 for byte in text), 'actual draft bounded ASCII')
        integer(draft['cursor'], len(text))
        if draft['selection'] is not None:
            require(type(draft['selection']) is list and len(draft['selection']) == 2
                    and 0 <= draft['selection'][0] <= draft['selection'][1] <= len(text),
                    'actual draft selection bounds')
        require(draft['actor']['session_id'] == frame['session_id']
                and draft['actor']['instance_id'] == frame['instance_id'], 'frame draft actual actor')
        for key in ('document_id', 'text_generation', 'view_version', 'confirmed_revision'):
            integer(draft[key], minimum=1)
    if join is not None:
        require(draft is not None and join['actor'] == draft['actor'], 'frame join actual draft actor')
        require(all(join[key] == draft[key] for key in ('document_id', 'text_generation', 'view_version')),
                'frame join same document/text/view')
        require((join['focus_epoch'], join['sequence'], join['app_crop_sha256']) ==
                (frame['focus_epoch'], frame['sequence'], frame['app_crop_sha256']),
                'frame join same actual publication')
        for key in ('surface_id', 'surface_generation', 'mapping_generation', 'focus_epoch', 'sequence'):
            integer(join[key], minimum=1)
    if chrome is not None:
        require(ref['kind'] == 'composed_frame', 'protected chrome actual composed publication')
        validate_actor(chrome['current_frame_actor'])
        require(chrome['current_frame_actor']['session_id'] == frame['session_id'] and
                chrome['current_frame_actor']['instance_id'] == frame['instance_id'] and
                (chrome['focus_epoch'], chrome['frame_sequence']) == (frame['focus_epoch'], frame['sequence']),
                'chrome-only actual compositor actor/focus/sequence')
        if join is not None:
            require(chrome['current_frame_actor'] == join['actor'] and
                    all(chrome[key] == join[key] for key in ('document_id', 'text_generation', 'view_version')) and
                    chrome['current_document_phase'] == draft['phase'], 'protected same private current draft publication')
        if chrome['label'] == 'Saved':
            require(join is not None and draft is not None, 'Saved requires full private current draft/frame join')
        require(chrome['outstanding_original_count'] == len(chrome['pending_originals'])
                and len(chrome['pending_originals']) <= 48, 'protected complete pending original count')
        for original in chrome['pending_originals']:
            validate_actor(original['original_actor'])
            for key in ('object_id', 'object_generation', 'intent_id', 'original_backend_epoch'):
                integer(original[key], minimum=1)
        labels = {'Loaded': 'LOADED', 'Unsaved': 'UNSAVED', 'AllocationPending': 'SAVING',
                  'AllocationUnknown': 'UNSAVED', 'Saving': 'SAVING', 'Unknown': 'UNSAVED',
                  'Saved': 'SAVED', 'Conflict': 'CONFLICT', 'Unavailable': 'UNAVAILABLE'}
        require(chrome['label'] in labels and chrome['current_document_phase'] in labels and
                labels[chrome['label']] == labels[chrome['current_document_phase']],
                'current protected label exact document phase')
        kinds = {original['phase'] for original in chrome['pending_originals']}
        require(kinds <= {'AllocationUnknown', 'CommitUnknown'}, 'closed unresolved original banner phases')
        banner = ('SAVE UNKNOWN | ALLOCATION UNKNOWN' if len(kinds) == 2 else
                  'ALLOCATION UNKNOWN' if 'AllocationUnknown' in kinds else 'SAVE UNKNOWN' if kinds else '')
        # A supplied source expectation is additional evidence, never a way to
        # suppress absence or substitute a relocated subset of glyphs.
        if frame['expected_protected_label'] is not None:
            require((frame['expected_protected_label'], frame['expected_unknown_banner']) ==
                    (labels[chrome['label']], banner), 'source expectation equals actual publisher classification')
        protected_raster(bgra, labels[chrome['label']], banner, font)
        if chrome['record248'] is not None:
            data = hex_bytes(chrome['record248'], 248, 248)
            if u32(data, 0) == 1:
                require(bound_original is None, 'Read protected no mutation original')
                decoded = chrome_read248(data)
            else:
                if bound_original is None:
                    decoded = chrome248(data)
                    require(decoded['state'] in (0, 4), 'protected original outcome needs actual bound original')
                else:
                    binding, epoch, receipt = bound_original
                    decoded = chrome248(data, binding, receipt, epoch)
                    require(chrome['original_actor'] == binding['allocation']['actor'],
                            'protected original actual actor')
            require(decoded['attachment_version'] == chrome['attachment_version']
                    and decoded['status_version'] == chrome['status_version'], 'protected exact current versions')
            if chrome['label'] == 'Saved':
                require(bound_original is not None and decoded['state'] == 2
                        and digest(hex_bytes(draft['bytes'], 4096)) == decoded['source_content_hash']
                        and draft['confirmed_revision'] == decoded['result_revision']
                        and draft['confirmed_hash'] == decoded['source_content_hash'],
                        'Saved current draft equals actual committed source/outcome')
        else:
            require(chrome['label'] != 'Saved', 'Saved actual canonical outcome record required')
    return frame


def internally_joined_io_validate(row, snapshot, joins):
    # Quiesce retains an actual verified opaque IO join in its fence owner.
    # Validate that projection's closed provenance without inventing a caller row.
    base = snapshot['base']
    require(row in base['object_workers'] and row['kind'] == 'ObjectIoWorker'
            and row['completed_join'] is True and row['live_ownership_after_join'] is False
            and row['producer_generation'] == 1 and row['join_outcome'] == 'Returned',
            'internally joined IO actual completed worker only')
    object_id = integer(row['selected_object_id'], minimum=1)
    request_id = integer(row['request_id'], minimum=1)
    operation_id = integer(row['operation_id'], minimum=1)
    objects = [obj for obj in base['objects'] if obj['selected_object_id'] == object_id]
    require(len(objects) == 1, 'internally joined IO unique actual object')
    obj = objects[0]
    permits = [permit for permit in obj['issued_runtime_permits']
               if permit['binding']['allocation']['operation_id'] == operation_id]
    closures = [closure for closure in obj['original_closures']
                if closure['allocation']['operation_id'] == operation_id
                and closure['original_request_id'] == request_id]
    require(len(permits) == 1 and len(closures) == 1,
            'internally joined IO unique actual permit and closure')
    permit, closure = permits[0], closures[0]
    binding = permit['binding']
    validate_binding(binding, native_source=True)
    allocation, source = binding['allocation'], binding['source']
    require(permit['schema_version'] == 1
            and allocation['selected_object_id'] == object_id
            and allocation['selected_generation'] == obj['selected_generation']
            and allocation['actor']['owner_id'] == obj['owner_id']
            and permit['service_epoch'] == obj['service_epoch_at_fence']
            and allocation['expected_revision'] < U64_MAX
            and permit['successor_revision'] == allocation['expected_revision'] + 1
            and closure['allocation'] == allocation and closure['submitted_binding'] == binding
            and closure['closure'] == 'service_retired'
            and closure['original_producer_generation'] == 1,
            'internally joined IO complete original permit/closure binding')
    integer(permit['writer_id'], obj['writer_high_water_min'], 1)
    integer(permit['admission_epoch'], obj['admission_epoch_min'], 1)
    tickets = [ticket for ticket in snapshot['original_tickets']
               if ticket['kind'] == 'Commit' and ticket['binding'] == binding
               and ticket['actor'] == allocation['actor'] and ticket['object_id'] == object_id
               and ticket['original_epoch'] == permit['service_epoch']
               and ticket['closed'] is True and ticket['commit_dispatched'] is True
               and ticket['completion'] == 'JoinedReturned']
    require(len(tickets) == 1, 'internally joined IO unique closed original Commit')
    ticket = tickets[0]
    wire = decode_wire88(hex_bytes(ticket['original_request'], 88, 88))
    require(wire['protocol'] == 368 and wire['message'] == 5
            and wire['request_id'] == request_id
            and u64(wire['payload'], 8) == allocation['actor']['session_id']
            and u64(wire['payload'], 16) == allocation['actor']['session_generation']
            and u64(wire['payload'], 24) == allocation['expected_revision']
            and u64(wire['payload'], 32) == operation_id
            and u64(wire['payload'], 40) == source['source_object_id']
            and u64(wire['payload'], 48) == 64 + source['byte_len'],
            'internally joined IO complete canonical original Commit request')
    dispatch = joins.get(closure['original_producer_id'])
    require(dispatch is not None and dispatch['kind'] == 'StoreDispatch'
            and dispatch['outcome'] == 'Returned' and dispatch['origin_id'] == ticket['origin_id'],
            'internally joined IO real returned original Store dispatch')
    supervisors = [join for join in joins.values()
                   if join['kind'] == 'StoreSupervisor' and join['outcome'] == 'Returned'
                   and join['origin_id'] == ticket['origin_id']]
    require(len(supervisors) == 1, 'internally joined IO unique returned Store supervisor')
    supervisor = supervisors[0]
    require(len({row['producer_id'], dispatch['producer_id'], supervisor['producer_id']}) == 3,
            'internally joined IO distinct actual producer identities')
    require(dispatch['producer_generation'] == supervisor['producer_generation'] == 1
            and row['native_pid'] == dispatch['native_pid'] == supervisor['native_pid']
            and len({row['rust_thread_id'], dispatch['rust_thread_id'], supervisor['rust_thread_id']}) == 3,
            'internally joined IO actual returned parent process and distinct threads')
    for parent, kind in ((dispatch, 'Dispatcher'), (supervisor, 'Supervisor')):
        retained = [producer for producer in base['service_producers']
                    if producer['producer_id'] == parent['producer_id']]
        if parent is supervisor and not retained:
            # A real supervisor join may have been delivered before quiesce.
            # Only producers still retained at closure enter the fence roster.
            require(all(supervisor['rust_thread_id'] != producer['rust_thread_id']
                        for producer in base['service_producers'] + base['object_workers']),
                    'internally joined IO earlier returned supervisor distinct held thread')
            continue
        require(len(retained) == 1, 'internally joined IO actual retained parent')
        producer = retained[0]
        require(producer['kind'] == kind and producer['join_outcome'] == 'Returned'
                and all(parent[key] == producer[key] for key in
                        ('producer_generation', 'native_pid', 'rust_thread_id'))
                and producer['selected_object_id'] == object_id
                and producer['request_id'] == request_id and producer['operation_id'] == operation_id
                and row['native_pid'] == parent['native_pid'],
                'internally joined IO exact original parent metadata')
    require(all(row['rust_thread_id'] != producer['rust_thread_id']
                for producer in base['service_producers'] + base['object_workers']
                if producer['producer_id'] != row['producer_id']),
            'internally joined IO actual distinct held thread')
    barrier = row['settled_barrier']
    require(barrier is not None and barrier['request_id'] == request_id
            and barrier['operation_id'] == operation_id
            and all(barrier[key] is True for key in ('entered', 'released', 'settled', 'permit_issued')),
            'internally joined IO actual settled permitted pause')


def join_fence_validate(join_rows, fences):
    require(type(join_rows) is list and len(join_rows) <= 65536, 'complete native join bound')
    joins = {}
    for row in join_rows:
        producer = integer(row['producer_id'], minimum=1)
        require(row['producer_generation'] == 1 and producer not in joins, 'actual unique producer join')
        integer(row['origin_id'], minimum=1)
        integer(row['native_pid'], 0xffffffff, 1)
        require(row['outcome'] in ('Returned', 'Panicked') and type(row['rust_thread_id']) is str
                and 0 < len(row['rust_thread_id'].encode('utf8')) <= 64, 'actual finished owned join data')
        joins[producer] = row
    for snapshot in fences:
        validate_fence(snapshot)
        for row in snapshot['base']['service_producers'] + snapshot['base']['object_workers']:
            if row['producer_id'] not in joins:
                internally_joined_io_validate(row, snapshot, joins)
                continue
            join = joins[row['producer_id']]
            require(all(join[key] == row[key] for key in
                        ('producer_generation', 'native_pid', 'rust_thread_id'))
                    and join['outcome'] == row['join_outcome']
                    and {'StoreDispatch': 'Dispatcher', 'StoreSupervisor': 'Supervisor',
                         'DesktopDispatch': 'Dispatcher', 'DesktopSupervisor': 'Supervisor',
                         'StoreIo': 'ObjectIoWorker'}.get(join['kind']) == row['kind'],
                    'fence same actual producer/thread join outcome')
    return joins



def trusted_inventory(data, definitions, admitted_sources, expected_sha256,
                      expected_contract_sha256, expected_recording_sha256):
    """Admit independently root-pinned source inventory, never a cases file.

    admitted_sources are the outer runner's actual held pre/post verified source
    records. The caller must choose the independent inventory pin before child
    execution; the record cannot select this context or substitute its own pin.
    """
    require(type(expected_sha256) is str and re.fullmatch('[0-9a-f]{64}', expected_sha256),
            'independent inventory pin required')
    require(digest(data) == expected_sha256, 'trusted inventory raw pin')
    inventory = bounded_json(data, 1_048_576)
    require('SourceInventory' in definitions and 'SourceCase' in definitions,
            'root-reviewed inventory schema present')
    validate_shape(inventory, definitions['SourceInventory'], definitions)
    require(inventory['contract_sha256'] == expected_contract_sha256
            and inventory['recording_contract_sha256'] == expected_recording_sha256,
            'inventory separately frozen contract pins')
    require(type(admitted_sources) is list and 0 < len(admitted_sources) <= 1024,
            'actual outer source closure required')
    source_map = {}
    for row in admitted_sources:
        exact_keys(row, ('relative_path', 'byte_len', 'sha256'))
        name = row['relative_path']
        require(type(name) is str and name not in source_map and 0 < len(name.encode('utf8')) <= 256
                and not name.startswith('/') and all(part not in ('', '.', '..')
                for part in name.split('/')), 'actual unique source path')
        integer(row['byte_len'], 8_388_608, 1)
        hex_bytes(row['sha256'], 32, 32)
        source_map[name] = row
    names = set()
    for row in inventory['assertion_sources']:
        require(row['relative_path'] not in names and source_map.get(row['relative_path']) == row,
                'inventory actual assertion/compiler source pin')
        names.add(row['relative_path'])
    require(len(names) > 0, 'inventory actual source pins nonempty')
    required_cases = definitions['SourceCase']['properties']['case']['enum']
    require(len(required_cases) == 18 and [case['case'] for case in inventory['cases']] == required_cases,
            'inventory exact eighteen maintained case order')
    scenarios = {}
    for case in inventory['cases']:
        require(0 < len(case['scenarios']) <= 128, 'every actual case has source-fixed scenarios')
        for scenario in case['scenarios']:
            stem, leg = scenario['scenario'], integer(scenario['source_leg'], 0xffffffff)
            require(re.fullmatch('[a-z0-9_-]{1,96}', stem) is not None,
                    'trusted scenario exact source-fixed stem')
            key = (case['case'], stem, leg)
            require(key not in scenarios and len(scenarios) < 128, 'global source scenario128 bound/uniqueness')
            path = '/'.join((case['case'], stem, str(leg)))
            require(len(path.encode('ascii')) <= 256, 'computed exact source scenario path')
            require(len(set(scenario['core_ordinals'])) == len(scenario['core_ordinals'])
                    and all(type(core) is int and 0 <= core <= 3 for core in scenario['core_ordinals']),
                    'actual Core ordinal inventory')
            require(len(set(scenario['record_names'])) == len(scenario['record_names'])
                    and 'summary.json' in scenario['record_names']
                    and 'cleanup.json' in scenario['record_names'], 'complete unique required record names')
            require(len(set(scenario['required_relations'])) == len(scenario['required_relations']),
                    'no duplicate required semantic relation')
            require(len(set(scenario['requirements'])) == len(scenario['requirements']),
                    'source requirement identity uniqueness')
            snapshot_ids = set()
            snapshot_purposes = {}
            for snap in scenario['store_snapshots']:
                pair = (snap['epoch_index'], snap['observation_index'])
                require(pair not in snapshot_ids and snap['epoch_index'] in scenario['core_ordinals'],
                        'source-fixed unique actual Store snapshot')
                snapshot_ids.add(pair)
                snapshot_purposes[pair] = snap['purpose']
                require(snap['durability'] in ('live_checkpoint', 'joined_checkpoint', 'negative_disk_fault'),
                        'source checkpoint classification closed')
                filename = 'store-epoch-%d-snapshot-%d.json' % pair
                require(filename in scenario['record_names'], 'actual snapshot file in source inventory')
            exception_ids = set()
            for exception in scenario['file_exceptions']:
                pair = (exception['epoch_index'], exception['observation_index'])
                name = exception['relative_path']
                require(pair in snapshot_purposes and snapshot_purposes[pair] == exception['snapshot_purpose'],
                        'trusted file exception actual snapshot identity')
                require(type(name) is str and not name.startswith('/') and
                        all(part not in ('', '.', '..') for part in name.split('/')),
                        'trusted exception exact owner-relative canonical path')
                key_exception = pair + (exception['snapshot_purpose'], name)
                require(key_exception not in exception_ids, 'trusted exact file exceptions unique')
                exception_ids.add(key_exception)
                if exception['allow_empty']:
                    allowed = {'corrupt_journal': 'TruncatedJournal', 'cas_blob': 'EmptyAsciiObject',
                               'fixture_file': 'SourceDeclaredEmptyFixture'}
                    require(allowed.get(exception['raw_kind']) == exception['reason'],
                            'trusted zero file has exact source-specific reason')
            # Repeated source loop sites remain distinct through exact ordered
            # multiplicity and actual per-family local observation indices.
            require([row['observation_index'] for row in scenario['grant_sites']] ==
                    list(range(len(scenario['grant_sites']))), 'source grant sites exact local0..N-1')
            require(len(set(row['kind'] for row in scenario['raw_limits'])) == len(scenario['raw_limits']),
                    'source raw kind limits unique')
            for row in scenario['raw_limits']:
                require(0 <= row['min_count'] <= row['max_count'] <= 2048,
                        'source raw count limits bounded')
            require(len(set(row['source_control'] for row in scenario['negative_controls'])) ==
                    len(scenario['negative_controls']), 'source negative controls unique')
            objects = scenario['initial_objects']
            require(len({row['object_id'] for row in objects}) == len(objects), 'source genesis objects unique')
            for row in objects:
                for name in ('owner_id', 'object_id', 'generation', 'revision'):
                    integer(row[name], minimum=1)
            control_sites_validate(scenario['control_sites'], case['case'], stem, leg,
                                   scenario['row_bounds']['control_observations'])
            scenarios[key] = {'path': path, 'binding_scenario': stem + '/' + str(leg), 'source': scenario}
    return inventory, scenarios


def negative_inputs_validate(rows, trusted_controls, raw_bytes):
    require(type(rows) is list and len(rows) <= 256, 'negative controls bounded')
    expected = {row['source_control']: row for row in trusted_controls}
    require(len(expected) == len(trusted_controls) and len(rows) == len(expected),
            'complete exact source negative inventory')
    seen = set()
    for row in rows:
        name = row['source_control']
        require(name in expected and name not in seen, 'source-fixed negative control identity')
        seen.add(name)
        require(all(row[key] == expected[name][key] for key in
                    ('kind', 'actual_denial_layer', 'actual_status', 'status_space')),
                'negative actual source-fixed layer/status namespace')
        require(row['actual_status'] in (1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11),
                'negative actual denial status non-success')
        ref = row['raw']
        require(ref['kind'] == row['kind'] and ref['file'] in raw_bytes,
                'negative actual raw classification')
        data = raw_bytes[ref['file']]
        require(len(data) == ref['byte_len'] and digest(data) == ref['sha256'],
                'negative original actual raw bytes')
        if row['kind'] == 'wire88_negative':
            require(len(data) == 88 and row['source_descriptor'] is None
                    and row['actual_write'] is None, 'negative wire no source-writing fiction')
            if row['actual_denial_layer'] == 'wire_decoder':
                require(row['status_space'] == 'DesktopStatus' and row['actual_status'] == 2
                        and row['actual_reply'] is None, 'actual wire decoder Invalid observation')
                try:
                    decode_wire88(data)
                except Denied:
                    pass
                else:
                    raise Denied('claimed decoder denial has canonical accepted wire')
            else:
                require(row['actual_denial_layer'] == 'native_call_admission'
                        and row['status_space'] == 'DesktopStatus', 'negative wire actual admission layer')
                decode_wire88(data)
        else:
            require(row['kind'] == 'text64_negative' and 64 <= len(data) <= 4160
                    and row['actual_denial_layer'] == 'native_source_consumption'
                    and row['status_space'] == 'StoreStatus' and row['source_descriptor'] is not None
                    and row['actual_write'] is not None and row['original_request'] is not None
                    and row['actual_reply'] is not None, 'negative actual source consumption boundary')
            handle = descriptor_handle(row['source_descriptor'], 'DraftSource', len(data))
            write = row['actual_write']
            require(write['offset'] == 0 and write['byte_len'] == len(data) and write['status'] == 0,
                    'negative actual successful mutable full source write')
            q, r = exchange_pair(hex_bytes(row['original_request'], 88, 88),
                                 hex_bytes(row['actual_reply'], 88, 88), native_error_redaction=True)
            require(q['protocol'] == 368 and q['message'] == 5 and u64(q['payload'], 40) == handle
                    and u32(q['payload'], 48) == len(data) and r['status'] == row['actual_status'],
                    'negative actual canonical Commit source descriptor/reply')
    return seen



def control_sites_validate(sites, case, scenario, source_leg, row_bound):
    """Closed trusted source groups; metadata cannot manufacture Option::None."""
    require(type(sites) is list and len(sites) <= 1024, 'source control groups1024 bound')
    integer(row_bound, 1024)
    total = 0
    for site in sites:
        exact_keys(site, ('purpose', 'call', 'kind', 'actual_status', 'min_count', 'max_count'))
        exact_keys(site['actual_status'], ('result', 'status_space', 'status'))
        integer(site['actual_status']['status'], 11)
        require(site['actual_status']['result'] in ('Ok', 'Err') and
                site['actual_status']['status_space'] in ('DesktopStatus', 'StoreStatus') and
                ((site['actual_status']['result'] == 'Ok' and site['actual_status']['status'] == 0) or
                 (site['actual_status']['result'] == 'Err' and site['actual_status']['status'] > 0)),
                'source group closed native typed result')
        minimum = integer(site['min_count'], 2)
        maximum = integer(site['max_count'], 2, 1)
        require(minimum <= maximum, 'source control group native min/max order')
        key = (case, scenario, source_leg, site['purpose'], site['call'], site['kind'],
               site['actual_status']['result'], site['actual_status']['status_space'], site['actual_status']['status'])
        if (minimum, maximum) != (1, 1):
            paused = (case == 'ui1_1_stalled_session_leaves_other_session_usable' and scenario == 'primary' and
                      type(source_leg) is int and source_leg in (0, 1) and
                      key[3:] == ('paused_join_not_ready', 'join_native_save_finished', 'unit_result', 'Err', 'StoreStatus', 6))
            optional = (case == 'ui1_1_store_lost_reply_reconciles_without_mutation_replay' and scenario == 'allocate_loss' and
                        type(source_leg) is int and source_leg == 0 and
                        key[3:] == ('control_main_020', 'source_for_intent', 'unit_result', 'Err', 'StoreStatus', 4))
            require((paused and (minimum, maximum) == (1, 2)) or
                    (optional and (minimum, maximum) == (0, 1)), 'only exact reviewed conditional source tuples')
        total += maximum
        require(total <= row_bound, 'sum source group maximums fits actual control row bound1024')
    return total


def control_observations_validate(rows, sites, *, case, scenario, source_leg, row_bound):
    """Actual typed control results, never manufactured endpoint exchanges."""
    limit = control_sites_validate(sites, case, scenario, source_leg, row_bound)
    require(type(rows) is list and len(rows) <= limit <= 1024,
            'complete actual rows fit admitted ordered source groups')
    # Finite dynamic positions admit complete ordered groups, including the sole
    # optional branch and1..2 real own-handle failures. No greedy branch guess,
    # row deduplication, synthesized None result or arbitrary omitted site.
    positions = {0}
    keys = ('purpose', 'call', 'kind', 'actual_status')
    for site in sites:
        following = set()
        for position in positions:
            for count in range(site['min_count'], site['max_count'] + 1):
                end = position + count
                if end <= len(rows) and all(all(rows[index][key] == site[key] for key in keys)
                                            for index in range(position, end)):
                    following.add(end)
        require(following and len(following) <= 1025, 'complete bounded ordered source control group matching')
        positions = following
    require(len(rows) in positions, 'every actual control consumed by exact ordered source groups')
    payloads = {'native_save_init': 'init', 'native_save_expose': 'expose',
                'native_save_delivery': 'delivery', 'inactive_save_desktop': 'inactive_desktop',
                'inactive_save_store': 'inactive_store', 'native_save_registration': 'registration'}
    for index, row in enumerate(rows):
        require(integer(row['observation_index'], 0xffffffff) == index, 'actual control local index exact0..N-1')
        status = row['actual_status']
        require(status['status_space'] in ('DesktopStatus', 'StoreStatus') and
                ((status['result'] == 'Ok' and status['status'] == 0) or
                 (status['result'] == 'Err' and 1 <= status['status'] <= 11)),
                'actual typed unit result exact status space/discriminant')
        field = payloads.get(row['kind'])
        require(row['kind'] == 'unit_result' or field is not None, 'known typed control kind')
        for name in payloads.values():
            require((row[name] is not None) == (name == field), 'one actual typed diagnostic variant only')
        if row['request'] is not None:
            decode_wire88(hex_bytes(row['request'], 88, 88))
        if field in ('init', 'expose', 'delivery'):
            require(row[field]['status'] == status['status'], 'actual control and diagnostic status agree')
        elif field == 'registration':
            registration = row[field]
            request = hex_bytes(registration['request'], 88, 88)
            decode_wire88(request)
            require(row['request'] is None or row['request'] == registration['request'],
                    'actual registration original canonical request matches diagnostic')
            require(len(set(registration['producer_ids'])) == len(registration['producer_ids']) and
                    all(integer(identifier, minimum=1) for identifier in registration['producer_ids']),
                    'actual registration producer identifiers unique positive diagnostics')
        elif field == 'inactive_desktop':
            diagnostic = row[field]
            for request, reply in zip(diagnostic['endpoint_requests'], diagnostic['endpoint_replies']):
                exchange_pair(hex_bytes(request, 88, 88), hex_bytes(reply, 88, 88),
                              native_error_redaction=True)
            for name in ('origin_status', 'bootstrap_status', 'grants_lease_status'):
                result = diagnostic[name]
                require(result['result'] == 'Err' and result['status_space'] == 'DesktopStatus'
                        and result['status'] == 6, 'actual inactive Desktop admission NotReady')
        elif field == 'inactive_store':
            result = row[field]['grant_status']
            require(result['result'] == 'Err' and result['status_space'] == 'StoreStatus'
                    and result['status'] == 6, 'actual inactive Store admission NotReady')


def scenario_records_validate(records, definitions, source, case, run_nonce):
    """Closed data records against one already-admitted source scenario tuple.

    Parsing/opening is outer-owned until the final held file admission is wired.
    Store snapshots are cumulative diagnostics, not additional executions. Their
    actual observation identity distinguishes same-Core changing file states.
    """
    require(type(records) is dict and set(records) == set(source['record_names']),
            'complete source-fixed scenario record inventory')
    stem, leg = source['scenario'], source['source_leg']
    scenario = stem + '/' + str(leg)
    snapshots = {('store-epoch-%d-snapshot-%d.json' % (row['epoch_index'], row['observation_index'])): row
                 for row in source['store_snapshots']}
    schemas = {'summary.json': 'SummaryRecord', 'desktop.json': 'DesktopRecord',
               'artifacts.json': 'ArtifactsRecord', 'joins.json': 'JoinsRecord'}
    schemas['cleanup.json'] = ('VolatileCleanup' if source['scope'] == 'volatile-control'
                              else 'NativeReadCleanup' if source['scope'] == 'native-read-control'
                              else 'NativeSaveCleanup')
    store = []
    for filename, record in records.items():
        kind = 'StoreEpochRecord' if filename in snapshots else schemas.get(filename)
        require(kind is not None and kind in definitions, 'every actual record has frozen schema')
        validate_shape(record, definitions[kind], definitions)
        binding = record['binding']
        require(binding['schema_version'] == 1 and binding['run_nonce'] == run_nonce
                and binding['case'] == case and binding['scenario'] == scenario
                and binding['scope'] == source['scope'], 'every actual record same run/source scenario')
        if filename in snapshots:
            expected = snapshots[filename]
            require(binding['epoch_index'] == expected['epoch_index']
                    and record['observation_index'] == expected['observation_index']
                    and record['purpose'] == expected['purpose'] and record['phase'] == expected['phase'],
                    'exact actual Store Core/observation/purpose/phase')
            store.append(record)
    summary = records['summary.json']
    require(len(summary['record_inventory']) == len(records) and len(set(summary['record_inventory'])) == len(records)
            and set(summary['record_inventory']) == set(records), 'summary complete unique record filenames')
    require(summary['source_case_requirement_ids'] == source['requirements'],
            'summary exact source requirements')
    require(summary['actual_store_epochs'] == source['core_ordinals'], 'summary exact actual Core ordinals')
    expected_snapshots = [{'epoch_index': row['epoch_index'], 'observation_index': row['observation_index'],
                           'purpose': row['purpose'], 'record': name} for name, row in snapshots.items()]
    require(summary['store_snapshots'] == expected_snapshots, 'summary exact source snapshot inventory')
    bounds = source['row_bounds']
    require(len(summary['frames']) <= bounds['frames']
            and len(summary['app_observations']) <= bounds['app_observations']
            and len(summary['negative_inputs']) <= bounds['negative_inputs'], 'actual summary source row bounds')
    require(len(summary['control_observations']) <= bounds['control_observations']
            and len(summary['producer_observations']) <= bounds['producer_observations']
            and len(summary['grant_observations']) <= bounds['grant_observations'],
            'actual diagnostic source row bounds')
    require([row['purpose'] for row in summary['producer_observations']] == source['producer_sites'],
            'actual producer observations exact source sites and order')
    require([{'observation_index': row['observation_index'], 'phase': row['phase']}
             for row in summary['grant_observations']] == source['grant_sites'],
            'actual grant observations exact source sites and order')
    control_observations_validate(summary['control_observations'], source['control_sites'],
                                  case=case, scenario=stem, source_leg=leg, row_bound=bounds['control_observations'])
    for family in ('frames', 'grant_observations'):
        require([row['observation_index'] for row in summary[family]] == list(range(len(summary[family]))),
                'actual independently consecutive local family indices: ' + family)
    previous_joined = 0
    for index, row in enumerate(summary['producer_observations']):
        require(row['observation_index'] == index, 'actual producer local0..N-1')
        counts = row['counts']
        require(counts['held'] <= 64 and counts['io'] <= 2 and counts['io'] <= counts['held']
                and counts['joining'] <= counts['held'] and counts['joined_total'] >= previous_joined,
                'actual shared producer counts bounded and cumulative')
        previous_joined = counts['joined_total']
    desktop = records.get('desktop.json')
    if desktop is not None:
        require(len(desktop['exchanges']) <= bounds['desktop_exchanges'], 'actual Desktop complete trace bound')
        require(desktop['volatile_backend'] is (source['scope'] in ('volatile-control', 'native-read-control')),
                'volatile versus actual Store scope stamp')
    for record in store:
        require(len(record['exchanges']) <= bounds['store_exchanges_per_snapshot']
                and len(record['io_events']) <= bounds['io_events_per_snapshot']
                and len(record['barriers']) <= bounds['barriers_per_snapshot']
                and len(record['files']) <= bounds['files_per_snapshot'], 'actual Store complete snapshot bounds')
        ledger = record['ledger']
        require(ledger['shared_objects'] <= 32 and ledger['active_io_workers'] <= 2
                and ledger['held_service_producers'] <= 64, 'actual Store shared/data/IO producer bounds')
        require(len({obj['selected']['selected_object_id'] for obj in record['objects']}) == len(record['objects']),
                'actual Store object inventory unique')
        for obj in record['objects']:
            selected_validate(obj['selected'])
            require(len(obj['operations']) <= 16 and obj['transition_count'] <= obj['permit_count']
                    <= obj['mutation_dispatch_count'], 'actual object mutation accounting')
            integer(obj['service_epoch'], minimum=1)
    artifacts = records.get('artifacts.json')
    if artifacts is not None:
        if source['scope'] in ('composition', 'direct-native'):
            require([batch['epoch_index'] for batch in artifacts['batches']] == source['core_ordinals'],
                    'complete actual Save Core artifact batches')
        else:
            require(source['scope'] in ('native-read-control', 'volatile-control') and artifacts['batches'] == [],
                    'Read/volatile cannot fabricate Save capture batches')
        for batch in artifacts['batches']:
            require(len(batch['captures']) <= bounds['captures_per_core'], 'actual capture source row bound')
    joins = records.get('joins.json')
    if joins is not None:
        require(len(joins['native_join_observations']) <= bounds['native_joins']
                and joins['held_after'] == 0, 'complete actual joined native producer inventory')
        if source['scope'] in ('composition', 'direct-native'):
            require([batch['epoch_index'] for batch in joins['fence_original_ticket_batches']] == source['core_ordinals'],
                    'complete actual Save Core original ticket batches')
        else:
            require(source['scope'] in ('native-read-control', 'volatile-control') and
                    joins['fence_original_ticket_batches'] == [], 'Read/volatile cannot mint Save original ticket batches')
        for batch in joins['fence_original_ticket_batches']:
            require(len(batch['original_tickets']) <= bounds['tickets_per_core'], 'actual source ticket retention bound')
    return summary, sorted(store, key=lambda row: (row['binding']['epoch_index'], row['observation_index']))



RAW_BOUNDS = {'text64': (64, 4160), 'receipt176': (176, 176), 'wire88': (88, 88),
              'journal': (1, 131072), 'corrupt_journal': (0, 131072),
              'cas_blob': (0, 4096), 'fixture_file': (0, 131072),
              'app_frame': (1228800, 1228800), 'composed_frame': (1454080, 1454080),
              'chrome248': (248, 248), 'wire88_negative': (88, 88),
              'text64_negative': (64, 4160), 'grant464': (464, 464)}


def stat_identity(info):
    return (info.st_dev, info.st_ino, info.st_mode, info.st_nlink, info.st_size,
            info.st_mtime_ns, info.st_ctime_ns, info.st_uid, info.st_gid)


class CaseFiles:
    """Bounded readonly admission of the outer's already-held fresh cases root.

    Never opens an artifact-supplied absolute path, service fixture root, PID,
    source file or command. The complete emitted tree must consist of genuine
    directories and single-link regular files. Zero-size data has no generic
    positive meaning and is checked again against trusted file exceptions.
    No imports or calls to this class have been executed during authoring.
    """
    def __init__(self, held_cases_fd, deadline):
        require(type(held_cases_fd) is int and held_cases_fd >= 0, 'outer actual held cases fd')
        self.deadline = deadline
        self.owner = os.getuid()
        check_deadline(deadline)
        self.fd = os.dup(held_cases_fd)
        self.files = {}
        self.directories = {}
        self.json_bytes = 0
        self.total_bytes = 0
        self.entries = 0
        self.case_json_bytes = Counter()
        self.inodes = set()
        try:
            root = os.fstat(self.fd)
            require(stat.S_ISDIR(root.st_mode) and root.st_uid == self.owner
                    and stat.S_IMODE(root.st_mode) == 0o700, 'outer owned0700 cases root')
            self._scan(self.fd, '', 0)
        except BaseException:
            os.close(self.fd)
            self.fd = None
            raise

    def close(self):
        if self.fd is not None:
            os.close(self.fd)
            self.fd = None

    @staticmethod
    def parts(relative):
        require(type(relative) is str and 0 < len(relative.encode('utf8')) <= 256,
                'bounded emitted relative path')
        parts = relative.split('/')
        require(len(parts) <= 8 and all(re.fullmatch('[A-Za-z0-9_.-]{1,128}', part) is not None
                and part not in ('.', '..') for part in parts), 'emitted path no aliases/traversal')
        return parts

    def _scan(self, directory, prefix, depth):
        check_deadline(self.deadline)
        require(depth <= 8, 'emitted tree depth8')
        before = os.fstat(directory)
        require(before.st_uid == self.owner and stat.S_IMODE(before.st_mode) == 0o700,
                'emitted actual owner/private0700 directory')
        inode = (before.st_dev, before.st_ino)
        require(inode not in self.inodes, 'emitted directory alias')
        self.inodes.add(inode)
        self.directories[prefix] = stat_identity(before)
        # scandir retains no unbounded list. Charge every observed entry before
        # insertion or recursion; sorting is unnecessary for exact set checks.
        with os.scandir(directory) as iterator:
            for entry in iterator:
                check_deadline(self.deadline)
                relative = prefix + '/' + entry.name if prefix else entry.name
                self.parts(relative)
                self.entries += 1
                require(self.entries <= 41115, 'emitted entries41115 before growth')
                info = entry.stat(follow_symlinks=False)
                if stat.S_ISDIR(info.st_mode):
                    require(depth < 8, 'emitted directory depth8')
                    child = os.open(entry.name, os.O_RDONLY | os.O_DIRECTORY | os.O_NOFOLLOW | os.O_CLOEXEC,
                                    dir_fd=directory)
                    try:
                        require(stat_identity(os.fstat(child)) == stat_identity(info), 'directory changed before hold')
                        self._scan(child, relative, depth + 1)
                    finally:
                        os.close(child)
                else:
                    require(stat.S_ISREG(info.st_mode) and info.st_nlink == 1
                            and info.st_uid == self.owner and stat.S_IMODE(info.st_mode) == 0o600
                            and 0 <= info.st_size <= 33554432, 'emitted owner0600/no hardlink/single32MiB')
                    inode = (info.st_dev, info.st_ino)
                    require(inode not in self.inodes and len(self.files) < 40960, 'emitted unique files40960')
                    self.inodes.add(inode)
                    require(self.total_bytes + info.st_size <= 2147483648, 'global emitted bytes2GiB')
                    self.total_bytes += info.st_size
                    if entry.name.endswith('.json'):
                        case = relative.split('/')[0]
                        require(self.json_bytes + info.st_size <= 536870912
                                and self.case_json_bytes[case] + info.st_size <= 134217728,
                                'global/per-case JSON pre-growth bounds')
                        self.json_bytes += info.st_size
                        self.case_json_bytes[case] += info.st_size
                    self.files[relative] = stat_identity(info)
        require(stat_identity(os.fstat(directory)) == stat_identity(before), 'emitted directory changed while scanning')

    def read(self, relative, maximum, minimum=1):
        check_deadline(self.deadline)
        parts = self.parts(relative)
        require(self.fd is not None and relative in self.files, 'held emitted file inventory')
        parent = os.dup(self.fd)
        try:
            prefix = ''
            for part in parts[:-1]:
                child = os.open(part, os.O_RDONLY | os.O_DIRECTORY | os.O_NOFOLLOW | os.O_CLOEXEC,
                                dir_fd=parent)
                os.close(parent)
                parent = child
                prefix = prefix + '/' + part if prefix else part
                require(prefix in self.directories and stat_identity(os.fstat(parent)) == self.directories[prefix],
                        'held emitted parent identity changed')
            fd = os.open(parts[-1], os.O_RDONLY | os.O_NOFOLLOW | os.O_NONBLOCK | os.O_CLOEXEC, dir_fd=parent)
            try:
                before = os.fstat(fd)
                require(stat_identity(before) == self.files[relative] and stat.S_ISREG(before.st_mode)
                        and before.st_nlink == 1 and minimum <= before.st_size <= maximum,
                        'held emitted leaf exact identity/narrow bounds')
                data = bytearray()
                while len(data) < before.st_size:
                    check_deadline(self.deadline)
                    block = os.read(fd, min(65536, before.st_size - len(data)))
                    require(bool(block) and len(data) + len(block) <= maximum, 'emitted read bounded/no truncation')
                    data.extend(block)
                require(not os.read(fd, 1) and stat_identity(os.fstat(fd)) == stat_identity(before),
                        'held emitted file exact EOF/unchanged')
                # A replaced directory entry must not hide behind a still-held
                # old leaf; recheck the original name under the same parent.
                require(stat_identity(os.stat(parts[-1], dir_fd=parent, follow_symlinks=False)) ==
                        stat_identity(before), 'emitted leaf replacement')
                check_deadline(self.deadline)
                return bytes(data)
            finally:
                os.close(fd)
        finally:
            os.close(parent)

    def raw(self, scenario_path, ref, empty_allowed=False):
        require(ref['kind'] in RAW_BOUNDS, 'raw closed kind bounds')
        low, high = RAW_BOUNDS[ref['kind']]
        length = integer(ref['byte_len'], high, low)
        require(length > 0 or empty_allowed is True, 'zero raw needs trusted exact exception/empty-object proof')
        data = self.read(scenario_path + '/' + ref['file'], high, 0 if empty_allowed else max(1, low))
        require(len(data) == length and digest(data) == ref['sha256'], 'actual raw held hash/length')
        return data



def io_events_validate(events, objects):
    require(type(events) is list and len(events) <= 65536, 'complete actual IO ledger bound')
    object_map = {obj['selected']['selected_object_id']: obj for obj in objects}
    require(len(object_map) == len(objects), 'actual IO objects unique')
    groups = {}
    previous = 0
    for event in events:
        sequence = integer(event['sequence'], minimum=1)
        require(sequence > previous, 'complete actual IO sequence ordering')
        previous = sequence
        require(event['selected_object_id'] in object_map, 'actual IO object scope')
        if event['artifact_hash'] is not None:
            hex_bytes(event['artifact_hash'], 32, 32)
        require(event['outcome'] in ('Began', 'Completed', 'SimulatedFailure', 'ActualFailure'),
                'actual IO outcome enum')
        writer, op = integer(event['writer_id']), integer(event['operation_id'])
        if writer == 0:
            # Allocation/reservation/provisioning/metadata IO never pretends to
            # be an immutable issued Commit permit or logical selection effect.
            continue
        obj = object_map[event['selected_object_id']]
        operation = next((row for row in obj['operations'] if row['allocation']['operation_id'] == op), None)
        require(operation is not None and operation['permit'] is not None
                and operation['permit']['writer_id'] == writer, 'actual IO immutable permit writer/operation')
        groups.setdefault((event['selected_object_id'], op, writer), []).append(event)
        if event['outcome'] == 'Completed' and event['point'] in ('CandidateBlobWrite', 'CandidateBlobSync'):
            require(event['artifact_hash'] == operation['binding']['source']['content_hash'],
                    'actual completed candidate body hash')
        if (event['outcome'] == 'Completed' and event['point'] == 'CandidateManifest'
                and operation['successor'] is not None):
            require(event['artifact_hash'] == operation['successor']['manifest_hash'],
                    'actual authenticated candidate manifest hash')
    return groups


def store_snapshots_validate(snapshots, definitions, raw_bytes, file_exceptions):
    """Crosslink captured files at the exact source-approved observation tuple.

    No service root is opened, followed or reconstructed. An exported symlink
    observation is data about the explicitly corrupted owned fixture; it is not
    permission to put a symlink in the emitted evidence tree.
    """
    require(type(snapshots) is list and len(snapshots) <= 64, 'actual Store snapshots64')
    exceptions = {}
    for row in file_exceptions:
        key = (row['epoch_index'], row['observation_index'], row['snapshot_purpose'], row['relative_path'])
        require(key not in exceptions, 'trusted file exception exact identity unique')
        exceptions[key] = row
    last_indexes = {}
    result = {}
    for snapshot in snapshots:
        core, index = snapshot['binding']['epoch_index'], snapshot['observation_index']
        require(core not in last_indexes or index > last_indexes[core], 'actual observation index monotonic per Core')
        last_indexes[core] = index
        io_events_validate(snapshot['io_events'], snapshot['objects'])
        hashes = {}
        paths = set()
        for observation in snapshot['files']:
            path = observation['relative_path']
            require(type(path) is str and 0 < len(path.encode('utf8')) <= 256
                    and not path.startswith('/') and all(part not in ('', '.', '..') for part in path.split('/'))
                    and path not in paths, 'actual owner-relative file inventory unique/canonical')
            paths.add(path)
            kind = observation['entry_kind']
            ref = observation['raw']
            if kind == 'regular':
                require(ref is not None and ref['file'] in raw_bytes, 'actual regular fixture raw copy required')
                data = raw_bytes[ref['file']]
                require(len(data) == observation['byte_len'] == ref['byte_len']
                        and digest(data) == observation['sha256'] == ref['sha256']
                        and observation['mode'] is not None and observation['symlink_target'] is None,
                        'actual observed regular file equals copied raw bytes')
                if len(data) == 0:
                    key = (core, index, snapshot['purpose'], path)
                    require(key in exceptions, 'actual empty file requires exact trusted checkpoint exception')
                    exception = exceptions[key]
                    require(exception['allow_empty'] is True and exception['raw_kind'] == ref['kind'],
                            'trusted exact empty file raw classification')
                    if ref['kind'] == 'corrupt_journal':
                        require(exception['reason'] == 'TruncatedJournal', 'only actual truncated empty journal')
                    elif ref['kind'] == 'fixture_file':
                        require(exception['reason'] == 'SourceDeclaredEmptyFixture',
                                'exact source-declared empty fixture')
                    else:
                        require(ref['kind'] == 'cas_blob' and exception['reason'] == 'EmptyAsciiObject',
                                'only legitimate empty ASCII CAS object')
                        require(path.split('/')[-1] == digest(b'') + '.blob', 'empty CAS exact content-addressed filename')
                        known_empty = []
                        for actual_snapshot in snapshots:
                            if actual_snapshot['binding']['epoch_index'] != core:
                                continue
                            for actual_file in actual_snapshot['files']:
                                r = actual_file['raw']
                                if r is None or r['kind'] != 'journal':
                                    continue
                                journal = canonical_journal(raw_bytes[r['file']], definitions)
                                parent = actual_file['relative_path'].rsplit('/', 1)[0] + '/cas/'
                                selections = [journal['initial'], journal['current']] + [row['successor']
                                              for row in journal['operations'] if row['successor'] is not None]
                                if path == parent + digest(b'') + '.blob' and any(
                                        selected['byte_len'] == 0 and selected['content_hash'] == digest(b'')
                                        for selected in selections):
                                    known_empty.append(journal['selected_object_id'])
                        require(known_empty, 'empty body actual complete canonical selected/initial/successor lineage')
                exception_key = (core, index, snapshot['purpose'], path)
                if ref['kind'] == 'corrupt_journal':
                    require(exception_key in exceptions and exceptions[exception_key]['raw_kind'] == 'corrupt_journal' and
                            exceptions[exception_key]['reason'] in ('TruncatedJournal', 'UnknownJournalVersion',
                            'ExtraJournalField', 'DuplicateJournalField', 'WrongJournalBinding', 'BrokenJournalChain',
                            'EarlierAllocatedOnlyJournal'), 'corrupt journal exact trusted fault/path, not phase implication')
                    require(path.endswith('/selection.json'), 'explicit journal fault exact journal leaf')
                if ref['kind'] == 'journal':
                    require(exception_key not in exceptions or exceptions[exception_key]['reason'] not in
                            ('MissingJournal', 'TruncatedJournal', 'UnknownJournalVersion', 'ExtraJournalField',
                             'DuplicateJournalField', 'WrongJournalBinding', 'BrokenJournalChain', 'EarlierAllocatedOnlyJournal'),
                            'actual journal classification agrees with source fault checkpoint')
                    journal = canonical_journal(data, definitions)
                    obj = next((obj for obj in snapshot['objects'] if
                                obj['selected']['selected_object_id'] == journal['selected_object_id']), None)
                    require(obj is not None and
                            journal['owner_id'] == obj['selected']['owner_id'] and
                            journal['selected_generation'] == obj['selected']['selected_generation'],
                            'canonical raw journal exact object identity')
                    # R is visible disk, not installed I or last synced S. A
                    # completed rename can expose a prepared successor before
                    # sync/readback/install. The three-state checker below must
                    # admit the precise source/IO episode; never substitute live
                    # ObjectEvidence rows for these actual canonical bytes.
                    result[(core, index, journal['selected_object_id'])] = journal
                hashes.setdefault(ref['sha256'], []).append((ref['kind'], data))
            elif kind == 'absent':
                if path.endswith('/selection.json'):
                    key = (core, index, snapshot['purpose'], path)
                    require(key in exceptions and exceptions[key]['reason'] == 'MissingJournal' and
                            exceptions[key]['raw_kind'] == 'corrupt_journal' and not exceptions[key]['allow_empty'],
                            'missing journal exact source fault identity, no fake zero file')
                require(all(observation[name] is None for name in
                            ('mode', 'byte_len', 'sha256', 'raw', 'symlink_target')), 'actual absent no fake empty file')
            elif kind == 'directory':
                require(observation['mode'] is not None and all(observation[name] is None for name in
                            ('sha256', 'raw', 'symlink_target')), 'actual directory no raw-file claim')
                if observation['byte_len'] is not None:
                    integer(observation['byte_len'])
            else:
                key = (core, index, snapshot['purpose'], path)
                require(key in exceptions and exceptions[key]['reason'] == 'SymlinkEntry' and
                        exceptions[key]['raw_kind'] == 'fixture_file' and not exceptions[key]['allow_empty'],
                        'symlink observation only exact source fault/path')
                require(kind == 'symlink' and observation['mode'] is not None
                        and observation['raw'] is None and observation['sha256'] is None
                        and observation['symlink_target'] is not None, 'explicit corrupted symlink observation only')
        for obj in snapshot['objects']:
            blob_hash = obj['blob_hash']
            if blob_hash in hashes:
                require(any(kind == 'cas_blob' and digest(data) == obj['selected']['content_hash']
                            and len(data) == obj['selected']['byte_len'] for kind, data in hashes[blob_hash]),
                        'actual selected CAS body matches typed selection')
            # Faulted snapshots may intentionally lack or corrupt a journal,
            # manifest or body. Required positive lineage checks are selected by
            # the trusted source relation inventory, never silently inferred.
    return result



JOURNAL_POINTS = ('JournalWrite', 'JournalSync', 'JournalRename',
                  'JournalDirectorySync', 'JournalReadback')


def journal_io_episodes(events, object_id):
    """Delimit actual sole-object publications from complete ordered IO data.

    No wall-clock assumption, IO action or sampled success is introduced. A
    failed/native-simulated stage can stop an episode; incomplete actual IO is
    retained and cannot prove that stage completed.
    """
    require(type(events) is list and len(events) <= 65536, 'journal complete IO bound')
    episodes = []
    current = None
    previous = 0
    for event in events:
        require(event['sequence'] > previous, 'full IO sequence order')
        previous = event['sequence']
        if event['selected_object_id'] != object_id or event['point'] not in JOURNAL_POINTS:
            continue
        point, outcome = event['point'], event['outcome']
        identity = (event['artifact_hash'], event['operation_id'], event['writer_id'])
        hex_bytes(identity[0], 32, 32)
        rank = JOURNAL_POINTS.index(point)
        if point == 'JournalWrite' and outcome in ('Began', 'SimulatedFailure'):
            require(current is None or current['terminal'] or current['pending'] is None,
                    'no overlapping same-object native journal write')
            require(len(episodes) < 16384, 'finite publication episodes before growth')
            current = {'identity': identity, 'completed': [], 'pending': None,
                       'terminal': False, 'failure': None,
                       'first_sequence': event['sequence'], 'last_sequence': event['sequence']}
            episodes.append(current)
        require(current is not None and current['identity'] == identity and not current['terminal'],
                'journal event exact current episode identity')
        current['last_sequence'] = event['sequence']
        require(rank == len(current['completed']), 'native journal stage order no skip')
        if outcome == 'Began':
            require(current['pending'] is None, 'single actual pending native IO stage')
            current['pending'] = point
        elif outcome == 'Completed':
            require(current['pending'] == point, 'completed IO requires actual matching begin')
            current['pending'] = None
            current['completed'].append(point)
            current['terminal'] = point == 'JournalReadback'
        elif outcome == 'ActualFailure':
            require(current['pending'] == point, 'native failure requires actual matching begin')
            current['pending'] = None
            current['failure'] = (point, outcome)
            current['terminal'] = True
        else:
            require(outcome == 'SimulatedFailure' and current['pending'] is None,
                    'simulation must not claim native IO begin/completion')
            current['failure'] = (point, outcome)
            current['terminal'] = True
    return episodes


def exact_journal_supplier(cache, expected_hash, sequence, object_id):
    """cache is built only from admitted complete actual canonical file copies."""
    require(type(cache) is dict and len(cache) <= 192,
            'bounded actual full-journal suppliers per scenario')
    require(expected_hash in cache, 'missing actual full canonical journal supplier')
    journal = cache[expected_hash]
    require(journal['transition_sequence'] == sequence and
            journal['selected_object_id'] == object_id, 'full supplier exact identity')
    return journal


def rows_immutable_join(earlier, later):
    require(earlier['allocation'] == later['allocation'] and
            earlier['allocated_service_epoch'] == later['allocated_service_epoch'],
            'operation immutable allocation and allocated original epoch')
    for field in ('binding', 'submitted_service_epoch', 'permit', 'receipt', 'successor'):
        if earlier[field] is not None:
            require(later[field] == earlier[field], 'non-null original tuple cannot change: ' + field)
    if earlier['closure'] != 'none':
        require(later['closure'] == earlier['closure'], 'first original closure remains sticky')


def live_rows_validate(installed, live_object, fence_object, source_bindings, allocation_retirements, carried_rows):
    """Installed journal plus actual runtime overlays; never a durable claim.

    source_bindings are complete already-validated actual source-freeze capture
    bindings, not records manufactured from ObjectEvidence. The future final
    relation caller must supply only those real admitted capture joins.
    """
    require(live_object['selected'] == installed['current'] and
            live_object['service_epoch'] == installed['service_epoch'],
            'live selected identity belongs to actual installed journal I')
    before = {row['allocation']['operation_id']: row for row in installed['operations']}
    after = {row['allocation']['operation_id']: row for row in live_object['operations']}
    require(len(after) == len(live_object['operations']) <= 16 and set(before) <= set(after),
            'live operation identity bound/no erased installed rows')
    closures = {row['allocation']['operation_id']: row for row in fence_object['original_closures']}
    runtime_permits = {row['binding']['allocation']['operation_id']: row
                       for row in fence_object['issued_runtime_permits']}
    durable_rows = {row['allocation']['operation_id']: row
                    for row in fence_object['known_durable_operations']}
    require(type(source_bindings) is list and len(source_bindings) <= 1024,
            'bounded actual lineage source-freeze bindings')
    for operation, row in after.items():
        if operation in before:
            old = before[operation]
            rows_immutable_join(old, row)
            if old['state'] == 'committed':
                require(row['state'] == 'committed', 'installed Committed never downgraded')
            elif old['state'] == 'noncommit':
                require(row['state'] == 'noncommit', 'Noncommit never readmitted/replayed')
            elif old['state'] == 'permitted':
                require(row['state'] == 'permitted',
                        'same-Core closure cannot cancel issued runtime permit')
        else:
            known = durable_rows.get(operation)
            witness = closures.get(operation)
            require(known is not None or witness is not None,
                    'actual inserted allocation witness required; preallocation pause is insufficient')
            allocation = known['allocation'] if known is not None else witness['allocation']
            require(allocation == row['allocation'], 'new live row matches real retained allocation')
        if row['binding'] is not None:
            old_binding = before.get(operation, {}).get('binding')
            require(old_binding == row['binding'] or row['binding'] in source_bindings,
                    'new runtime submission requires genuine full source-freeze capture')
        if row['permit'] is not None:
            require(runtime_permits.get(operation) == row['permit'], 'actual issued runtime permit retained')
        carried = False
        if operation in before and before[operation]['closure'] != 'none':
            proofs = [proof for proof in carried_rows if proof['allocation'] == row['allocation'] and
                      proof['allocated_service_epoch'] == row['allocated_service_epoch'] and
                      proof['closure'] == before[operation]['closure'] == row['closure']]
            require(len(proofs) <= 1, 'unique validated actual transferred closure')
            if proofs:
                rows_immutable_join(proofs[0], before[operation])
                rows_immutable_join(proofs[0], row)
                carried = True
        allocated_retirement = row in allocation_retirements
        if row['closure'] != 'none' and not carried and not allocated_retirement:
            witness = closures.get(operation)
            require(witness is not None and witness['allocation'] == row['allocation'] and
                    witness['submitted_binding'] == row['binding'] and
                    witness['closure'] == row['closure'], 'actual original sticky closure witness')
    # Use actual fence minima only as retained bounds, never invented precise
    # highwaters for this earlier live observation. Real source/permit witnesses
    # above prove additions before their journal publication completes.
    live = dict(installed)
    live['operations'] = live_object['operations']
    live['current'] = live_object['selected']
    live['operation_high_water'] = max(installed['operation_high_water'],
                                       fence_object['operation_high_water_min'])
    live['writer_high_water'] = max(installed['writer_high_water'],
                                    fence_object['writer_high_water_min'])
    journal_validate(live)


def prepared_journal_validate(installed, target, target_hash, point, live_object,
                              fence_object, events, source_bindings, verified_existing_cas, snapshot_index):
    """One exact stage envelope at this source-admitted actual checkpoint."""
    if target_hash == live_object['journal_hash']:
        require(target == installed, 'same I digest same complete actual canonical bytes')
        return
    require(target['transition_sequence'] == installed['transition_sequence'] + 1 and
            target['prior_journal_hash'] == live_object['journal_hash'] and
            target['service_epoch'] == installed['service_epoch'] and
            target['initial'] == installed['initial'], 'prepared journal exact installed predecessor/epoch/initial')
    oid = installed['selected_object_id']
    episodes = journal_io_episodes(events, oid)
    matches = [episode for episode in episodes if episode['identity'][0] == target_hash and
               point in episode['completed']]
    require(matches, 'ahead state requires actual digest-bound completed stage in this checkpoint IO ledger')
    old = {row['allocation']['operation_id']: row for row in installed['operations']}
    require(set(old) <= {row['allocation']['operation_id'] for row in target['operations']},
            'prepared successor cannot delete installed history')
    for row in target['operations']:
        op = row['allocation']['operation_id']
        if op in old:
            rows_immutable_join(old[op], row)
        else:
            require(any(w['allocation'] == row['allocation'] for w in fence_object['known_durable_operations']) or
                    any(w['allocation'] == row['allocation'] for w in fence_object['original_closures']),
                    'prepared actual allocation witness (BeforeAllocationJournal alone proves nothing)')
        if row['binding'] is not None and old.get(op, {}).get('binding') != row['binding']:
            require(row['binding'] in source_bindings, 'prepared bound source is actual same original freeze')
        if row['permit'] is not None:
            require(row['permit'] in fence_object['issued_runtime_permits'], 'prepared actual immutable runtime permit')
        if row['state'] != 'committed' or old.get(op, {}).get('state') == 'committed':
            continue
        permit = row['permit']
        require(permit is not None and row['binding'] in source_bindings, 'prepared Committed actual source/permit')
        completed = [event for event in events if event['selected_object_id'] == oid and
                     event['operation_id'] == op and event['writer_id'] == permit['writer_id'] and
                     event['point'] == 'CandidateReadback' and event['outcome'] == 'Completed' and
                     event['artifact_hash'] == row['binding']['source']['content_hash']]
        direct = any(event['sequence'] < episode['first_sequence'] and
                     episode['identity'][1:] == (op, permit['writer_id']) for event in completed for episode in matches)
        reused = any(prior['observation_index'] < snapshot_index and
                     prior['selected']['owner_id'] == row['successor']['owner_id'] and
                     prior['selected']['selected_object_id'] == oid and
                     prior['selected']['selected_generation'] == row['successor']['selected_generation'] and
                     all(prior['selected'][field] == row['successor'][field]
                         for field in ('content_hash', 'byte_len', 'manifest_hash')) and
                     any(prior['io_sequence'] < episode['first_sequence'] and
                         episode['identity'][1:] == (op, permit['writer_id']) for episode in matches)
                     for prior in verified_existing_cas)
        require(direct or reused, 'ahead Committed full atomic receipt/source plus actual prior candidate or root-verified existing CAS')


def three_journal_views_validate(live_object, visible, fence_object, cache, events,
                                source_bindings, verified_existing_cas, snapshot_index, allocation_retirements, carried_rows):
    """I/S/R at an actual joined checkpoint; no all-operations equality claim."""
    oid = live_object['selected']['selected_object_id']
    require((fence_object['selected_object_id'], fence_object['owner_id'], fence_object['selected_generation']) ==
            (oid, live_object['selected']['owner_id'], live_object['selected']['selected_generation']),
            'actual same-object joined fence')
    installed = exact_journal_supplier(cache, live_object['journal_hash'], live_object['journal_sequence'], oid)
    synced = exact_journal_supplier(cache, fence_object['durable_journal_hash'], fence_object['durable_journal_sequence'], oid)
    require(synced['operations'] == fence_object['known_durable_operations'] and
            synced['service_epoch'] == fence_object['service_epoch_at_fence'] == installed['service_epoch'],
            'S actual full bytes/retained durable operations/original epoch')
    live_rows_validate(installed, live_object, fence_object, source_bindings, allocation_retirements, carried_rows)
    prepared_journal_validate(installed, synced, fence_object['durable_journal_hash'], 'JournalDirectorySync',
                              live_object, fence_object, events, source_bindings, verified_existing_cas, snapshot_index)
    if visible is not None:
        hashes = [h for h, journal in cache.items() if journal == visible]
        require(len(hashes) == 1, 'R exact actual admitted canonical byte digest')
        prepared_journal_validate(installed, visible, hashes[0], 'JournalRename', live_object, fence_object,
                                  events, source_bindings, verified_existing_cas, snapshot_index)
    # Readback-failed S/R Committed keeps authentic live I Permitted/current old.
    # Rename-unsynced R Committed keeps S/I old. Neither promotes public Saved.
    return {'installed': installed, 'synced': synced, 'visible': visible}


def fault_journal_bytes(before, observation, reason, raw_bytes, definitions):
    """Exact source-owned storage mutation of an earlier complete copied file.

    This constructs an expectation from actual raw JSON only, never synthesizes
    a supplier or journal bytes from ObjectEvidence/fence diagnostic rows.
    """
    original = raw_bytes[before['raw']['file']]
    baseline = bounded_json(original, 131072)
    if reason == 'MissingJournal':
        require(observation['entry_kind'] == 'absent' and observation['raw'] is None,
                'actual missing journal observation has no invented bytes')
        return
    require(observation['entry_kind'] == 'regular' and observation['raw'] is not None and
            observation['raw']['kind'] == 'corrupt_journal', 'actual faulted journal regular copied bytes')
    data = raw_bytes[observation['raw']['file']]
    if reason == 'TruncatedJournal':
        require(data == original[:len(original) // 2] and len(data) > 0,
                'actual current fault writes original half, nonempty')
    elif reason == 'DuplicateJournalField':
        require(data == b'{"schema_version":1,' + original[1:], 'actual duplicate top-level schema field mutation')
    elif reason in ('UnknownJournalVersion', 'ExtraJournalField', 'WrongJournalBinding', 'BrokenJournalChain'):
        expected = dict(baseline)
        if reason == 'UnknownJournalVersion':
            expected['schema_version'] = 999
        elif reason == 'ExtraJournalField':
            expected['unexpected'] = True
        elif reason == 'WrongJournalBinding':
            expected['owner_id'] = baseline['current']['owner_id'] + 1
        else:
            expected['prior_journal_hash'] = [0] * 32
            require(baseline['prior_journal_hash'] != [0] * 32, 'broken chain actual changed predecessor')
        require(bounded_json(data, 131072) == expected, 'exact source-declared single journal fault, all other data retained')
    else:
        require(reason == 'EarlierAllocatedOnlyJournal', 'closed source journal fault reason')
        earlier = bounded_json(data, 131072)
        expected = dict(baseline)
        expected['current'] = baseline['initial']
        expected['operations'] = []
        for row in baseline['operations']:
            changed = dict(row)
            changed.update(state='allocated', closure='none', binding=None, submitted_service_epoch=None,
                           permit=None, receipt=None, successor=None)
            expected['operations'].append(changed)
        require(earlier == expected and earlier != baseline, 'coherent allocated-only downgrade exact retained mutation')
        canonical_journal(data, definitions)


def checkpoint_views_validate(c, core, fence):
    """All live/joined/fault checkpoints with exact source chronology.

    A final fence may supply retained immutable runtime witness bounds for an
    earlier live snapshot, but its latest S is never asserted simultaneous there.
    Corrupt/missing R requires the exact source-owned fault/path and a preceding
    full canonical journal copy. Failed reopen phase alone changes no rule.
    """
    admitted = {(row['epoch_index'], row['observation_index']): row for row in c['source']['store_snapshots']}
    current = [snap for snap in c['snapshots'] if snap['binding']['epoch_index'] == core]
    cache = c['suppliers'][core]
    checked = []
    latest_events = current[-1]['io_events'] if current else []
    for snap in current:
        index = snap['observation_index']; stage = admitted[(core, index)]
        require(stage['purpose'] == snap['purpose'], 'actual source checkpoint exact purpose')
        for obj in snap['objects']:
            oid = obj['selected']['selected_object_id']
            witnesses = [row for row in fence['base']['objects'] if row['selected_object_id'] == oid]
            require(len(witnesses) == 1, 'actual retained same-Core object witness')
            witness = witnesses[0]
            installed = exact_journal_supplier(cache, obj['journal_hash'], obj['journal_sequence'], oid)
            source_bindings = [binding for current_core, cap, _, binding in c['captures'] if
                               current_core == core and cap['kind'] == 'DraftSourceFreeze' and cap['object_id'] == oid]
            prior = c['verified_cas'].get(core, [])
            visible = c['visible'].get((core, index, oid))
            live_rows_validate(installed, obj, witness, source_bindings,
                               c['allocation_retirements'].get((core, oid), []),
                               c.get('carried_closure_rows', {}).get((core, oid), []))
            use_synced = stage['durability'] == 'joined_checkpoint' or visible is None
            if use_synced:
                # Final S is appropriate only after the authentic terminal join
                # cut, with no later synchronization of this object on that Core.
                require(snap['ledger']['active_io_workers'] == 0 and
                        latest_events[:len(snap['io_events'])] == snap['io_events'] and
                        not any(event['selected_object_id'] == oid and event['point'] == 'JournalDirectorySync' and
                                event['outcome'] == 'Completed' for event in latest_events[len(snap['io_events']):]),
                        'joined/fault S has no later completed synchronization in same actual Core')
            if visible is None:
                require(stage['durability'] == 'negative_disk_fault', 'positive checkpoint must retain full R')
                # Locate this object namespace from an earlier actual canonical
                # journal, never from a guessed folder convention or object ID.
                earlier = []
                for before in current:
                    if before['observation_index'] >= index:
                        continue
                    for file in before['files']:
                        ref = file['raw']
                        if ref is not None and ref['kind'] == 'journal':
                            journal = canonical_journal(c['raw'][ref['file']], c['definitions'])
                            if journal['selected_object_id'] == oid:
                                earlier.append((before['observation_index'], file))
                paths = {file['relative_path'] for _, file in earlier}
                require(len(paths) == 1, 'negative journal exact earlier actual object path/full byte supplier')
                path = next(iter(paths))
                exceptions = [row for row in c['source']['file_exceptions'] if
                              (row['epoch_index'], row['observation_index'], row['snapshot_purpose'], row['relative_path']) ==
                              (core, index, snap['purpose'], path)]
                require(len(exceptions) == 1 and exceptions[0]['raw_kind'] == 'corrupt_journal' and
                        exceptions[0]['reason'] in ('MissingJournal', 'TruncatedJournal', 'UnknownJournalVersion',
                        'ExtraJournalField', 'DuplicateJournalField', 'WrongJournalBinding', 'BrokenJournalChain',
                        'EarlierAllocatedOnlyJournal'), 'actual source exact journal fault, not generic failed-reopen phase')
                before_index, before_file = max(earlier, key=lambda value: value[0])
                observed = [file for file in snap['files'] if file['relative_path'] == path]
                require(len(observed) == 1, 'actual exact faulted journal path observation')
                fault_journal_bytes(before_file, observed[0], exceptions[0]['reason'], c['raw'], c['definitions'])
                # I/S still full native bytes; fault preserves visible broken R
                # without admitting it as durable or reconstructing rows.
                three_journal_views_validate(obj, None, witness, cache, snap['io_events'], source_bindings, prior, index,
                                             c['allocation_retirements'].get((core, oid), []),
                               c.get('carried_closure_rows', {}).get((core, oid), []))
            elif stage['durability'] == 'joined_checkpoint':
                three_journal_views_validate(obj, visible, witness, cache, snap['io_events'], source_bindings, prior, index,
                                             c['allocation_retirements'].get((core, oid), []),
                               c.get('carried_closure_rows', {}).get((core, oid), []))
            else:
                # live_checkpoint and canonical negative_disk_fault (MAX or a
                # nonjournal fault) validate I/live/R with this snapshot IO only.
                h = [h for h, journal in cache.items() if journal == visible]
                require(len(h) == 1, 'actual live R canonical byte identity')
                prepared_journal_validate(installed, visible, h[0], 'JournalRename', obj, witness,
                                          snap['io_events'], source_bindings, prior, index)
            checked.append((core, index, oid))
    return checked


def cleanup_validate(cleanup, joins, scope):
    # Records describe actual trusted fixture actions, not authority to touch a
    # recorded path/PID. This reader neither reopens the removed owner root nor
    # signals/reaps a service producer by numeric artifact data.
    callers = cleanup['external_callers']
    require(type(callers) is list and len(callers) <= 128, 'actual external caller join bound')
    for index, caller in enumerate(callers):
        require(integer(caller['observation_index'], 0xffffffff) == index and caller['joined'] is True and caller['service_producer_claim'] is False,
                'actual distinct external caller joins are not service roster authority')
        require(caller['outcome'] in ('Returned', 'Panicked') and type(caller['rust_thread_id']) is str
                and 0 < len(caller['rust_thread_id'].encode('utf8')) <= 64, 'actual bounded external join metadata')
    if joins is not None:
        require(joins['held_after'] == 0 and joins['external_callers'] == callers,
                'same actual terminal join/cleanup inventory')
    if scope == 'volatile-control':
        require(cleanup['root_identity'] is None and cleanup['store_fence'] is None
                and cleanup['native_process_cleanup_claim'] is False
                and cleanup['consumer_holders_dropped'] is True, 'volatile cleanup no fake native owner')
        # This actual old-host ledger is recorded intact. The scope promises
        # dropped consumer holders, not a new volatile service-fence API or
        # invented zero counts for controller-retained immutable objects.
        return
    root = cleanup['root_identity']
    require(type(root) is dict and type(root['path']) is str and 0 < len(root['path'].encode('utf8')) <= 4096
            and root['path'].startswith('/') and root['inode'] > 0,
            'actual fixture root identity data only')
    require(cleanup['owner_relative'] == 'owner' and cleanup['root_close_result'] == 'Ok'
            and cleanup['root_absence_result'] == 'NotFound', 'actual own root close/absence results')
    if scope == 'native-read-control':
        require(cleanup['held_after_join'] == 0 and cleanup['store_fence'] is None,
                'Read control no invented Save fence')
        ids = set()
        for row in cleanup['actual_join_records']:
            producer = integer(row['id'], minimum=1)
            require(producer not in ids and row['outcome'] == 1, 'actual Read returned joins')
            ids.add(producer)
    else:
        require(scope in ('composition', 'direct-native') and cleanup['service_held_after_join'] == 0
                and cleanup['service_held_before_cleanup'] <= 64
                and cleanup['all_recording_reservations_settled'] is True,
                'actual Save terminal producers and recording reservations settled')
        require(joins is not None and 0 < len(cleanup['fences']) <= 4,
                'actual Save cleanup requires owned fences')
        join_fence_validate(joins['native_join_observations'], cleanup['fences'])
        namespace = None
        fence_ids = set()
        for fence in cleanup['fences']:
            base = fence['base']
            if namespace is None:
                namespace = base['namespace_id']
            require(namespace == base['namespace_id'] and base['fence_id'] not in fence_ids,
                    'same actual exclusive fixture namespace and one-shot fence generations')
            integer(namespace, minimum=1)
            integer(base['fence_id'], minimum=1)
            fence_ids.add(base['fence_id'])



def base_fence_digest(base, definitions):
    """Reverse the frozen hex projection, hashing native declared Serde order.

    This hashes only actual base FenceSnapshot, never the NativeSave wrapper or
    recorder JSON. The separately admitted DTO map is the declaration-order
    contract; it cannot be selected by the evidence itself.
    """
    validate_shape(base, definitions['FenceSnapshot'], definitions)
    def native(value, spec):
        if '$ref' in spec:
            name = spec['$ref'][8:]
            if name == 'Hash32':
                return list(hex_bytes(value, 32, 32))
            return native(value, definitions[name])
        if 'anyOf' in spec:
            if value is None:
                return None
            parts = [part for part in spec['anyOf'] if part.get('type') != 'null']
            require(len(parts) == 1, 'native fence closed optional')
            return native(value, parts[0])
        if spec.get('type') == 'object':
            return {key: native(value[key], part) for key, part in spec['properties'].items()}
        if spec.get('type') == 'array':
            return [native(item, spec['items']) for item in value]
        return value
    data = json.dumps(native(base, definitions['FenceSnapshot']), separators=(',', ':'), ensure_ascii=False).encode('utf8')
    require(len(data) <= 1048576, 'actual bounded base native fence serialization')
    return digest(data)


def native_selected_data(selected):
    """Exact declared SelectedArtifactV0 native Serde value, not authorization."""
    selected_validate(selected)
    names = ('schema_version', 'owner_id', 'selected_object_id', 'selected_generation',
             'revision', 'content_hash', 'byte_len', 'manifest_hash')
    return {name: list(hex_bytes(selected[name], 32, 32)) if name in ('content_hash', 'manifest_hash')
            else selected[name] for name in names}


def verified_manifest_bytes(mode, body, manifest_raw, root_verifier):
    """Root-injected actual Rust verifier; artifacts never select key/callback.

    Root must independently freeze verifier source/binary/review/public-key pins
    and retain each bounded actual child result/birth/reap. This pure reader does
    not compile/run that child, signal a PID, or treat its JSON as authority.
    A caller-supplied service/artifact callback is outside the trusted-root ABI.
    """
    require(mode in ('native-task', 'native-read') and type(body) is bytes and len(body) <= 4096 and
            type(manifest_raw) is bytes and 0 < len(manifest_raw) <= 2048, 'complete verifier input byte bounds')
    require(type(root_verifier) is dict and set(root_verifier) ==
            {'callback', 'source_sha256', 'binary_sha256', 'review_sha256', 'public_key'} and
            callable(root_verifier['callback']), 'actual root-frozen verifier seam required')
    for name in ('source_sha256', 'binary_sha256', 'review_sha256', 'public_key'):
        hex_bytes(root_verifier[name], 32, 32)
    key = 'native-task-v0-fixture' if mode == 'native-task' else 'native-read-fixture-v0'
    response = root_verifier['callback'](mode, body, manifest_raw)
    # No arbitrary exception is caught as success. Callback returns actual raw
    # bounded diagnostic bytes after the root verifies its child exit/reap.
    diagnostic = bounded_json(response, 1024)
    exact_keys(diagnostic, ('schema_version', 'kind', 'profile_mode', 'key_id', 'public_key',
                           'manifest_byte_len', 'manifest_sha256', 'body_byte_len', 'body_sha256',
                           'signing_policy_sha256'))
    public = hex_bytes(root_verifier['public_key'], 32, 32)
    policy = [1, 'RequireSignature', 'editor-selected-private', 'dev-host-editor', key, list(public)]
    policy_sha = digest(json.dumps(policy, separators=(',', ':'), ensure_ascii=False).encode('utf8'))
    require(type(diagnostic['schema_version']) is int and diagnostic['schema_version'] == 1 and
            diagnostic['kind'] == 'fixture_manifest' and diagnostic['profile_mode'] == mode and
            diagnostic['key_id'] == key and diagnostic['public_key'] == root_verifier['public_key'] and
            type(diagnostic['manifest_byte_len']) is int and diagnostic['manifest_byte_len'] == len(manifest_raw) and
            type(diagnostic['body_byte_len']) is int and diagnostic['body_byte_len'] == len(body) and
            diagnostic['manifest_sha256'] == digest(manifest_raw) and diagnostic['body_sha256'] == digest(body) and
            diagnostic['signing_policy_sha256'] == policy_sha, 'actual verifier closed diagnostic equals copied bytes/fixed trust')
    manifest = bounded_json(manifest_raw, 2048)
    names = ('schema_version', 'content_id', 'size_bytes', 'kind', 'channels', 'signatures')
    exact_keys(manifest, names)
    require(type(manifest['schema_version']) is int and manifest['schema_version'] == 1 and
            manifest['content_id'] == 'sha256:' + digest(body) and
            type(manifest['size_bytes']) is int and manifest['size_bytes'] == len(body) and
            manifest['kind'] == 'editor-selected-private' and manifest['channels'] == ['dev-host-editor'] and
            type(manifest['signatures']) is list and len(manifest['signatures']) == 1 and
            type(manifest['signatures'][0]) is str and len(manifest['signatures'][0].encode('utf8')) <= 1024,
            'verified full typed Manifest content/size/kind/channel')
    signature = bounded_json(manifest['signatures'][0].encode('utf8'), 1024)
    exact_keys(signature, ('algorithm', 'signature_data', 'key_id'))
    require(signature['algorithm'] == 'ed25519' and signature['key_id'] == key and
            type(signature['signature_data']) is str and len(signature['signature_data'].encode('utf8')) <= 128,
            'actual verified fixed-key signature envelope')
    # Storage hashes the typed compact Manifest, which need not equal its raw
    # whitespace/order hash. Preserve the exact signed string; do not clear it.
    typed = {name: manifest[name] for name in names}
    typed_hash = digest(json.dumps(typed, separators=(',', ':'), ensure_ascii=False).encode('utf8'))
    return typed_hash, policy_sha, key


def verified_cas_observations(c, root_verifier):
    """Full actual body/manifest/owner at exact source-admitted snapshot/path.

    Returns per-Core provenance-bearing candidates. A later copied identical
    body cannot prove prior existence for an earlier publication episode.
    """
    scope = c['source']['scope']
    if scope == 'volatile-control':
        return {}, None
    mode = 'native-read' if scope == 'native-read-control' else 'native-task'
    require(scope in ('composition', 'direct-native', 'native-read-control'), 'trusted source signing mode')
    result, initials, memo = {}, {}, {}
    expected_initials = {row['object_id']: row for row in c['source']['initial_objects']}
    for snap in c['snapshots']:
        core, index = snap['binding']['epoch_index'], snap['observation_index']
        files = {row['relative_path']: row for row in snap['files']}
        for observation in snap['files']:
            ref = observation['raw']
            if ref is None or ref['kind'] != 'journal':
                continue
            journal = canonical_journal(c['raw'][ref['file']], c['definitions'])
            initial = journal['initial']; oid = journal['selected_object_id']
            require(oid in expected_initials, 'actual canonical journal in trusted initial object inventory')
            expected = expected_initials[oid]
            require((initial['owner_id'], oid, initial['selected_generation'], initial['revision'],
                     initial['content_hash'], initial['byte_len']) ==
                    (expected['owner_id'], expected['object_id'], expected['generation'], expected['revision'],
                     expected['body_sha256'], expected['body_len']), 'actual journal original initial/source identity')
            require(oid not in initials or initials[oid] == initial, 'all Core epochs retain exact initial Selected tuple')
            # Merely knowing the hash is insufficient. Only complete actual
            # signed bytes can supply an existing CAS candidate.
            parent = observation['relative_path'].rsplit('/', 1)[0] + '/cas'
            selecteds = [initial, journal['current']] + [row['successor'] for row in journal['operations']
                                                       if row['successor'] is not None]
            for selected in selecteds:
                h = selected['content_hash']
                paths = [parent + '/' + h + suffix for suffix in ('.blob', '.manifest.json', '.ownership.json')]
                if not all(path in files and files[path]['entry_kind'] == 'regular' and
                           files[path]['raw'] is not None for path in paths):
                    continue
                rows = [files[path] for path in paths]
                if [row['raw']['kind'] for row in rows] != ['cas_blob', 'fixture_file', 'fixture_file']:
                    continue
                body, manifest, owner_raw = [c['raw'][row['raw']['file']] for row in rows]
                # Exact source-declared faults intentionally retain invalid
                # bytes. They cannot become a positive existing-CAS supplier.
                exceptions = {(row['epoch_index'], row['observation_index'], row['relative_path']): row
                              for row in c['source']['file_exceptions']}
                if any((core, index, path) in exceptions and
                       exceptions[(core, index, path)]['reason'] not in ('EmptyAsciiObject', 'SourceDeclaredEmptyFixture')
                       for path in paths):
                    continue
                require(len(body) == selected['byte_len'] and digest(body) == h,
                        'full actual CAS candidate body equals canonical journal selection')
                memo_key = (mode, digest(body), digest(manifest))
                if memo_key not in memo:
                    require(len(memo) < 153, 'bounded distinct copied-byte signature validations')
                    memo[memo_key] = verified_manifest_bytes(mode, body, manifest, root_verifier)
                typed_hash, policy_sha, key = memo[memo_key]
                require(typed_hash == selected['manifest_hash'] and policy_sha == journal['signing_policy_hash'],
                        'actual signed typed Manifest and fixed policy equal journal')
                owner = bounded_json(owner_raw, 512)
                exact_keys(owner, ('content_id', 'domain_id', 'is_global', 'ingested_at'))
                require(owner['content_id'] == 'sha256:' + h and type(owner['domain_id']) is int and
                        owner['domain_id'] == selected['owner_id'] and owner['is_global'] is False and
                        type(owner['ingested_at']) is int and 0 <= owner['ingested_at'] <= U64_MAX,
                        'full actual owner sidecar u64 ingestion/domain')
                if selected == initial:
                    initials[oid] = initial
                obj = next(obj for obj in snap['objects'] if obj['selected']['selected_object_id'] == oid)
                if obj['selected'] == selected:
                    require((obj['blob_hash'], obj['manifest_hash'], obj['ownership_hash']) ==
                            (digest(body), typed_hash, digest(owner_raw)), 'actual installed selected body/typed-manifest/owner diagnostic digests')
                entries = result.setdefault(core, [])
                require(len(entries) < 9792, 'actual signed CAS observations bounded64snapshots*3objects*51selections')
                entries.append(dict(selected=selected, observation_index=index, purpose=snap['purpose'],
                                    journal_sha256=ref['sha256'], relative_path=paths[0],
                                    io_sequence=max((event['sequence'] for event in snap['io_events']), default=0)))
    require(set(initials) == set(expected_initials), 'every actual initial Selected has complete root-verified signed bytes')
    native_initial = {str(oid): native_selected_data(initials[oid]) for oid in sorted(initials)}
    key = 'native-task-v0-fixture' if mode == 'native-task' else 'native-read-fixture-v0'
    profile_tuple = [native_initial, 1, key, list(bytes.fromhex(digest(bytes([0x53]) * 32)))]
    profile_hash = digest(json.dumps(profile_tuple, separators=(',', ':'), ensure_ascii=False).encode('utf8'))
    for fence in c['records'].get('cleanup.json', {}).get('fences', []):
        require(fence['base']['profile_hash'] == profile_hash, 'actual owned fence fixed original verified profile tuple')
    return result, profile_hash


def relation_context(records, source, definitions, raw_bytes, font, root_verifier=None):
    summary, snapshots = scenario_records_validate(records, definitions, source,
        records['summary.json']['binding']['case'], records['summary.json']['binding']['run_nonce'])
    objects, traces = {}, {}
    for snap in snapshots:
        core = snap['binding']['epoch_index']
        if core in traces:
            require(snap['exchanges'][:len(traces[core])] == traces[core],
                    'complete cumulative Core exchange immutable prefix')
        objects[core], traces[core] = snap['objects'], snap['exchanges']
    exchanges = []
    for core, rows in traces.items():
        for row in rows:
            validate_actor(row['actor'])
            wire = hex_bytes(row['request_wire'], 88, 88)
            q, r = decode_wire88(wire), None
            require(q['protocol'] == 368 and q['message'] in (1, 3, 5, 7, 9), 'native actual request class')
            actor_wire(row['actor'], q, instance=q['message'] == 1)
            if row['actual_reply_wire'] is not None:
                q, r = exchange_pair(wire, hex_bytes(row['actual_reply_wire'], 88, 88), native_error_redaction=True)
            exchanges.append((core, row, q, r))
    visible = store_snapshots_validate(snapshots, definitions, raw_bytes, source['file_exceptions'])
    suppliers = {}
    for snap in snapshots:
        core = snap['binding']['epoch_index']
        cache = suppliers.setdefault(core, {})
        for file in snap['files']:
            if file['raw'] is None or file['raw']['kind'] != 'journal':
                continue
            ref = file['raw']; journal = canonical_journal(raw_bytes[ref['file']], definitions)
            require(ref['sha256'] not in cache or cache[ref['sha256']] == journal,
                    'actual canonical journal hash has one byte identity')
            require(ref['sha256'] in cache or len(cache) < 192, 'full actual journal suppliers bounded')
            cache[ref['sha256']] = journal
    initial = {row['object_id']: row for row in source['initial_objects']}
    batches = records.get('artifacts.json', {}).get('batches', [])
    captures = captures_validate(batches, objects, raw_bytes, initial) if batches else []
    tickets = [(batch['epoch_index'], row) for batch in records.get('joins.json', {}).get(
        'fence_original_ticket_batches', []) for row in batch['original_tickets']]
    context = dict(records=records, source=source, summary=summary, snapshots=snapshots,
                   objects=objects, exchanges=exchanges, captures=captures, tickets=tickets,
                   raw=raw_bytes, font=font, definitions=definitions, visible=visible, suppliers=suppliers)
    context['verified_cas'], context['verified_profile_hash'] = verified_cas_observations(context, root_verifier)
    return context


def captured_exchange(context, core, capture):
    require(any(current == core and row['actor'] == capture['actor'] and
                row['selected_object_id'] == capture['object_id'] and
                row['request_wire'] == capture['capture_request'] and
                row['actual_reply_wire'] == capture['capture_reply']
                for current, row, _, _ in context['exchanges']), 'copy exact public invocation/reply')


def original_commit(context, capture):
    found = [ticket for _, ticket in context['tickets'] if ticket['kind'] == 'Commit' and
             ticket['original_request'] == capture['original_request'] and ticket['actor'] == capture['actor'] and
             ticket['object_id'] == capture['object_id'] and ticket['intent_id'] == capture['intent_id'] and
             ticket['original_epoch'] == capture['original_backend_epoch']]
    require(found and all(ticket['binding'] == found[0]['binding'] for ticket in found) and
            found[0]['binding'] is not None and found[0]['commit_dispatched'],
            'actual retained original Commit/source owner across Core transfer')
    require(found[0]['binding']['allocation']['operation_id'] == capture['operation_id'], 'original operation')
    return found[0]


def frame_original(context, frame):
    chrome = frame['chrome']
    if chrome is None or chrome['record248'] is None:
        return None
    data = hex_bytes(chrome['record248'], 248, 248)
    if u32(data, 0) != 2 or u32(data, 8) in (0, 4):
        return None
    operation, state = u64(data, 88), u32(data, 8)
    matches = []
    for _, capture, _, binding in context['captures']:
        if capture['operation_id'] != operation or capture['actor'] != chrome['original_actor']:
            continue
        if state == 1 and capture['kind'] == 'DraftSourceFreeze':
            matches.append((binding, capture['original_backend_epoch'], None))
        elif state in (2, 3) and capture['kind'] == 'ReceiptCopy':
            raw = context['raw'][capture['raw']['file']]
            if digest(raw) == data[208:240].hex():
                matches.append((binding, capture['original_backend_epoch'], raw))
    require(matches and all(value == matches[0] for value in matches),
            'protected248 exact real bound source or legitimate176 receipt')
    return matches[0]


def relation_app_save_chain(c):
    require(c['source']['scope'] == 'composition', 'actual app-chain scope')
    found = []
    for core, capture, receipt, binding in c['captures']:
        if capture['kind'] != 'ReceiptCopy' or receipt['outcome'] != 'committed':
            continue
        captured_exchange(c, core, capture)
        original_commit(c, capture)
        sources = [(current, row) for current, row, _, value in c['captures'] if
                   row['kind'] == 'DraftSourceFreeze' and row['original_request'] == capture['original_request'] and
                   row['actor'] == capture['actor'] and row['intent_id'] == capture['intent_id'] and value == binding]
        require(len(sources) == 1, 'app actual source frozen once, not separate direct chain')
        a = binding['allocation']
        require(sum(current == sources[0][0] and row['actor'] == capture['actor'] and
                    row['selected_object_id'] == capture['object_id'] and q['message'] == 3 and
                    r is not None and r['status'] == 0 and u64(r['payload'], 8) == a['operation_id'] and
                    u64(q['payload'], 24) == a['expected_revision']
                    for current, row, q, r in c['exchanges']) == 1, 'app actual allocation-to-Commit join')
        frames = [frame for frame in c['summary']['frames'] if frame['draft'] is not None and
                  frame['chrome'] is not None and frame['chrome']['original_actor'] == capture['actor'] and
                  frame['chrome']['label'] == 'Saved' and frame['chrome']['record248'] is not None and
                  u64(hex_bytes(frame['chrome']['record248'], 248, 248), 88) == capture['operation_id']]
        require(frames, 'app actual current protected Saved publication')
        for frame in frames:
            original = frame_original(c, frame)
            require(original == (binding, capture['original_backend_epoch'], c['raw'][capture['raw']['file']]),
                    'app Saved exact original binding epoch and receipt')
            frame_validate(frame, c['raw'], c['font'], original)
        found.append((core, capture['operation_id']))
    require(found, 'required committed app64/source/176/frame chain missing')
    return found


def allocation_retirement_validate(c, core, obj, row, fence, actual_joins):
    a = row['allocation']; oid = a['selected_object_id']; op = a['operation_id']
    require(row['state'] == 'noncommit' and row['closure'] in ('service_retired', 'deadline') and
            all(row[field] is None for field in ('binding', 'submitted_service_epoch', 'permit', 'receipt', 'successor')) and
            row['allocated_service_epoch'] == obj['service_epoch'],
            'finite allocation-only whole-Core retirement tuple')
    witnesses = [ticket for ticket in fence['original_tickets'] if ticket['kind'] == 'Allocate' and
                 ticket['allocation'] == a and ticket['actor'] == a['actor'] and ticket['object_id'] == oid and
                 ticket['original_epoch'] == row['allocated_service_epoch'] and ticket['binding'] is None and
                 ticket['closed'] is True and ticket['completion'] == 'JoinedReturned']
    require(len(witnesses) == 1, 'allocation retirement exact genuine completed Allocate ticket')
    ticket = witnesses[0]
    calls = [(ex, q, r) for epoch, ex, q, r in c['exchanges'] if epoch == core and
             ex['request_wire'] == ticket['original_request'] and ex['actor'] == a['actor'] and
             ex['selected_object_id'] == oid and q['message'] == 3 and r is not None and
             r['status'] == 0 and u64(q['payload'], 24) == a['expected_revision'] and
             u64(r['payload'], 8) == op]
    require(len(calls) == 1, 'allocation retirement actual successful original reply')
    if row['closure'] == 'deadline':
        # This source-fixed runtime assertion waits on the genuine intent's
        # Instant deadline, then exercises authenticated source continuation.
        # The recording checks its actual effects; it does not invent a clock.
        require(c['summary']['binding']['case'] == 'ui1_1_store_lost_reply_reconciles_without_mutation_replay' and
                c['source']['scenario'] == 'allocate_loss' and c['source']['source_leg'] == 0 and core == 0 and
                oid == 101 and op == 1 and calls[0][1]['request_id'] == 1600 and
                calls[0][0]['reply_suppressed'] is True and ticket['commit_dispatched'] is False,
                'closed finite source-owned lost Allocate deadline case')
        observed = [(ex, q, r) for epoch, ex, q, r in c['exchanges'] if epoch == core and
                    ex['actor'] == a['actor'] and ex['selected_object_id'] == oid and q['message'] == 9 and
                    q['request_id'] == 1601 and u64(q['payload'], 24) == 1600 and r is not None and
                    r['status'] == 0 and u64(r['payload'], 8) == 1600 and u64(r['payload'], 16) == op]
        controls = [control for control in c['summary']['control_observations'] if
                    control['purpose'] == 'control_main_020' and control['call'] == 'source_for_intent' and
                    control['kind'] == 'unit_result' and control['actual_status'] ==
                    dict(result='Err', status_space='StoreStatus', status=4)]
        require(len(observed) == len(controls) == 1 and
                not any(epoch == core and q['message'] == 5 for epoch, _, q, _ in c['exchanges']) and
                obj['mutation_dispatch_count'] == obj['permit_count'] == obj['transition_count'] == 0,
                'actual retained Allocate observation then stale continuation has no Commit effects')

    dispatches = [join for join in actual_joins.values() if join['kind'] == 'StoreDispatch' and
                  join['origin_id'] == ticket['origin_id'] and join['producer_generation'] == 1 and
                  join['outcome'] == 'Returned']
    require(len(dispatches) == 1, 'allocation retirement genuine returned original dispatcher join')
    prior = [old for journal in c['suppliers'][core].values() if
             journal['selected_object_id'] == oid and journal['service_epoch'] == row['allocated_service_epoch']
             for old in journal['operations'] if old['allocation'] == a and
             old['allocated_service_epoch'] == row['allocated_service_epoch'] and
             old['state'] == 'allocated' and old['closure'] == 'none']
    require(prior, 'allocation retirement actual canonical allocated predecessor')
    require(not any(old['binding'] is not None or old['permit'] is not None or old['receipt'] is not None or
                    old['successor'] is not None for journal in c['suppliers'][core].values() if
                    journal['selected_object_id'] == oid for old in journal['operations'] if old['allocation'] == a),
            'allocation-only retirement has no hidden submitted or permitted supplier')
    validate_fence(fence)
    return row


def relation_reopened_selected_read(c):
    # Validate every reopened object, including profile peers without a selected
    # copy. Historical closures retain their actual earlier owner proof; reopen
    # creates no new original request for these installed operations.
    ordinals = c['source']['core_ordinals']
    fences = c['records']['cleanup.json']['fences']
    require(len(fences) == len(ordinals) <= 4 and ordinals == sorted(ordinals),
            'bounded ordered actual Core/fence lineage')
    by_core = dict(zip(ordinals, fences))
    actual_joins = join_fence_validate(c['records']['joins.json']['native_join_observations'], fences)
    checked, proved = set(), {}
    c['carried_closure_rows'] = {}
    for core in ordinals:
        fence = by_core[core]
        for obj in c['objects'][core]:
            oid = obj['selected']['selected_object_id']
            current = [row for row in fence['base']['objects'] if row['selected_object_id'] == oid]
            require(len(current) == 1, 'unique actual same-Core object fence')
            current = current[0]
            w = obj['reopen_witness']
            require((w is None) == (core == ordinals[0]),
                    'every actual successor Core object retains its reopen witness')
            require(all(peer['reopen_witness'] == w and peer['service_epoch'] == obj['service_epoch']
                        for snap in c['snapshots'] if snap['binding']['epoch_index'] == core
                        for peer in snap['objects'] if peer['selected']['selected_object_id'] == oid),
                    'same-Core reopen witness immutable across all actual checkpoints')
            carried = {}
            if w is not None:
                require(w['actual_service_epoch'] == w['prior_service_epoch'] + 1 == obj['service_epoch'] and
                        w['selected_object_id'] == oid and
                        w['selected_generation'] == obj['selected']['selected_generation'],
                        'actual fresh authorized reopened object/epoch')
                predecessors = [(old_core, old_fence) for old_core, old_fence in by_core.items() if
                                old_core < core and old_fence['base']['fence_id'] == w['fence_id'] and
                                base_fence_digest(old_fence['base'], c['definitions']) == w['fence_snapshot_hash']]
                require(len(predecessors) == 1, 'reopen bound to one declared earlier native-Serde fence')
                old_core, old_fence = predecessors[0]
                require(old_fence['base']['namespace_id'] == fence['base']['namespace_id'] and
                        old_fence['base']['profile_hash'] == fence['base']['profile_hash'],
                        'reopen preserves actual namespace and fixed profile')
                old_objects = [row for row in old_fence['base']['objects'] if row['selected_object_id'] == oid]
                old_live = [row for row in c['objects'][old_core] if row['selected']['selected_object_id'] == oid]
                require(len(old_objects) == len(old_live) == 1, 'unique actual predecessor object')
                predecessor, prior = old_objects[0], old_live[0]
                require(predecessor['service_epoch_at_fence'] == w['prior_service_epoch'] and
                        predecessor['reserved_successor_epoch'] == w['actual_service_epoch'] and
                        predecessor['owner_id'] == current['owner_id'] == obj['selected']['owner_id'] and
                        predecessor['selected_generation'] == current['selected_generation'] == w['selected_generation'],
                        'checked reserved epoch and immutable object ownership transferred')
                reopened = exact_journal_supplier(c['suppliers'][core], w['reopened_journal_hash'],
                                                  w['reopened_journal_sequence'], oid)
                require(reopened['service_epoch'] == w['actual_service_epoch'],
                        'actual reopened full canonical journal')
                # The publisher hashes its exact validated input before the new
                # canonical transition; use that retained byte supplier, never a
                # journal synthesized from diagnostic rows.
                input_journal = exact_journal_supplier(c['suppliers'][old_core], reopened['prior_journal_hash'],
                                                       reopened['transition_sequence'] - 1, oid)
                require(input_journal['service_epoch'] <= w['actual_service_epoch'],
                        'actual validated predecessor journal epoch')
                require(all(reopened[field] == input_journal[field] for field in
                            ('schema_version', 'owner_id', 'selected_object_id', 'selected_generation',
                             'initial', 'signing_policy_hash', 'current')) and
                        input_journal['operation_high_water'] <= U64_MAX - 16 and
                        input_journal['writer_high_water'] <= U64_MAX - 16 and
                        reopened['operation_high_water'] == input_journal['operation_high_water'] + 16 and
                        reopened['writer_high_water'] == input_journal['writer_high_water'] + 16,
                        'reopen preserves immutable journal identity and checked reservation ranges')
                input_rows = {row['allocation']['operation_id']: row for row in input_journal['operations']}
                prior_rows = {row['allocation']['operation_id']: row for row in prior['operations']}
                require({row['allocation']['operation_id'] for row in reopened['operations']} == set(input_rows),
                        'reopen preserves exact input operation inventory')
                for row in reopened['operations']:
                    op = row['allocation']['operation_id']
                    require(op in prior_rows, 'reopen operation has actual predecessor runtime row')
                    rows_immutable_join(input_rows[op], row)
                    rows_immutable_join(prior_rows[op], row)
                    expected = dict(input_rows[op])
                    known = prior_rows[op]
                    if expected['binding'] is None and known['binding'] is not None:
                        expected.update(binding=known['binding'], submitted_service_epoch=known['submitted_service_epoch'],
                                        state='submitted')
                    if expected['permit'] is None and known['permit'] is not None:
                        expected.update(permit=known['permit'], state='permitted')
                    if known['closure'] != 'none':
                        expected['closure'] = known['closure']
                    resolving = expected['state'] in ('submitted', 'permitted')
                    if resolving:
                        expected.update(state='noncommit', successor=None,
                                        closure=expected['closure'] if expected['closure'] != 'none' else 'service_retired')
                        receipt_binding(row['receipt'], expected['binding'], expected['submitted_service_epoch'])
                        require(row['receipt']['outcome'] == 'definitive_noncommit' and
                                row['receipt']['result_revision'] == row['receipt']['committed_at_ms'] == 0,
                                'reopen exact original-epoch noncommit receipt')
                        expected['receipt'] = row['receipt']
                    elif expected['closure'] != 'none' and expected['permit'] is None and expected['state'] != 'committed':
                        expected['state'] = 'noncommit'
                    require(row == expected, 'full actual deterministic reopen operation merge')
                    if row['closure'] != 'none':
                        proof = proved.get((old_core, oid, op))
                        require(proof is not None and proof['closure'] == row['closure'] or
                                resolving and known['closure'] == 'none' and row['closure'] == 'service_retired',
                                'carried closure has predecessor owner proof or exact joined reopen resolution')
                        if proof is not None:
                            rows_immutable_join(proof, row)
                        carried[op] = row
                c['carried_closure_rows'][(core, oid)] = list(carried.values())
                checked.add((core, oid))
            closures = {row['allocation']['operation_id']: row for row in current['original_closures']}
            for row in obj['operations']:
                if row['closure'] == 'none':
                    continue
                op = row['allocation']['operation_id']
                witness = closures.get(op)
                if witness is not None:
                    require(witness['allocation'] == row['allocation'] and
                            witness['submitted_binding'] == row['binding'] and witness['closure'] == row['closure'],
                            'actual predecessor original closure exact full tuple')
                    join = actual_joins.get(witness['original_producer_id'])
                    tickets = [ticket for ticket in fence['original_tickets'] if ticket['kind'] == 'Commit' and
                               ticket['actor'] == row['allocation']['actor'] and ticket['object_id'] == oid and
                               ticket['original_epoch'] == (row['submitted_service_epoch'] or row['allocated_service_epoch']) and
                               (row['binding'] is None or ticket['binding'] == row['binding']) and
                               decode_wire88(hex_bytes(ticket['original_request'], 88, 88))['request_id'] == witness['original_request_id'] and
                               u64(decode_wire88(hex_bytes(ticket['original_request'], 88, 88))['payload'], 32) == op]
                    require(len(tickets) == 1 and join is not None and
                            join['producer_generation'] == witness['original_producer_generation'] and
                            join['kind'] == 'StoreDispatch' and join['origin_id'] == tickets[0]['origin_id'],
                            'historical original ticket joins distinct request and genuine execution origin')
                    require(sum(epoch == core and ex['request_wire'] == tickets[0]['original_request'] and
                                ex['actor'] == row['allocation']['actor'] and ex['selected_object_id'] == oid
                                for epoch, ex, _, _ in c['exchanges']) == 1,
                            'historical original actual same-Core full request trace')
                elif op in carried:
                    rows_immutable_join(carried[op], row)
                else:
                    allocation_retirement_validate(c, core, obj, row, fence, actual_joins)
                proved[(core, oid, op)] = row
    found = []
    for core, capture, header, _ in c['captures']:
        if capture['kind'] != 'SelectedTextCopy' or (core, capture['object_id']) not in checked:
            continue
        captured_exchange(c, core, capture)
        found.append((core, capture['capture_id']))
    require(found, 'required actual reopened selected64 copy missing')
    return found


def relation_grant(c, phase):
    found = []
    for row in c['summary']['grant_observations']:
        if row['phase'] != phase:
            continue
        ref = row['raw']; data = c['raw'].get(ref['file'])
        require(ref['kind'] == 'grant464' and type(data) is bytes and len(data) == 464 == ref['byte_len'] and
                digest(data) == ref['sha256'], 'actual issued full grant464')
        grant = grant464(data, phase, 2 if c['source']['scope'] == 'native-read-control' else 3)
        q, r = exchange_pair(hex_bytes(row['request'], 88, 88), hex_bytes(row['reply'], 88, 88))
        require(q['protocol'] == 352 and q['message'] == (1 if phase == 'Preview' else 5) and r['status'] == 0 and
                u64(q['payload'], 8) == grant['session_id'] and u64(q['payload'], 16) == grant['session_generation'],
                'actual Prepare/Confirm issued context')
        if phase == 'Preview':
            require(u64(r['payload'], 8) == grant['plan_id'] and u64(r['payload'], 16) == grant['preview_revision'] and
                    u32(r['payload'], 32) == 464 and u32(r['payload'], 40) == 63 and
                    q['payload'][24:56].hex() == grant['application_hash'], 'Preview exact original selection plan')
            decode_handle(u64(r['payload'], 24), 2)
        else:
            require(u64(q['payload'], 24) == grant['plan_id'] and u64(q['payload'], 32) == grant['preview_revision'] and
                    (u64(r['payload'], 8), u64(r['payload'], 16), u64(r['payload'], 24)) ==
                    (grant['session_generation'], grant['instance_id'], grant['instance_generation']) and
                    u32(r['payload'], 40) == 464, 'Active actual approved instance/grant reply')
            decode_handle(u64(r['payload'], 32), 2)
        require(any(ex['request'] == row['request'] and ex['reply'] == row['reply'] for ex in
                    c['records']['desktop.json']['exchanges']), 'issued grant belongs to actual Desktop trace')
        if c['objects']:
            selections = [obj['selected'] for objs in c['objects'].values() for obj in objs]
            selections += [op['successor'] for objs in c['objects'].values() for obj in objs for op in obj['operations'] if op['successor'] is not None]
            selections += [dict(selected_object_id=row['object_id'], selected_generation=row['generation'],
                                revision=row['revision'], content_hash=row['body_sha256']) for row in c['source']['initial_objects']]
            require(any(sel['selected_object_id'] == grant['object_id'] and sel['selected_generation'] == grant['object_generation'] and
                        sel['revision'] == grant['selected_revision'] and sel['content_hash'] == grant['selected_content_hash'] for sel in selections),
                    'issued grant pins actual immutable selected object/revision/body lineage')
        found.append(grant)
    require(found, 'required issued grant phase missing')
    return found


def relation_active_grant(c):
    return relation_grant(c, 'Active')


def relation_preview_grant(c):
    return relation_grant(c, 'Preview')


def relation_saved_chrome(c):
    frames = [frame for frame in c['summary']['frames'] if frame['chrome'] is not None and frame['chrome']['label'] == 'Saved']
    require(frames, 'required actual protected Saved frame missing')
    for frame in frames:
        frame_validate(frame, c['raw'], c['font'], frame_original(c, frame))
    return len(frames)


def relation_unknown_original(c):
    found = []
    for frame in c['summary']['frames']:
        chrome = frame['chrome']
        if chrome is None or not chrome['pending_originals']:
            continue
        frame_validate(frame, c['raw'], c['font'], frame_original(c, frame))
        for pending in chrome['pending_originals']:
            kind = 'Allocate' if pending['phase'] == 'AllocationUnknown' else 'Commit'
            tickets = [ticket for _, ticket in c['tickets'] if ticket['kind'] == kind and
                       ticket['actor'] == pending['original_actor'] and ticket['object_id'] == pending['object_id'] and
                       ticket['intent_id'] == pending['intent_id'] and ticket['original_epoch'] == pending['original_backend_epoch']]
            require(tickets and all(ticket['original_request'] == tickets[0]['original_request'] for ticket in tickets),
                    'Unknown genuine retained original owner, not stale idle')
            ticket = tickets[0]
            calls = [(row, r) for _, row, _, r in c['exchanges'] if row['request_wire'] == ticket['original_request'] and
                     row['actor'] == ticket['actor'] and row['selected_object_id'] == ticket['object_id']]
            require(len(calls) == 1, 'Unknown first original execution no replay across epochs')
            row, r = calls[0]
            require(row['reply_suppressed'] or r is not None and r['status'] in (8, 10), 'actual lost/deadline original outcome')
            if kind == 'Commit':
                require(ticket['binding'] is not None and pending['operation_id'] == ticket['binding']['allocation']['operation_id'] and
                        any(cap['kind'] == 'DraftSourceFreeze' and cap['original_request'] == ticket['original_request'] and
                            cap['actor'] == ticket['actor'] and binding == ticket['binding'] for _, cap, _, binding in c['captures']),
                        'Unknown Commit real sealed source/original Binding')
            else:
                require(ticket['binding'] is None and (pending['operation_id'] is None or ticket['allocation'] is not None and
                        pending['operation_id'] == ticket['allocation']['operation_id']), 'allocation Unknown no fake Commit binding')
            found.append((kind, ticket['origin_id']))
    require(found, 'required Unknown real protected original missing')
    return found


def relation_joined_fence(c):
    cleanup, joins = c['records']['cleanup.json'], c['records']['joins.json']
    require(c['source']['scope'] in ('composition', 'direct-native') and len(cleanup['fences']) == len(c['source']['core_ordinals']),
            'actual Save Core/fence inventory')
    require(len(c['source']['core_ordinals']) == 1 or
            'reopened_selected_read' in c['source']['required_relations'],
            'multi-Core installed history requires complete reopen lineage relation')
    actual = join_fence_validate(joins['native_join_observations'], cleanup['fences'])
    cleanup_validate(cleanup, joins, c['source']['scope'])
    c['allocation_retirements'] = {}
    for core, fence in zip(c['source']['core_ordinals'], cleanup['fences']):
        for snap in c['snapshots']:
            if snap['binding']['epoch_index'] != core:
                continue
            for obj in snap['objects']:
                for row in obj['operations']:
                    if (row['closure'] in ('service_retired', 'deadline') and row['binding'] is None and
                            row['allocated_service_epoch'] == obj['service_epoch']):
                        proof = allocation_retirement_validate(c, core, obj, row, fence, actual)
                        rows = c['allocation_retirements'].setdefault((core, obj['selected']['selected_object_id']), [])
                        if proof not in rows:
                            require(len(rows) < 16, 'bounded actual allocation retirement rows')
                            rows.append(proof)
        batch = next(batch for batch in joins['fence_original_ticket_batches'] if batch['epoch_index'] == core)
        require(batch['original_tickets'] == fence['original_tickets'], 'actual same-Core original fence owner array')
        admitted = {(row['epoch_index'], row['observation_index']): row for row in c['source']['store_snapshots']}
        require(any(row['epoch_index'] == core and row['durability'] == 'joined_checkpoint'
                    for row in admitted.values()), 'actual Core source declares joined checkpoint')
        checkpoint_views_validate(c, core, fence)
    return len(actual)


def relation_negative_source(c):
    rows = [row for row in c['summary']['negative_inputs'] if row['kind'] == 'text64_negative']
    sites = [row for row in c['source']['negative_controls'] if row['kind'] == 'text64_negative']
    require(rows and sites, 'required actual mutable source negatives missing')
    negative_inputs_validate(rows, sites, c['raw'])
    for row in rows:
        q = decode_wire88(hex_bytes(row['original_request'], 88, 88))
        calls = [(core, ex) for core, ex, _, _ in c['exchanges'] if
                 ex['request_wire'] == row['original_request'] and ex['actual_reply_wire'] == row['actual_reply']]
        require(len(calls) == 1, 'negative source exact one real original Commit invocation')
        core, ex = calls[0]
        oid, op = ex['selected_object_id'], u64(q['payload'], 32)
        objects = [obj for obj in c['objects'][core] if obj['selected']['selected_object_id'] == oid]
        require(len(objects) == 1, 'negative source actual invocation Core/object, not numerical cross-Core alias')
        obj = objects[0]
        operations = [rec for rec in obj['operations'] if rec['allocation']['operation_id'] == op]
        require(len(operations) == 1, 'negative source retained original allocated operation must exist')
        rec = operations[0]; a = rec['allocation']
        require(a['actor'] == ex['actor'] and a['selected_object_id'] == oid and
                a['selected_generation'] == obj['selected']['selected_generation'] and
                a['expected_revision'] == u64(q['payload'], 24), 'negative source exact actor/object/allocation base')
        require(rec['permit'] is None and rec['successor'] is None and
                rec['state'] in ('allocated', 'submitted', 'noncommit'), 'negative source no issued permit/successor')
        if rec['binding'] is not None:
            require(rec['binding']['allocation'] == a and
                    rec['binding']['source']['source_object_id'] == u64(q['payload'], 40),
                    'negative source actual optional submitted binding')
        tickets = [ticket for current, ticket in c['tickets'] if current == core and
                   ticket['kind'] == 'Commit' and ticket['original_request'] == row['original_request'] and
                   ticket['actor'] == ex['actor'] and ticket['object_id'] == oid]
        require(len(tickets) == 1, 'negative source actual same-Core retained original Commit ticket')
        commit = tickets[0]
        require(commit['allocation'] is None and commit['binding'] == rec['binding'] and
                commit['original_epoch'] == rec['allocated_service_epoch'] and
                commit['closed'] is True and commit['completion'] == 'JoinedReturned',
                'rejected Commit retains actual pre-seal or exact submitted ticket state')
        allocations = [ticket for current, ticket in c['tickets'] if current == core and
                       ticket['kind'] == 'Allocate' and ticket['allocation'] == a and
                       ticket['actor'] == commit['actor'] and ticket['object_id'] == oid and
                       ticket['original_epoch'] == commit['original_epoch'] and
                       ticket['intent_id'] == commit['intent_id'] and ticket['binding'] is None and
                       ticket['completion'] == 'JoinedReturned']
        require(len(allocations) == 1 and commit['intent_id'] is not None,
                'negative original full allocation belongs to genuine same-intent Allocate ticket')
        allocation = allocations[0]
        require(allocation['origin_id'] != commit['origin_id'],
                'Allocate and rejected Commit retain distinct actual execution owners')
        allocated = [(ex, request, reply) for current, ex, request, reply in c['exchanges'] if current == core and
                     ex['request_wire'] == allocation['original_request'] and ex['actor'] == a['actor'] and
                     ex['selected_object_id'] == oid and request['message'] == 3 and
                     u64(request['payload'], 24) == a['expected_revision'] and reply is not None and
                     reply['status'] == 0 and u64(reply['payload'], 8) == op]
        require(len(allocated) == 1, 'negative Commit allocation has actual successful original Allocate reply')
        actual_joins = join_fence_validate(c['records']['joins.json']['native_join_observations'],
                                          c['records']['cleanup.json']['fences'])
        for ticket in (allocation, commit):
            require(sum(join['kind'] == 'StoreDispatch' and join['origin_id'] == ticket['origin_id'] and
                        join['producer_generation'] == 1 and join['outcome'] == 'Returned'
                        for join in actual_joins.values()) == 1,
                    'negative Allocate and Commit retain genuine distinct returned dispatcher joins')
        require(not any(join['kind'] == 'StoreIo' and join['origin_id'] == commit['origin_id']
                        for join in actual_joins.values()), 'rejected source original has no IO producer')
        if commit['binding'] is None:
            require(not any(current == core and cap['original_request'] == row['original_request'] and
                            cap['kind'] in ('DraftSourceFreeze', 'ReceiptCopy')
                            for current, cap, _, _ in c['captures']),
                    'pre-seal source rejection fabricates no accepted source or receipt capture')
        require(not any(event['selected_object_id'] == oid and event['operation_id'] == op and
                        event['writer_id'] != 0
                        for snap in c['snapshots'] if snap['binding']['epoch_index'] == core
                        for event in snap['io_events']), 'negative original no permit writer or selection IO')
    return len(rows)


def relation_negative_wire(c):
    rows = [row for row in c['summary']['negative_inputs'] if row['kind'] == 'wire88_negative']
    sites = [row for row in c['source']['negative_controls'] if row['kind'] == 'wire88_negative']
    require(rows and sites, 'required actual wire negative controls missing')
    negative_inputs_validate(rows, sites, c['raw'])
    for row in rows:
        if row['actual_denial_layer'] == 'wire_decoder':
            require(not any(ex['request_wire'] == c['raw'][row['raw']['file']].hex() for _, ex, _, _ in c['exchanges']),
                    'codec Err not fabricated public native exchange')
    return len(rows)


def relation_capacity_boundary(c):
    errors = [row for row in c['summary']['control_observations'] if row['actual_status']['result'] == 'Err' and
              row['actual_status']['status'] == 5]
    errors += [row for _, row, _, reply in c['exchanges'] if reply is not None and reply['status'] == 5]
    # This source-fixed ServiceEpochMAX leg has an actual typed reopen error,
    # not a public request/reply or a fabricated unit_result control.
    if (c['summary']['binding']['case'] == 'ui1_1_clock_capacity_and_counter_limits_fail_closed' and
            c['source']['scenario'] == 'store_max' and c['source']['source_leg'] == 2):
        errors += [snap['reopen_error'] for snap in c['snapshots'] if
                   snap['phase'] == 'actual_failed_reopen_closed' and snap['purpose'] == 'final' and
                   snap['reopen_error'] is not None and
                   type(snap['reopen_error']['status']) is int and snap['reopen_error']['status'] == 5]
    require(errors, 'actual source-site typed Exhausted capacity control')
    for snap in c['snapshots']:
        ledger = snap['ledger']
        require(ledger['shared_objects'] <= 32 and ledger['active_io_workers'] <= 2 and ledger['held_service_producers'] <= 64 and
                sum(len(obj['operations']) for obj in snap['objects']) <= 48, 'exported shared32/IO2/roster64/history48 bounds')
    for batch in c['records'].get('artifacts.json', {}).get('batches', []):
        ids = [row['capture_id'] for row in batch['captures']]
        require(len(ids) <= 256 and len(ids) == len(set(ids)), 'complete capture256 distinct IDs/no coalescing')
    require(all(len(batch['original_tickets']) <= 96 for batch in c['records']['joins.json']['fence_original_ticket_batches']) and
            len(c['summary']['frames']) <= 128, 'complete tickets96/frame128 bounds')
    # pin16 and private native MAX counters are genuine reviewed Rust18 behavior
    # assertions, not invented numeric fields in unit_result. Outer actual18PASS
    # is still required; this relation checks exported consistency only.
    return {'exported_exhausted_controls': len(errors), 'private_count_scope': 'ACTUAL_RUST18_REQUIRED'}


RELATION_CHECKERS = {
    'app_save_chain': relation_app_save_chain, 'reopened_selected_read': relation_reopened_selected_read,
    'active_grant': relation_active_grant, 'preview_grant': relation_preview_grant,
    'saved_chrome': relation_saved_chrome, 'unknown_original': relation_unknown_original,
    'joined_fence': relation_joined_fence, 'negative_source': relation_negative_source,
    'negative_wire': relation_negative_wire, 'capacity_boundary': relation_capacity_boundary,
}



def desktop_exchange_pairs(c):
    """Preserve one source-fixed invalid reserved field as denial evidence only."""
    binding = c['summary']['binding']
    malformed_case = (binding['case'] == 'ui1_1_ascii_bounds_preserve_prior_draft'
                      and binding['scenario'] == 'primary/0')
    rows, malformed_count = [], 0
    for row in c['records'].get('desktop.json', {}).get('exchanges', []):
        request, reply = hex_bytes(row['request'], 88, 88), hex_bytes(row['reply'], 88, 88)
        try:
            pair = exchange_pair(request, reply)
        except Denied:
            require(malformed_case and u32(request, 0) == 802 and u32(request, 4) == 3
                    and u32(request, 16) == 48 and u32(request, 64) == 1,
                    'only source-fixed malformed reserved1 input')
            canonical = bytearray(request)
            canonical[64:68] = bytes(4)
            q, r = exchange_pair(bytes(canonical), reply, native_error_redaction=True)
            require(r['status'] == 2, 'actual malformed input Invalid denial')
            payload = bytearray(q['payload'])
            payload[44:48] = request[64:68]
            q['payload'] = bytes(payload)
            pair = q, r
            malformed_count += 1
            require(malformed_count == 1, 'one actual malformed input row')
        rows.append(pair)
    require(malformed_count == int(malformed_case), 'complete malformed input denial row')
    return rows


def validate_common_observations(c):
    desktop = desktop_exchange_pairs(c)
    for frame in c['summary']['frames']:
        frame_validate(frame, c['raw'], c['font'], frame_original(c, frame))
    app = [row for row in c['summary']['raw_inventory'] if row['kind'] == 'app_frame']
    require(len(app) <= 1, 'one source-fixed actual Surface full copy')
    for ref in app:
        data = c['raw'][ref['file']]
        require(len(data) == ref['byte_len'] == 1228800 and digest(data) == ref['sha256'], 'full actual frozen Surface copy')
        matched = []
        for frame in c['summary']['frames']:
            if frame['raw']['kind'] != 'composed_frame' or frame['app_crop_sha256'] != ref['sha256']:
                continue
            actual = c['raw'][frame['raw']['file']]
            require(actual[48 * 2560:528 * 2560] == data, 'Surface actual full-byte composed crop identity')
            for present_index, (q, r) in enumerate(desktop):
                if not (q['protocol'] == 833 and q['message'] == 5 and r['status'] == 0 and
                        u64(q['payload'], 32) == frame['sequence'] and
                        u64(r['payload'], 8) == frame['sequence']):
                    continue
                surface, generation, mapping = (u64(q['payload'], off) for off in (8, 16, 24))
                acquired = [(a, b) for a, b in desktop[:present_index] if a['protocol'] == 833 and a['message'] == 3 and
                            b['status'] == 0 and a['handle'] == q['handle'] and
                            (u64(a['payload'], 8), u64(a['payload'], 16), u32(a['payload'], 24)) ==
                            (surface, generation, u32(q['payload'], 40)) and
                            u64(b['payload'], 8) == mapping and u32(b['payload'], 28) == 1228800]
                # Actual successful Focus.Assign binds the same instance and
                # Surface. Generic Present deliberately invalidates the private
                # NativeFrameObservation, so it is not required for this copy.
                owners = [(a, b) for a, b in desktop[:present_index] if a['protocol'] == 832 and a['message'] == 1 and
                          b['status'] == 0 and
                          (u64(a['payload'], 8), u64(a['payload'], 24), u64(a['payload'], 40), u64(a['payload'], 48)) ==
                          (frame['session_id'], frame['instance_id'], surface, generation) and
                          u64(b['payload'], 8) == frame['focus_epoch']]
                if acquired and owners:
                    matched.append((frame['observation_index'], surface, generation, mapping))
        require(matched, 'Surface copied bytes link actual Acquire/Present/owned Surface and composed publication')
    return len(desktop)


def validate_required_relations(context):
    names = context['source']['required_relations']
    require(type(names) is list and len(set(names)) == len(names) <= 10, 'closed distinct required relation inventory')
    require(context['source']['scope'] not in ('composition', 'direct-native') or 'joined_fence' in names,
            'native Save evidence requires complete actual joined fence relation')
    validate_common_observations(context)
    result = {}
    # Authenticate all transferred closure rows before any live checkpoint may
    # use one instead of a new original-request witness.
    if 'reopened_selected_read' in names:
        result['reopened_selected_read'] = relation_reopened_selected_read(context)
    for name in names:
        require(name in RELATION_CHECKERS, 'unknown/unimplemented relation denied')
        if name not in result:
            result[name] = RELATION_CHECKERS[name](context)
    return result


def check_deadline(deadline):
    require(type(deadline) in (int, float) and deadline > 0
            and deadline < float('inf') and time.monotonic() < deadline,
            'original absolute reader deadline')


def pin(value, reason):
    require(type(value) is str and re.fullmatch('[0-9a-f]{64}', value) is not None,
            reason)
    return value


def raw_references(value, result, deadline, depth=0):
    """Find only closed RawRef objects in already shape-validated records.

    Summary.raw_inventory is excluded by the caller: it cannot self-authorize
    an otherwise unused file. Repeated uses must bind the same complete ref.
    """
    check_deadline(deadline)
    require(depth <= 64, 'raw reference walk depth')
    if type(value) is dict:
        if set(value) == {'file', 'kind', 'byte_len', 'sha256'}:
            name = value['file']
            require(name not in result or result[name] == value,
                    'all record raw references identical')
            require(name in result or len(result) < 2048, 'scenario raw refs2048')
            result[name] = value
        else:
            for child in value.values():
                raw_references(child, result, deadline, depth + 1)
    elif type(value) is list:
        for child in value:
            raw_references(child, result, deadline, depth + 1)


def raw_inventory_admit(records, source, case, cases, path, deadline, case_names):
    """Bind complete runtime RawInventory to genuine source-admitted outputs.

    Every template is taken from the fixed recorder source, not a free path
    pattern in a record. Core ordinal, snapshot index and family-local ordinals
    have different meanings and cannot substitute for one another.
    """
    summary = records['summary.json']
    inventory = {}
    for ref in summary['raw_inventory']:
        check_deadline(deadline)
        require(ref['file'] not in inventory, 'actual raw inventory names unique')
        inventory[ref['file']] = ref
    references = {}
    for name, record in records.items():
        if name == 'summary.json':
            for key, value in record.items():
                if key != 'raw_inventory':
                    raw_references(value, references, deadline)
        else:
            raw_references(record, references, deadline)

    template_names = set()
    zero_files = set()

    def bind(ref, name, kind):
        check_deadline(deadline)
        require(ref['file'] == name and ref['kind'] == kind
                and inventory.get(name) == ref, 'source template and whole raw ref match')
        require(name not in template_names, 'one source producer for each raw name')
        template_names.add(name)

    captures = records.get('artifacts.json', {}).get('batches', [])
    capture_kinds = {'SelectedTextCopy': 'text64', 'DraftSourceFreeze': 'text64',
                     'ReceiptCopy': 'receipt176'}
    for batch in captures:
        previous = 0
        for capture in batch['captures']:
            identifier = integer(capture['capture_id'], minimum=1)
            require(identifier > previous, 'actual Core capture IDs monotonic/no reuse')
            previous = identifier
            bind(capture['raw'], 'capture-%d-%d.bin' % (batch['epoch_index'], identifier),
                 capture_kinds[capture['kind']])

    for frame in summary['frames']:
        index = frame['observation_index']
        bind(frame['raw'], 'frame-%d.bgra' % index, 'composed_frame')
        chrome = frame['chrome']
        if chrome is not None and chrome['record248'] is not None:
            data = hex_bytes(chrome['record248'], 248, 248)
            name = 'chrome-%d.bin' % index
            ref = {'file': name, 'kind': 'chrome248', 'byte_len': 248, 'sha256': digest(data)}
            bind(ref, name, 'chrome248')
            require(name not in references, 'implicit chrome raw has one publication owner')
            references[name] = ref

    for grant in summary['grant_observations']:
        bind(grant['raw'], 'grant-%d.bin' % grant['observation_index'], 'grant464')

    exceptions = {(row['epoch_index'], row['observation_index'], row['snapshot_purpose'],
                   row['relative_path']): row for row in source['file_exceptions']}
    for snapshot in source['store_snapshots']:
        core, index = snapshot['epoch_index'], snapshot['observation_index']
        record = records['store-epoch-%d-snapshot-%d.json' % (core, index)]
        for row_index, observation in enumerate(record['files']):
            ref = observation['raw']
            if ref is None:
                continue
            bind(ref, 'file-%d-%d-%d.bin' % (core, index, row_index), ref['kind'])
            require(ref['kind'] in ('journal', 'corrupt_journal', 'cas_blob', 'fixture_file'),
                    'observed fixture file closed raw class')
            if ref['byte_len'] == 0:
                key = (core, index, snapshot['purpose'], observation['relative_path'])
                exception = exceptions.get(key)
                require(exception is not None and exception['allow_empty'] is True
                        and exception['raw_kind'] == 'cas_blob'
                        and exception['reason'] == 'EmptyAsciiObject'
                        and ref['kind'] == 'cas_blob'
                        and observation['entry_kind'] == 'regular'
                        and observation['byte_len'] == 0
                        and observation['relative_path'].endswith('/' + digest(b'') + '.blob'),
                        'zero only exact source-owned EmptyAsciiObject')
                zero_files.add(ref['file'])

    for index, negative in enumerate(summary['negative_inputs']):
        kind = negative['kind']
        prefix = 'negative-source' if kind == 'text64_negative' else 'negative-wire'
        bind(negative['raw'], '%s-%d.bin' % (prefix, index), kind)

    # These two actual copies have no separate RawRef-bearing DTO. Their only
    # admission is exact source case/scope, exact leaf and the trusted kind bound.
    required_cases = case_names
    extras = set(inventory) - set(references)
    for name in extras:
        ref = inventory[name]
        allowed = ((case == required_cases[4] and name == 'surface-frozen-copy-0.bgra'
                    and ref['kind'] == 'app_frame' and source['scope'] == 'direct-native')
                   or (case == required_cases[7] and name == 'volatile-receipt.bin'
                       and ref['kind'] == 'receipt176' and source['scope'] == 'volatile-control'))
        require(allowed, 'no unused raw output outside exact two source copies')
        bind(ref, name, ref['kind'])
        references[name] = ref

    require(set(references) == set(inventory) == template_names,
            'complete actual inventory/reference/source template sets')
    require(all(references[name] == ref for name, ref in inventory.items()),
            'all raw references exactly match full inventory')
    limits = {row['kind']: row for row in source['raw_limits']}
    require(set(limits) == set(RAW_BOUNDS), 'trusted thirteen raw class limits complete')
    counts = Counter(ref['kind'] for ref in inventory.values())
    for kind, bound in limits.items():
        require(bound['min_count'] <= counts[kind] <= bound['max_count'],
                'complete actual raw multiplicity within source fixed kind bounds: ' + kind)
    raw = {}
    for name, ref in inventory.items():
        raw[name] = cases.raw(path, ref, empty_allowed=name in zero_files)
        check_deadline(deadline)
    for frame in summary['frames']:
        chrome = frame['chrome']
        if chrome is not None and chrome['record248'] is not None:
            require(raw['chrome-%d.bin' % frame['observation_index']] ==
                    hex_bytes(chrome['record248'], 248, 248), 'actual copied248 equals same publication')
    for grant in summary['grant_observations']:
        grant464(raw[grant['raw']['file']], grant['phase'],
                 2 if source['scope'] == 'native-read-control' else 3)
    return raw


def volatile_receipt_join(records, raw):
    """The separately stamped old volatile control is not a native Store chain.

    Its single actual copied receipt can link the corresponding old Commit and
    receipt-query envelopes plus original instance in its composed frame. This
    does not confer native Core, durable CAS, join, current-draft or Saved proof.
    """
    if 'volatile-receipt.bin' not in raw:
        return
    receipt = decode_receipt176(raw['volatile-receipt.bin'])
    rows = [exchange_pair(hex_bytes(row['request'], 88, 88), hex_bytes(row['reply'], 88, 88))
            for row in records['desktop.json']['exchanges']]
    commits = [(q, r) for q, r in rows if q['protocol'] == 368 and q['message'] == 5
               and r['status'] == 0 and u64(q['payload'], 32) == receipt['operation_id']
               and (u64(q['payload'], 8), u64(q['payload'], 16), u64(q['payload'], 24)) ==
               (receipt['session_id'], receipt['session_generation'], receipt['expected_revision'])
               and (u64(r['payload'], 8), u64(r['payload'], 16), u32(r['payload'], 36)) ==
               (receipt['operation_id'], receipt['result_revision'], 176)]
    require(len(commits) == 1 and receipt['outcome'] == 'committed',
            'volatile copied receipt exact actual Commit correlation and outcome')
    commit, _ = commits[0]
    require(any(q['protocol'] == 368 and q['message'] == 7 and q['handle'] == commit['handle']
                and r['status'] == 0 and (u64(q['payload'], 8), u64(q['payload'], 16),
                u64(q['payload'], 24)) == (receipt['session_id'], receipt['session_generation'],
                receipt['operation_id']) and (u64(r['payload'], 8), u64(r['payload'], 16),
                u32(r['payload'], 36)) == (receipt['operation_id'], receipt['result_revision'], 176)
                for q, r in rows), 'volatile actual receipt query matches copied receipt')
    require(any(frame['session_id'] == receipt['session_id']
                and frame['instance_id'] == receipt['original_instance_id']
                for frame in records['summary.json']['frames']), 'volatile original instance frame correlation')


def validate_recording(held_cases_fd, admitted_sources, run_nonce, schema_bytes,
                       expected_schema_sha256, inventory_bytes, expected_inventory_sha256,
                       expected_contract_sha256, expected_recording_sha256, font_bytes,
                       root_verifier, deadline):
    """Source-only final-entry candidate; outer gate remains DEFERRED.

    All inputs are selected by the trusted outer runner before execution. The
    cases FD grants readonly inspection of already emitted data only. This
    function never chooses a source/binary/key/callback, opens an artifact root,
    spawns a process, signals a PID or replays evidence. Root callback execution
    remains disabled until separate verifier and outer integration review.
    """
    check_deadline(deadline)
    for value in (expected_schema_sha256, expected_inventory_sha256,
                  expected_contract_sha256, expected_recording_sha256):
        pin(value, 'root-selected complete input pin required')
    require(type(run_nonce) is str and re.fullmatch('[0-9a-f]{32}', run_nonce) is not None,
            'root-selected actual run nonce32hex')
    require(type(schema_bytes) is bytes and 0 < len(schema_bytes) <= 1_048_576
            and digest(schema_bytes) == expected_schema_sha256, 'held trusted schema raw pin')
    schema = bounded_json(schema_bytes, 1_048_576)
    require(type(schema) is dict and type(schema.get('$defs')) is dict
            and len(schema['$defs']) == 93, 'complete trusted93 definitions')
    require(schema.get('x-contract-sha256') == expected_contract_sha256
            and schema.get('x-recording-proposal-sha256') == expected_recording_sha256,
            'schema independent contract/recording projections')
    definitions = schema['$defs']
    require(type(font_bytes) is bytes and len(font_bytes) == 1536
            and digest(font_bytes) == FONT_SHA256, 'actual independently pinned font1536')
    inventory, scenarios = trusted_inventory(inventory_bytes, definitions, admitted_sources,
        expected_inventory_sha256, expected_contract_sha256, expected_recording_sha256)
    check_deadline(deadline)
    # Pass exact case names as data to the raw classifier, never ambient state.
    case_names = definitions['SourceCase']['properties']['case']['enum']
    require(type(root_verifier) is dict and set(root_verifier) ==
            {'callback', 'source_sha256', 'binary_sha256', 'review_sha256', 'public_key'}
            and callable(root_verifier['callback']), 'root-selected actual verifier seam required')
    # Retain the root's immutable selected tuple. Artifacts cannot replace this
    # dictionary or select a key, source, executable or callback during traversal.
    verifier = dict(root_verifier)
    for key in ('source_sha256', 'binary_sha256', 'review_sha256', 'public_key'):
        pin(verifier[key], 'root verifier independently fixed pin')
    cases = CaseFiles(held_cases_fd, deadline)
    expected_files, expected_directories = set(), {''}
    outcomes = []
    try:
        for (case, stem, leg), admitted in scenarios.items():
            check_deadline(deadline)
            path, source = admitted['path'], admitted['source']
            parts = path.split('/')
            for count in range(1, len(parts) + 1):
                expected_directories.add('/'.join(parts[:count]))
            records = {}
            for name in source['record_names']:
                full = path + '/' + name
                require(full not in expected_files, 'global record path unique')
                expected_files.add(full)
                data = cases.read(full, 33_554_432)
                records[name] = bounded_json(data, 33_554_432)
                check_deadline(deadline)
            scenario_records_validate(records, definitions, source, case, run_nonce)
            raw = raw_inventory_admit(records, source, case, cases, path, deadline, case_names)
            require(len(raw) <= 2048, 'complete scenario raw files2048')
            expected_files.update(path + '/' + name for name in raw)
            negative_inputs_validate(records['summary.json']['negative_inputs'],
                                     source['negative_controls'], raw)
            volatile_receipt_join(records, raw)
            cleanup_validate(records['cleanup.json'], records.get('joins.json'), source['scope'])
            check_deadline(deadline)
            context = relation_context(records, source, definitions, raw, font_bytes, verifier)
            check_deadline(deadline)
            relations = validate_required_relations(context)
            check_deadline(deadline)
            outcomes.append({'case': case, 'scenario': admitted['binding_scenario'],
                             'scope': source['scope'], 'raw_files': len(raw),
                             'records': len(records), 'required_relations': list(relations)})
            del context, raw, records
        require(set(cases.files) == expected_files and set(cases.directories) == expected_directories,
                'exact complete emitted file/directory sets, no missing/extra/transient marker')
        check_deadline(deadline)
        final = CaseFiles(cases.fd, deadline)
        try:
            require(final.files == cases.files and final.directories == cases.directories
                    and final.total_bytes == cases.total_bytes and final.json_bytes == cases.json_bytes,
                    'actual entire held tree unchanged at final admission')
        finally:
            final.close()
        check_deadline(deadline)
        return {'schema_version': 1, 'scope': 'exported-native-task-data-consistency',
                'cases': len(inventory['cases']), 'scenarios': outcomes,
                'file_count': len(expected_files), 'entry_count': cases.entries,
                'byte_len': cases.total_bytes, 'json_byte_len': cases.json_bytes,
                'schema_sha256': expected_schema_sha256, 'inventory_sha256': expected_inventory_sha256,
                'contract_sha256': expected_contract_sha256,
                'recording_sha256': expected_recording_sha256,
                'font_sha256': FONT_SHA256,
                'source_authority_claim': False, 'native_lifecycle_claim': False,
                'runtime_acceptance_claim': False}
    finally:
        cases.close()
