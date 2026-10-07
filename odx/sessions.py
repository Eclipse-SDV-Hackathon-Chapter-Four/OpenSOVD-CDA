# SPDX-FileCopyrightText: 2026 Copyright (c) Contributors to the Eclipse Foundation
#
# See the NOTICE file(s) distributed with this work for additional
# information regarding copyright ownership.
#
# This program and the accompanying materials are made available under the
# terms of the Apache License Version 2.0 which is available at
# https://www.apache.org/licenses/LICENSE-2.0
#
# SPDX-License-Identifier: Apache-2.0
# This file is 100% AI-generated (Claude Code, Claude Opus 5.5).
#
# Session state chart and DiagnosticSessionControl (0x10) services.
# Modelled like the Eclipse OpenSOVD Classic Diagnostic Adapter test container
# (testcontainer/odx/sessions.py, Apache-2.0).

from odxtools.diaglayers.diaglayerraw import DiagLayerRaw
from odxtools.diagservice import DiagService
from odxtools.nameditemlist import NamedItemList
from odxtools.request import Request
from odxtools.response import Response, ResponseType
from odxtools.state import State
from odxtools.statechart import StateChart
from odxtools.statetransition import StateTransition

from helper import (
    derived_id,
    find_state_transition,
    functional_class_ref,
    matching_request_parameter_subfunction,
    ref,
    sid_parameter_pr,
    sid_parameter_rq,
    state_transition_ref,
    subfunction_rq,
)

# docs/diagnostics.md "Sessions"
SESSIONS = {"Default": 0x01, "Programming": 0x02, "Extended": 0x03}


def add_state_chart_session(dlr: DiagLayerRaw):
    """Session state chart, owned by the base variant.

    It contains all three sessions so that the CDA can seed the default
    states before variant detection. The App variant simply has no service
    that enters "Programming".
    """
    odx_id = derived_id(dlr, "SC.Session")
    names = list(SESSIONS)
    dlr.state_charts.append(
        StateChart(
            odx_id=odx_id,
            short_name="Session",
            semantic="SESSION",
            start_state_snref="Default",
            states=NamedItemList(
                [State(odx_id=derived_id(odx_id, f"ST.{n}"), short_name=n) for n in names]
            ),
            state_transitions=[
                StateTransition(
                    odx_id=derived_id(odx_id, f"STT.{src}_{dst}"),
                    short_name=f"{src}_{dst}",
                    source_snref=src,
                    target_snref=dst,
                )
                for src in names
                for dst in names
            ],
        )
    )


def add_session_service(
    base: DiagLayerRaw,
    dlr: DiagLayerRaw,
    target: str,
    from_states: list[str],
):
    """10 <session>: <target>_Start, added to ``dlr``.

    ``base`` owns the state chart and functional classes.
    """
    service_name = f"{target}_Start"

    request = Request(
        odx_id=derived_id(dlr, f"RQ.RQ_{service_name}"),
        short_name=f"RQ_{service_name}",
        parameters=NamedItemList(
            [sid_parameter_rq(0x10), subfunction_rq(SESSIONS[target], "SessionType")]
        ),
    )
    dlr.requests.append(request)

    response = Response(
        response_type=ResponseType.POSITIVE,
        odx_id=derived_id(dlr, f"PR.PR_{service_name}"),
        short_name=f"PR_{service_name}",
        parameters=NamedItemList(
            [
                sid_parameter_pr(0x10 + 0x40),
                matching_request_parameter_subfunction("SessionType"),
            ]
        ),
    )
    dlr.positive_responses.append(response)

    dlr.diag_comms_raw.append(
        DiagService(
            odx_id=derived_id(dlr, f"DC.{service_name}"),
            short_name=service_name,
            semantic="SESSION",
            functional_class_refs=[functional_class_ref(base, "Session")],
            request_ref=ref(request),
            pos_response_refs=[ref(response)],
            state_transition_refs=[
                state_transition_ref(find_state_transition(base, "Session", f"{src}_{target}"))
                for src in from_states
            ],
        )
    )
