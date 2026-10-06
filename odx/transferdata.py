# SPDX-License-Identifier: Apache-2.0
# This file is 100% AI-generated (Claude Code, Claude Opus 5.5).
#
# Download services: RequestDownload (0x34), TransferData (0x36) and
# RequestTransferExit (0x37). Modelled like the Eclipse OpenSOVD Classic
# Diagnostic Adapter test container (testcontainer/odx/transferdata.py,
# Apache-2.0). docs/diagnostics.md "Download (Boot)": session 02 and security
# level 03 required.

from odxtools.compumethods.compucategory import CompuCategory
from odxtools.compumethods.identicalcompumethod import IdenticalCompuMethod
from odxtools.dataobjectproperty import DataObjectProperty
from odxtools.diaglayers.diaglayerraw import DiagLayerRaw
from odxtools.diagservice import DiagService
from odxtools.minmaxlengthtype import MinMaxLengthType
from odxtools.nameditemlist import NamedItemList
from odxtools.odxtypes import DataType
from odxtools.parameters.valueparameter import ValueParameter
from odxtools.physicaltype import PhysicalType
from odxtools.radix import Radix
from odxtools.request import Request
from odxtools.response import Response, ResponseType
from odxtools.termination import Termination

from helper import (
    coded_const_int_parameter,
    derived_id,
    find_dop,
    functional_class_ref,
    matching_request_parameter,
    pre_condition_state_ref,
    ref,
    sid_parameter_pr,
    sid_parameter_rq,
)

# 74 20 0F FF: maxNumberOfBlockLength = 4095 bytes including SID and BSC
MAX_NUMBER_OF_BLOCK_LENGTH = 4095


def _preconditions(base: DiagLayerRaw, session: str, security: str):
    return [
        pre_condition_state_ref(base.state_charts["Session"].states[session]),
        pre_condition_state_ref(base.state_charts["SecurityAccess"].states[security]),
    ]


def _service(base, dlr, name, request, response, session, security, semantic=None):
    dlr.requests.append(request)
    dlr.positive_responses.append(response)
    dlr.diag_comms_raw.append(
        DiagService(
            odx_id=derived_id(dlr, f"DC.{name}"),
            short_name=name,
            semantic=semantic,
            functional_class_refs=[functional_class_ref(base, "flash_download_upload")],
            request_ref=ref(request),
            pos_response_refs=[ref(response)],
            pre_condition_state_refs=_preconditions(base, session, security),
        )
    )


def add_transfer_services(base: DiagLayerRaw, dlr: DiagLayerRaw, session: str, security: str):
    uint8 = find_dop(base, "IDENTICAL_UINT_8")
    uint16 = find_dop(base, "IDENTICAL_UINT_16")
    uint32 = find_dop(base, "IDENTICAL_UINT_32")

    # ---- 34 00 44 <addr:4> <size:4> -> 74 20 <maxNumberOfBlockLength:2> ----
    request = Request(
        odx_id=derived_id(dlr, "RQ.RQ_RequestDownload"),
        short_name="RQ_RequestDownload",
        parameters=NamedItemList(
            [
                sid_parameter_rq(0x34),
                # 00: no compression, no encryption
                coded_const_int_parameter("DataFormatIdentifier", "DATA", 1, "0"),
                # 44: 4-byte memoryAddress, 4-byte memorySize
                coded_const_int_parameter("AddressAndLengthFormatIdentifier", "DATA", 2, str(0x44)),
                ValueParameter(
                    short_name="MemoryAddress",
                    semantic="DATA",
                    byte_position=3,
                    dop_ref=ref(uint32),
                ),
                ValueParameter(
                    short_name="MemorySize",
                    semantic="DATA",
                    byte_position=7,
                    dop_ref=ref(uint32),
                ),
            ]
        ),
    )
    response = Response(
        odx_id=derived_id(dlr, "PR.PR_RequestDownload"),
        short_name="PR_RequestDownload",
        response_type=ResponseType.POSITIVE,
        parameters=NamedItemList(
            [
                sid_parameter_pr(0x34 + 0x40),
                ValueParameter(
                    short_name="LengthFormatIdentifier",
                    semantic="DATA",
                    byte_position=1,
                    dop_ref=ref(uint8),
                ),
                ValueParameter(
                    short_name="MaxNumberOfBlockLength",
                    semantic="DATA",
                    byte_position=2,
                    dop_ref=ref(uint16),
                ),
            ]
        ),
    )
    _service(base, dlr, "RequestDownload", request, response, session, security, "DATA")

    # ---- 36 <bsc> <data...> -> 76 <bsc> ----
    data_dop: DataObjectProperty = DataObjectProperty(
        odx_id=derived_id(dlr, "DOP.TransferData_Data"),
        short_name="TransferData_Data",
        compu_method=IdenticalCompuMethod(
            category=CompuCategory.IDENTICAL,
            physical_type=DataType.A_BYTEFIELD,
            internal_type=DataType.A_BYTEFIELD,
        ),
        physical_type=PhysicalType(base_data_type=DataType.A_BYTEFIELD, display_radix=Radix.HEX),
        diag_coded_type=MinMaxLengthType(
            base_data_type=DataType.A_BYTEFIELD,
            min_length=1,
            max_length=MAX_NUMBER_OF_BLOCK_LENGTH - 2,
            termination=Termination.END_OF_PDU,
        ),
    )
    dlr.diag_data_dictionary_spec.data_object_props.append(data_dop)

    request = Request(
        odx_id=derived_id(dlr, "RQ.RQ_TransferData"),
        short_name="RQ_TransferData",
        parameters=NamedItemList(
            [
                sid_parameter_rq(0x36),
                ValueParameter(
                    short_name="BlockSequenceCounter",
                    semantic="DATA",
                    byte_position=1,
                    dop_ref=ref(uint8),
                ),
                ValueParameter(
                    short_name="TransferRequestParameterRecord",
                    semantic="DATA",
                    byte_position=2,
                    dop_ref=ref(data_dop),
                ),
            ]
        ),
    )
    response = Response(
        odx_id=derived_id(dlr, "PR.PR_TransferData"),
        short_name="PR_TransferData",
        response_type=ResponseType.POSITIVE,
        parameters=NamedItemList(
            [
                sid_parameter_pr(0x36 + 0x40),
                matching_request_parameter("BlockSequenceCounter", "DATA", 1),
            ]
        ),
    )
    _service(base, dlr, "TransferData", request, response, session, security)

    # ---- 37 -> 77 ----
    request = Request(
        odx_id=derived_id(dlr, "RQ.RQ_TransferExit"),
        short_name="RQ_TransferExit",
        parameters=NamedItemList([sid_parameter_rq(0x37)]),
    )
    response = Response(
        odx_id=derived_id(dlr, "PR.PR_TransferExit"),
        short_name="PR_TransferExit",
        response_type=ResponseType.POSITIVE,
        parameters=NamedItemList([sid_parameter_pr(0x37 + 0x40)]),
    )
    _service(base, dlr, "TransferExit", request, response, session, security)
