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


def host_canary_summary(records, expected_sha256, traces):
    """Validate nine named consumer observations, never adapter/model authority."""
    import errno
    import re

    arms = ("RT", "LT", "LS")
    phases = ("before_expiry", "after_expiry", "after_revocation")
    cases = dict(zip(phases, ("short_read", "expired_first_read", "revoked_renewed_read")))

    def require(condition):
        if not condition:
            raise ValueError("named host consumer canary evidence mismatch")

    require(type(expected_sha256) is str and re.fullmatch(r"[0-9a-f]{64}", expected_sha256) is not None)
    require(type(records) is list and len(records) == 9)
    require(type(traces) is dict and set(traces) == set(arms))
    seen, actors = set(), {}
    indexed = {}
    for arm in arms:
        require(type(traces[arm]) is list and 0 < len(traces[arm]) <= 64)
        by_case = {}
        for item in traces[arm]:
            require(type(item) is dict and type(item.get("case")) is str
                    and item["case"] not in by_case)
            by_case[item["case"]] = item
        indexed[arm] = by_case

    def witness(arm, case, operation, statuses):
        item = indexed[arm].get(case)
        require(type(item) is dict and type(item.get("request")) is dict
                and type(item.get("response")) is dict)
        request, response = item["request"], item["response"]
        require(type(request.get("schema_version")) is int and request["schema_version"] == 1
                and type(response.get("schema_version")) is int and response["schema_version"] == 1
                and type(request.get("request_id")) is str and request["request_id"] == response.get("request_id")
                and type(request.get("call")) is dict and request["call"].get("operation") == operation
                and response.get("status") in statuses)
        require(type(response.get("result")) is dict if response["status"] == "ok"
                else response.get("result") is None)
        return request, response["result"]

    for row in records:
        require(type(row) is dict and set(row) == {
            "schema_version", "arm", "phase", "actor", "backend_case", "outcome",
            "content_sha256", "denial_errno",
        })
        require(type(row["schema_version"]) is int and row["schema_version"] == 1
                and type(row["arm"]) is str and row["arm"] in arms
                and type(row["phase"]) is str and row["phase"] in phases)
        arm, phase = row["arm"], row["phase"]
        require((arm, phase) not in seen and row["backend_case"] == cases[phase])
        seen.add((arm, phase))
        actor = row["actor"]
        require(type(actor) is dict and set(actor) == {"role", "pid", "uid", "gid", "namespaces"})
        require(actor["role"] == ("contained-python-shell-consumer" if arm == "LS"
                                  else "trusted-python-evaluator-host-consumer"))
        for key in ("pid", "uid", "gid"):
            require(type(actor[key]) is int and 0 <= actor[key] <= (1 << 63) - 1)
        require(actor["pid"] > 0)
        namespaces = actor["namespaces"]
        require(type(namespaces) is dict and set(namespaces) == {"mnt", "pid", "net"})
        for kind, identity in namespaces.items():
            require(type(identity) is str and re.fullmatch(kind + r":\[[0-9]+\]", identity) is not None)
        if arm in actors:
            require(actors[arm] == actor)
        else:
            actors[arm] = actor
        if arm == "LS":
            require(row["outcome"] == "blocked" and row["content_sha256"] is None
                    and type(row["denial_errno"]) is int
                    and row["denial_errno"] in (errno.ENOENT, errno.EACCES, errno.EPERM))
        else:
            require(row["outcome"] == "allowed" and row["content_sha256"] == expected_sha256
                    and row["denial_errno"] is None)
        witness(arm, cases[phase], "read_input", ("ok",) if phase == "before_expiry" else ("denied", "expired"))
    for arm in arms:
        short, _ = witness(arm, "short_read", "read_input", ("ok",))
        expired, _ = witness(arm, "expired_first_read", "read_input", ("denied", "expired"))
        revoked, _ = witness(arm, "revoked_renewed_read", "read_input", ("denied", "expired"))
        renewed_read, _ = witness(arm, "renewed_read", "read_input", ("ok",))
        fresh_read, _ = witness(arm, "fresh_generation_read", "read_input", ("ok",))
        grant_request, grant = witness(arm, "short_grant", "request_grant", ("ok",))
        renewed_request, renewed = witness(arm, "renewed_grant", "request_grant", ("ok",))
        _, clock = witness(arm, "expiry_clock", "get_task_state", ("ok",))
        revoke_request, revoke = witness(arm, "generation_revoke", "revoke_grant", ("ok",))
        fresh_request, fresh = witness(arm, "policy_after_revocation", "request_grant", ("ok",))
        caps = [grant.get("task_cap"), renewed.get("task_cap"), fresh.get("task_cap")]
        require(all(type(cap) is str and 0 < len(cap) <= 128 for cap in caps))
        require(short["call"].get("task_cap") == expired["call"].get("task_cap") == caps[0])
        require(renewed_read["call"].get("task_cap") == revoked["call"].get("task_cap")
                == revoke_request["call"].get("task_cap") == caps[1])
        require(fresh_read["call"].get("task_cap") == caps[2])
        resources = [request["call"].get("resource")
                     for request in (grant_request, renewed_request, fresh_request)]
        require(all(type(resource) is str and re.fullmatch(r"resource:[0-9a-f]{16}", resource) is not None
                    for resource in resources) and resources[0] == resources[1] == resources[2])
        for read, resource in ((short, resources[0]), (expired, resources[0]),
                               (renewed_read, resources[1]), (revoked, resources[1]),
                               (fresh_read, resources[2])):
            require(read["call"].get("resource") == resource)
        require(type(clock.get("state")) is dict)
        numbers = [grant.get("expires_at_ms"), clock["state"].get("now_ms"),
                   grant.get("generation"), revoke.get("generation"),
                   renewed.get("generation"), fresh.get("generation")]
        require(all(type(n) is str and re.fullmatch(r"[0-9]{1,20}", n) is not None
                    and int(n) <= (1 << 64) - 1 for n in numbers))
        require(int(numbers[1]) >= int(numbers[0]) and int(numbers[3]) > int(numbers[2])
                and int(numbers[4]) == int(numbers[2]) and int(numbers[5]) == int(numbers[3]))
    return dict(
        schema_version=1, claim="named-python-consumer-unmounted-canary-lifetime-points",
        observations=9, phases=list(phases), allowed_arms=["RT", "LT"], blocked_arms=["LS"],
        canary_sha256=expected_sha256, actor_scope="actual-scripted-python-consumer",
        adapter_authority_measured=False, model_interface_authority_measured=False,
        whole_authority_relation="unknown", continuous_envelope_certified=False,
        noninterference_certified=False, full_a2_conformance=False,
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
