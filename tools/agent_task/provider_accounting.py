#!/usr/bin/env python3
"""Offline frozen usage accounting of trusted host records, not provider attestation."""

import copy
import hashlib
import re

from fixture_bank import BankError, encoded, manifest_hash, release_record, validate_manifest

ARMS = ("RT", "LT", "LS")
MAX_INTEGER = (1 << 63) - 1
STUDY_FIELDS = {
    "schema_version", "bank_kind", "bank_sha256", "release", "fixture_ids",
    "provider", "model", "rate_schedule_id", "input_usd_micros_per_million",
    "output_usd_micros_per_million", "context_policy_sha256", "max_input_tokens",
    "max_output_tokens", "funded_ceiling_usd_micros", "attempts",
}
RELEASE_FIELDS = {
    "schema_version", "bank_sha256", "partition", "purpose", "bank_commit", "study_sha256",
}
ROW_FIELDS = {
    "schema_version", "accounting_study_sha256", "attempt_id", "instance_id", "arm",
    "outcome", "failure_reason", "input_tokens", "output_tokens", "model_calls",
    "visible_context_bytes", "transcript_sha256",
}


class AccountingError(ValueError):
    pass


def _require(condition, reason):
    if not condition:
        raise AccountingError(reason)


def _object(value, fields, reason):
    _require(type(value) is dict and set(value) == fields, reason)


def _integer(value):
    return type(value) is int and 0 <= value <= MAX_INTEGER


def _hash(value, length=64):
    return type(value) is str and re.fullmatch(r"[0-9a-f]{%d}" % length, value) is not None


def _identity(value):
    return type(value) is str and re.fullmatch(r"[A-Za-z0-9][A-Za-z0-9._:/+\-]{0,127}", value) is not None


def study_digest(study):
    """Hash the complete accounting plan, including the opaque external study digest.

    This is accounting_study_sha256, distinct from release.study_sha256. The latter
    binds a separately frozen comparison study; no circular hash or omitted fields.
    Use validate_study before trusting this identity.
    """
    return hashlib.sha256(encoded(study)).hexdigest()


def _schedule(study):
    return [dict(
        attempt_id=hashlib.sha256(encoded(dict(
            bank_sha256=study["bank_sha256"], release=study["release"],
            instance_id=instance_id, arm=arm,
        ))).hexdigest(),
        instance_id=instance_id, arm=arm,
    ) for instance_id in study["fixture_ids"] for arm in ARMS]


def _validate_basis(study, manifest):
    """Validate bounded independent fields before constructing derived schedules."""
    _object(study, STUDY_FIELDS, "invalid accounting study fields")
    _require(type(study["schema_version"]) is int and study["schema_version"] == 1,
             "unsupported accounting study version")
    try:
        validate_manifest(manifest)
        _require(study["bank_kind"] == manifest["kind"]
                 and study["bank_sha256"] == manifest_hash(manifest), "bank mismatch")
        release = study["release"]
        _object(release, RELEASE_FIELDS, "invalid release fields")
        _require(type(release["schema_version"]) is int and release["schema_version"] == 1,
                 "unsupported release version")
        _require(release == release_record(
            manifest, release["partition"], release["purpose"],
            bank_commit=release["bank_commit"], study_sha256=release["study_sha256"],
        ), "release mismatch")
    except (BankError, KeyError, TypeError) as exc:
        raise AccountingError("invalid bank/release contract") from exc
    ids = study["fixture_ids"]
    _require(type(ids) is list and 0 < len(ids) <= 15
             and all(_hash(i, 32) for i in ids), "invalid fixture identities")
    _require(len(set(ids)) == len(ids), "duplicate fixtures are not independent attempts")
    available = {i["instance_id"] for i in manifest["instances"]
                 if i["partition"] == release["partition"]}
    _require(set(ids) <= available, "fixture outside released partition")
    for key in ("provider", "model", "rate_schedule_id"):
        _require(_identity(study[key]), "invalid frozen " + key)
    _require(_hash(study["context_policy_sha256"]), "invalid context policy identity")
    for key in ("input_usd_micros_per_million", "output_usd_micros_per_million",
                "max_input_tokens", "max_output_tokens", "funded_ceiling_usd_micros"):
        _require(_integer(study[key]), "invalid frozen integer: " + key)
    _require(study["max_input_tokens"] > 0 and study["max_output_tokens"] > 0,
             "token budgets must be positive")

def validate_study(study, manifest):
    """Fail closed on malformed plans and exact bank/release/partition mismatch."""
    _validate_basis(study, manifest)
    _require(type(study["attempts"]) is list
             and len(study["attempts"]) == 3 * len(study["fixture_ids"])
             and study["attempts"] == _schedule(study),
             "changed or incomplete frozen three-arm schedule")


def freeze_study(manifest, release, *, fixture_ids, provider, model, rate_schedule_id,
                 input_usd_micros_per_million, output_usd_micros_per_million,
                 context_policy_sha256, max_input_tokens, max_output_tokens,
                 funded_ceiling_usd_micros):
    """Snapshot one immutable accounting plan; no release or funding authority."""
    _require(type(manifest) is dict and type(release) is dict and type(fixture_ids) is list,
             "invalid accounting freeze inputs")
    try:
        validate_manifest(manifest)
    except (BankError, TypeError, KeyError) as exc:
        raise AccountingError("invalid bank contract") from exc
    study = dict(
        schema_version=1, bank_kind=manifest["kind"], bank_sha256=manifest_hash(manifest),
        release=release, fixture_ids=fixture_ids,
        provider=provider, model=model, rate_schedule_id=rate_schedule_id,
        input_usd_micros_per_million=input_usd_micros_per_million,
        output_usd_micros_per_million=output_usd_micros_per_million,
        context_policy_sha256=context_policy_sha256, max_input_tokens=max_input_tokens,
        max_output_tokens=max_output_tokens, funded_ceiling_usd_micros=funded_ceiling_usd_micros,
        attempts=[],
    )
    _validate_basis(study, manifest)
    study = copy.deepcopy(study)
    study["attempts"] = _schedule(study)
    validate_study(study, manifest)
    return study


def _validate_row(row, digest, attempts):
    _object(row, ROW_FIELDS, "invalid usage record fields")
    _require(type(row["schema_version"]) is int and row["schema_version"] == 1,
             "unsupported usage record version")
    _require(row["accounting_study_sha256"] == digest, "usage belongs to another accounting study")
    _require(_hash(row["attempt_id"]) and row["attempt_id"] in attempts,
             "unknown attempt identity")
    attempt = attempts[row["attempt_id"]]
    _require(row["instance_id"] == attempt["instance_id"] and row["arm"] == attempt["arm"],
             "attempt fixture/arm mismatch")
    _require(row["outcome"] in ("completed", "failed"), "invalid attempt outcome")
    reason = row["failure_reason"]
    _require((row["outcome"] == "completed" and reason is None)
             or (row["outcome"] == "failed" and type(reason) is str and 0 < len(reason) <= 512
                 and all(ch.isprintable() for ch in reason)), "invalid failure reason")
    for key in ("input_tokens", "output_tokens"):
        _require(row[key] is None or _integer(row[key]), "invalid usage integer: " + key)
    for key in ("model_calls", "visible_context_bytes"):
        _require(_integer(row[key]), "invalid usage integer: " + key)
    _require(_hash(row["transcript_sha256"]), "invalid transcript identity")
    if row["model_calls"] == 0:
        _require(row["outcome"] == "failed" and row["input_tokens"] == 0
                 and row["output_tokens"] == 0, "zero calls require known-zero predispatch failure")


def accounting_report(study, manifest, records):
    """Retain terminal rows and partial usage; missing/unknown usage cannot certify.

    Rates price two uncached token categories only. Rounded per-attempt estimates
    are not invoices and omit cached, tool, storage and other provider charges.
    Policy violations remain evidence, not malformed records to discard.
    """
    validate_study(study, manifest)
    _require(type(records) is list and len(records) <= len(study["attempts"]),
             "invalid or excess attempt records")
    digest = study_digest(study)
    attempts = {a["attempt_id"]: a for a in study["attempts"]}
    indexed = {}
    for row in records:
        _validate_row(row, digest, attempts)
        _require(row["attempt_id"] not in indexed, "duplicate attempts are not retried")
        indexed[row["attempt_id"]] = copy.deepcopy(row)
    ordered = [indexed[a["attempt_id"]] for a in study["attempts"]
               if a["attempt_id"] in indexed]
    pending = [a["attempt_id"] for a in study["attempts"] if a["attempt_id"] not in indexed]
    unknown = 0
    known_input = known_output = lower_cost = 0
    breaches = []
    for row in ordered:
        input_tokens, output_tokens = row["input_tokens"], row["output_tokens"]
        unknown += int(input_tokens is None or output_tokens is None)
        known_input += input_tokens if input_tokens is not None else 0
        known_output += output_tokens if output_tokens is not None else 0
        numerator = ((input_tokens if input_tokens is not None else 0)
                     * study["input_usd_micros_per_million"]
                     + (output_tokens if output_tokens is not None else 0)
                     * study["output_usd_micros_per_million"])
        lower_cost += (numerator + 999999) // 1000000
        for direction, tokens in (("input", input_tokens), ("output", output_tokens)):
            limit = study["max_" + direction + "_tokens"]
            if tokens is not None and tokens > limit:
                breaches.append(dict(kind=direction + "-token-budget", attempt_id=row["attempt_id"],
                                     observed=tokens, limit=limit))
    if lower_cost > study["funded_ceiling_usd_micros"]:
        breaches.append(dict(kind="funded-ceiling", observed=lower_cost,
                             limit=study["funded_ceiling_usd_micros"]))
    complete_usage = not pending and unknown == 0
    return dict(
        schema_version=1, accounting_study_sha256=digest, bank_kind=study["bank_kind"],
        provenance="trusted-host-usage-records", provider_attestation=False,
        provider_billing=False, funding_authorization=False,
        planned_attempts=len(attempts), recorded_attempts=len(ordered),
        completed_attempts=sum(r["outcome"] == "completed" for r in ordered),
        failed_attempts=sum(r["outcome"] == "failed" for r in ordered),
        complete=not pending, pending_attempt_ids=pending, unknown_usage_attempts=unknown,
        known_input_tokens=known_input, known_output_tokens=known_output,
        known_cost_lower_bound_usd_micros=lower_cost,
        cost_usd_micros=lower_cost if complete_usage else None,
        funded_ceiling_usd_micros=study["funded_ceiling_usd_micros"],
        ceiling_certified=complete_usage and not breaches, budget_breaches=breaches,
        records=ordered,
    )
