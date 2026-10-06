# SPDX-License-Identifier: Apache-2.0
# This file is 100% AI-generated (Claude Code, Claude Opus 5.5).
#
# SecurityAccess state chart and services (0x27). Modelled like the Eclipse
# OpenSOVD Classic Diagnostic Adapter test container
# (testcontainer/odx/security_access.py, Apache-2.0).
#
# docs/diagnostics.md: Boot only, sessions 02/03, level 03 (seed) / 04 (key),
# 4-byte seed and key, key = seed XOR 0xDEADBEEF.

from odxtools.dataobjectproperty import DataObjectProperty
from odxtools.diaglayers.diaglayerraw import DiagLayerRaw
from odxtools.diagservice import DiagService
from odxtools.nameditemlist import NamedItemList
from odxtools.odxtypes import DataType
from odxtools.parameters.valueparameter import ValueParameter
from odxtools.radix import Radix
from odxtools.request import Request
from odxtools.response import Response, ResponseType
from odxtools.state import State
from odxtools.statechart import StateChart
from odxtools.statetransition import StateTransition

from helper import (
    derived_id,
    find_state,
    find_state_transition,
    functional_class_ref,
    identical_dop,
    matching_request_parameter_subfunction,
    pre_condition_state_ref,
    ref,
    sid_parameter_pr,
    sid_parameter_rq,
    state_transition_ref,
    subfunction_rq,
)

SECURITY_LEVELS = {"Level_3": 0x03}
SEED_KEY_LENGTH = 4


def add_state_chart_security_access(dlr: DiagLayerRaw):
    """SecurityAccess state chart, owned by the base variant."""
    states = ["Locked", *SECURITY_LEVELS]
    transitions = [("Locked", "Locked")]
    for level in SECURITY_LEVELS:
        transitions += [("Locked", level), (level, "Locked")]

    odx_id = derived_id(dlr, "SC.SecurityAccess")
    dlr.state_charts.append(
        StateChart(
            odx_id=odx_id,
            short_name="SecurityAccess",
            semantic="SECURITY",
            start_state_snref="Locked",
            states=NamedItemList(
                [State(odx_id=derived_id(odx_id, f"ST.{n}"), short_name=n) for n in states]
            ),
            state_transitions=[
                StateTransition(
                    odx_id=derived_id(odx_id, f"STT.{src}_{dst}"),
                    short_name=f"{src}_{dst}",
                    source_snref=src,
                    target_snref=dst,
                )
                for src, dst in transitions
            ],
        )
    )


def _session_preconditions(base: DiagLayerRaw, sessions: list[str]):
    return [pre_condition_state_ref(find_state(base, "Session", s)) for s in sessions]


def add_request_seed_service(
    base: DiagLayerRaw,
    dlr: DiagLayerRaw,
    level_name: str,
    seed_dop: DataObjectProperty,
    sessions: list[str],
):
    level = SECURITY_LEVELS[level_name]
    name = f"RequestSeed_{level_name}"

    request = Request(
        odx_id=derived_id(dlr, f"RQ.RQ_{name}"),
        short_name=f"RQ_{name}",
        parameters=NamedItemList(
            [sid_parameter_rq(0x27), subfunction_rq(level, short_name="SecurityAccessType")]
        ),
    )
    dlr.requests.append(request)

    response = Response(
        odx_id=derived_id(dlr, f"PR.PR_{name}"),
        short_name=f"PR_{name}",
        response_type=ResponseType.POSITIVE,
        parameters=NamedItemList(
            [
                sid_parameter_pr(0x27 + 0x40),
                matching_request_parameter_subfunction("SecurityAccessType"),
                ValueParameter(
                    short_name="SecuritySeed",
                    semantic="DATA",
                    byte_position=2,
                    dop_ref=ref(seed_dop),
                ),
            ]
        ),
    )
    dlr.positive_responses.append(response)

    dlr.diag_comms_raw.append(
        DiagService(
            odx_id=derived_id(dlr, f"DC.{name}"),
            short_name=name,
            request_ref=ref(request),
            pos_response_refs=[ref(response)],
            functional_class_refs=[functional_class_ref(base, "SecurityAccess")],
            pre_condition_state_refs=_session_preconditions(base, sessions),
        )
    )


def add_send_key_service(
    base: DiagLayerRaw,
    dlr: DiagLayerRaw,
    level_name: str,
    key_dop: DataObjectProperty,
    sessions: list[str],
):
    level = SECURITY_LEVELS[level_name]
    name = f"SendKey_{level_name}"

    request = Request(
        odx_id=derived_id(dlr, f"RQ.RQ_{name}"),
        short_name=f"RQ_{name}",
        parameters=NamedItemList(
            [
                sid_parameter_rq(0x27),
                subfunction_rq(level + 1, short_name="SecurityAccessType"),
                ValueParameter(
                    short_name="SecurityKey",
                    semantic="DATA",
                    byte_position=2,
                    dop_ref=ref(key_dop),
                ),
            ]
        ),
    )
    dlr.requests.append(request)

    response = Response(
        odx_id=derived_id(dlr, f"PR.PR_{name}"),
        short_name=f"PR_{name}",
        response_type=ResponseType.POSITIVE,
        parameters=NamedItemList(
            [
                sid_parameter_pr(0x27 + 0x40),
                matching_request_parameter_subfunction("SecurityAccessType"),
            ]
        ),
    )
    dlr.positive_responses.append(response)

    dlr.diag_comms_raw.append(
        DiagService(
            odx_id=derived_id(dlr, f"DC.{name}"),
            short_name=name,
            request_ref=ref(request),
            pos_response_refs=[ref(response)],
            state_transition_refs=[
                state_transition_ref(
                    find_state_transition(base, "SecurityAccess", f"Locked_{level_name}")
                )
            ],
            functional_class_refs=[functional_class_ref(base, "SecurityAccess")],
            pre_condition_state_refs=_session_preconditions(base, sessions),
        )
    )


def add_security_access_services(base: DiagLayerRaw, dlr: DiagLayerRaw, sessions: list[str]):
    seed_key_dop = identical_dop(
        dlr,
        "SecurityAccess_4ByteArray",
        DataType.A_BYTEFIELD,
        SEED_KEY_LENGTH * 8,
        display_radix=Radix.HEX,
    )
    dlr.diag_data_dictionary_spec.data_object_props.append(seed_key_dop)

    # 27 03 RequestSeed_Level_3
    add_request_seed_service(base, dlr, "Level_3", seed_key_dop, sessions)
    # 27 04 SendKey_Level_3
    add_send_key_service(base, dlr, "Level_3", seed_key_dop, sessions)
