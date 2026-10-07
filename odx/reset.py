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
# ECUReset (0x11) services. Modelled like the Eclipse OpenSOVD Classic
# Diagnostic Adapter test container (testcontainer/odx/reset.py, Apache-2.0).

from odxtools.diaglayers.diaglayerraw import DiagLayerRaw
from odxtools.diagservice import DiagService
from odxtools.nameditemlist import NamedItemList
from odxtools.request import Request
from odxtools.response import Response, ResponseType

from helper import (
    derived_id,
    functional_class_ref,
    matching_request_parameter_subfunction,
    ref,
    sid_parameter_pr,
    sid_parameter_rq,
    state_transition_ref,
    subfunction_rq,
)

RESET_TYPES = {"HardReset": 0x01, "SoftReset": 0x03}


def add_reset_service(base: DiagLayerRaw, dlr: DiagLayerRaw, name: str):
    """11 <type>: a reset returns the ECU to the Default session and Locked
    security level, so the service references every transition into those
    states (base variant state charts)."""
    request = Request(
        odx_id=derived_id(dlr, f"RQ.RQ_{name}"),
        short_name=f"RQ_{name}",
        parameters=NamedItemList(
            [sid_parameter_rq(0x11), subfunction_rq(RESET_TYPES[name], "ResetType")]
        ),
    )
    dlr.requests.append(request)

    response = Response(
        odx_id=derived_id(dlr, f"PR.PR_{name}"),
        short_name=f"PR_{name}",
        response_type=ResponseType.POSITIVE,
        parameters=NamedItemList(
            [
                sid_parameter_pr(0x11 + 0x40),
                matching_request_parameter_subfunction("ResetType"),
            ]
        ),
    )
    dlr.positive_responses.append(response)

    transitions = [
        stt
        for stt in base.state_charts["Session"].state_transitions
        if stt.target_snref == "Default"
    ] + [
        stt
        for stt in base.state_charts["SecurityAccess"].state_transitions
        if stt.target_snref == "Locked"
    ]

    dlr.diag_comms_raw.append(
        DiagService(
            odx_id=derived_id(dlr, f"DC.{name}"),
            short_name=name,
            request_ref=ref(request),
            pos_response_refs=[ref(response)],
            state_transition_refs=[state_transition_ref(stt) for stt in transitions],
            functional_class_refs=[functional_class_ref(base, "EcuReset")],
        )
    )
