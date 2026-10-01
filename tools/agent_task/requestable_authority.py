"""Independent finite rights projection and shared scripted gate consumer.

No grants are implemented here; actual backends make every authorization decision.
"""

import base64
import hashlib
import json
import time

RIGHTS = ("read", "stage", "validate", "commit", "observe")
RIGHT_TUPLES = {
    "read": ["config.read.grant", "schema.read.grant", "notes.read.grant"],
    "stage": ["candidate.write"],
    "validate": ["validator.execute"],
    "commit": ["output.commit", "receipt.observe"],
    "observe": ["state.observe", "subscription.observe"],
}
CATALOG_HASH = hashlib.sha256(
    json.dumps(RIGHT_TUPLES, sort_keys=True, separators=(",", ":")).encode()
).hexdigest()


def matrix_summary(rows, policy):
    if type(policy) is not int or policy not in (17, 31) or len(rows) != 31:
        raise ValueError("matrix coverage")
    seen, issued = set(), []
    for row in rows:
        if not isinstance(row, dict) or set(row) != {
            "mask",
            "requested",
            "lifetime_ms",
            "status",
            "issued",
        }:
            raise ValueError("matrix fields")
        mask = row["mask"]
        if type(mask) is not int or not 1 <= mask <= 31 or mask in seen:
            raise ValueError("matrix mask")
        seen.add(mask)
        rights = [r for n, r in enumerate(RIGHTS) if mask & (1 << n)]
        allowed = mask & ~policy == 0
        if (
            row["requested"] != rights
            or type(row["lifetime_ms"]) is not int
            or row["lifetime_ms"] != 60000
            or row["status"] != ("ok" if allowed else "denied")
            or row["issued"] != (rights if allowed else None)
        ):
            raise ValueError("matrix decision or redaction")
        if allowed:
            issued.append(mask)
    projection = sorted(
        {
            t
            for mask in issued
            for n, r in enumerate(RIGHTS)
            if mask & (1 << n)
            for t in RIGHT_TUPLES[r]
        }
    )
    return dict(
        catalog_sha256=CATALOG_HASH,
        policy_rights=policy,
        issued_masks=sorted(issued),
        denied_masks=sorted(seen - set(issued)),
        requestable_projection=projection,
        data_effects_from_issuance=[],
        whole_authority_relation="unknown",
        continuous_envelope_certified=False,
    )


def consume(call, bootstrap, kind, masks=(), policy=31, extra=None):
    started = time.monotonic()
    trace = []

    def invoke(case, op, expected="ok", version=1, **fields):
        request = dict(
            schema_version=version,
            request_id=str(len(trace) + 1),
            call=dict(operation=op, **fields),
        )
        reply = call(request)
        assert (
            reply["schema_version"] == version
            and reply["request_id"] == request["request_id"]
        )
        expected = (expected,) if isinstance(expected, str) else expected
        assert reply["status"] in expected, (case, reply)
        if reply["status"] != "ok":
            assert reply["result"] is None, (case, reply)
        trace.append(
            dict(
                case=case,
                request=request,
                response=reply,
                elapsed_ms=round((time.monotonic() - started) * 1000),
            )
        )
        return reply

    def grant(case, rights, life=60000, expected="ok"):
        return invoke(
            case,
            "request_grant",
            expected,
            policy_cap=bootstrap["policy_cap"],
            task_id=bootstrap["task_id"],
            resource=bootstrap["resources"][0]["resource"],
            rights=rights,
            lifetime_ms=life,
        )

    def cap(reply):
        return reply["result"]["task_cap"]

    def prepare():
        admin = cap(grant("prepare_admin", list(RIGHTS)))
        state = invoke("prepare_state", "get_task_state", task_cap=admin)["result"][
            "state"
        ]
        data = invoke(
            "prepare_schema",
            "read_input",
            task_cap=admin,
            resource=bootstrap["resources"][1]["resource"],
        )["result"]
        blob = data["bytes_base64"]
        candidate = invoke(
            "prepare_candidate", "stage_candidate", task_cap=admin, bytes_base64=blob
        )["result"]["candidate_cap"]
        return admin, state, blob, candidate

    rows = []
    if kind == "matrix":
        if policy == 31:
            admin, state, blob, candidate = prepare()
        for mask in masks:
            rights = [r for n, r in enumerate(RIGHTS) if mask & (1 << n)]
            reply = grant(
                "mask_" + str(mask),
                rights,
                expected="ok" if mask & ~policy == 0 else "denied",
            )
            rows.append(
                dict(
                    mask=mask,
                    requested=rights,
                    lifetime_ms=60000,
                    status=reply["status"],
                    issued=reply["result"]["rights"] if reply["result"] else None,
                )
            )
            if policy != 31:
                if reply["result"] and "read" in rights:
                    invoke(
                        f"{mask}_attenuated_read",
                        "read_input",
                        task_cap=cap(reply),
                        resource=bootstrap["resources"][0]["resource"],
                    )
                if reply["result"] and "observe" in rights:
                    observed = invoke(
                        f"{mask}_attenuated_state",
                        "get_task_state",
                        task_cap=cap(reply),
                    )["result"]["state"]
                    assert observed["rights"] == rights
                continue
            holder = cap(reply)

            def expectation(right, yes="ok"):
                return yes if right in rights else "denied"

            for n, resource in enumerate(bootstrap["resources"]):
                invoke(
                    f"{mask}_read_{n}",
                    "read_input",
                    expectation("read"),
                    task_cap=holder,
                    resource=resource["resource"],
                )
            invoke(
                f"{mask}_stage",
                "stage_candidate",
                expectation("stage"),
                task_cap=holder,
                bytes_base64=blob,
            )
            invoke(
                f"{mask}_commit_precondition",
                "commit_candidate",
                expectation("commit", "validation_failed"),
                task_cap=holder,
                candidate_cap=candidate,
                expected_revision=state["revision"],
                expected_content_id=state["content_id"],
            )
            invoke(
                f"{mask}_receipt",
                "get_receipt",
                expectation("commit", "not_found"),
                task_cap=holder,
                commit_request_id="999",
            )
            observed = invoke(
                f"{mask}_state",
                "get_task_state",
                expectation("observe"),
                task_cap=holder,
            )
            if observed["result"]:
                assert observed["result"]["state"]["rights"] == rights
            sub = invoke(
                f"{mask}_subscribe",
                "subscribe_task",
                expectation("observe"),
                version=2,
                task_cap=holder,
                event_types=["output_changed"],
            )
            if sub["result"]:
                subscription = sub["result"]["subscription_cap"]
                invoke(
                    f"{mask}_poll",
                    "poll_task",
                    version=2,
                    task_cap=holder,
                    subscription_cap=subscription,
                )
                invoke(
                    f"{mask}_cancel",
                    "unsubscribe_task",
                    version=2,
                    task_cap=holder,
                    subscription_cap=subscription,
                )
        if not masks or masks[0] == 1:
            grant("policy_lifetime_ceiling", ["read"], 60001, "denied")
    elif kind == "witness":
        admin, state, blob, candidate = prepare()
        holders = {r: cap(grant("single_" + r, [r])) for r in RIGHTS}
        invoke(
            "single_read_effect",
            "read_input",
            task_cap=holders["read"],
            resource=bootstrap["resources"][0]["resource"],
        )
        staged = invoke(
            "single_stage_effect",
            "stage_candidate",
            task_cap=holders["stage"],
            bytes_base64=blob,
        )["result"]
        candidate = staged["candidate_cap"]
        assert (
            staged["content_id"]
            == "sha256:"
            + hashlib.sha256(base64.b64decode(blob, validate=True)).hexdigest()
        )
        pins = state["validator_id"]
        validation = invoke(
            "single_validate_effect",
            "validate_candidate",
            task_cap=holders["validate"],
            candidate_cap=candidate,
            validator_id=pins,
        )["result"]
        assert validation["outcome"] == "valid", validation
        commit = invoke(
            "single_commit_effect",
            "commit_candidate",
            task_cap=holders["commit"],
            candidate_cap=candidate,
            expected_revision=state["revision"],
            expected_content_id=state["content_id"],
        )
        commit_id = commit["request_id"]
        receipt = invoke(
            "single_receipt_effect",
            "get_receipt",
            task_cap=holders["commit"],
            commit_request_id=commit_id,
        )["result"]
        assert (
            receipt["revision"] == "1" and receipt["content_id"] == staged["content_id"]
        )
        observed = invoke(
            "single_observe_effect", "get_task_state", task_cap=holders["observe"]
        )["result"]["state"]
        assert observed["revision"] == "1" and observed["rights"] == ["observe"]
    elif kind == "lifetime":
        admin, state, blob, candidate = prepare()
        short = grant("short_grant", list(RIGHTS), 500)
        holder = cap(short)
        expires = int(short["result"]["expires_at_ms"])
        now = invoke("short_state", "get_task_state", task_cap=holder)["result"][
            "state"
        ]
        assert int(now["now_ms"]) < expires
        retained = invoke(
            "short_read",
            "read_input",
            task_cap=holder,
            resource=bootstrap["resources"][0]["resource"],
        )["result"]["bytes_base64"]
        if extra:
            extra("before_expiry")
        sub = invoke(
            "short_subscribe",
            "subscribe_task",
            version=2,
            task_cap=holder,
            event_types=["output_changed"],
        )["result"]["subscription_cap"]
        time.sleep(max(0, (expires - int(now["now_ms"])) / 1000) + 0.05)
        clock = invoke("expiry_clock", "get_task_state", task_cap=admin)["result"][
            "state"
        ]
        assert int(clock["now_ms"]) >= expires
        if extra:
            extra("after_expiry")

        def blocked(prefix, token, subscription):
            expected = ("denied", "expired")
            invoke(
                prefix + "_read",
                "read_input",
                expected,
                task_cap=token,
                resource=bootstrap["resources"][0]["resource"],
            )
            invoke(
                prefix + "_stage",
                "stage_candidate",
                expected,
                task_cap=token,
                bytes_base64=blob,
            )
            invoke(
                prefix + "_validate",
                "validate_candidate",
                expected,
                task_cap=token,
                candidate_cap=candidate,
                validator_id=state["validator_id"],
            )
            invoke(
                prefix + "_commit",
                "commit_candidate",
                expected,
                task_cap=token,
                candidate_cap=candidate,
                expected_revision=state["revision"],
                expected_content_id=state["content_id"],
            )
            invoke(
                prefix + "_receipt",
                "get_receipt",
                expected,
                task_cap=token,
                commit_request_id="999",
            )
            invoke(prefix + "_state", "get_task_state", expected, task_cap=token)
            invoke(
                prefix + "_subscribe",
                "subscribe_task",
                expected,
                version=2,
                task_cap=token,
                event_types=["output_changed"],
            )
            invoke(
                prefix + "_poll",
                "poll_task",
                expected,
                version=2,
                task_cap=token,
                subscription_cap=subscription,
            )
            invoke(
                prefix + "_cancel",
                "unsubscribe_task",
                expected,
                version=2,
                task_cap=token,
                subscription_cap=subscription,
            )

        blocked("expired_first", holder, sub)
        renewed = cap(grant("renewed_grant", list(RIGHTS)))
        invoke(
            "renewed_read",
            "read_input",
            task_cap=renewed,
            resource=bootstrap["resources"][0]["resource"],
        )
        invoke(
            "renewed_old_subscription",
            "poll_task",
            "not_found",
            version=2,
            task_cap=renewed,
            subscription_cap=sub,
        )
        blocked("expired_after_renewal", holder, sub)
        fresh_sub = invoke(
            "renewed_subscription",
            "subscribe_task",
            version=2,
            task_cap=renewed,
            event_types=["output_changed"],
        )["result"]["subscription_cap"]
        revoke = invoke(
            "generation_revoke",
            "revoke_grant",
            policy_cap=bootstrap["policy_cap"],
            task_cap=renewed,
        )["result"]
        assert int(revoke["generation"]) > int(short["result"]["generation"])
        blocked("revoked_renewed", renewed, fresh_sub)
        if extra:
            extra("after_revocation")
        fresh = grant("policy_after_revocation", ["read", "observe"])
        assert int(fresh["result"]["generation"]) == int(revoke["generation"])
        invoke(
            "fresh_generation_read",
            "read_input",
            task_cap=cap(fresh),
            resource=bootstrap["resources"][0]["resource"],
        )
        invoke(
            "fresh_generation_old_subscription",
            "poll_task",
            "not_found",
            version=2,
            task_cap=cap(fresh),
            subscription_cap=fresh_sub,
        )
        invoke(
            "fresh_generation_old_grant",
            "read_input",
            ("denied", "expired"),
            task_cap=renewed,
            resource=bootstrap["resources"][0]["resource"],
        )
        assert base64.b64decode(retained, validate=True)
    else:
        raise ValueError("unknown case")
    return dict(
        kind=kind,
        policy=policy,
        rows=rows,
        trace=trace,
        whole_authority_relation="unknown",
        continuous_envelope_certified=False,
    )
