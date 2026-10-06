# SPDX-License-Identifier: Apache-2.0
# This file is 100% AI-generated (Claude Code, Claude Opus 5.5).
#
# Communication parameters of the FLXC1000 base variant. Modelled like the
# Eclipse OpenSOVD Classic Diagnostic Adapter test container
# (testcontainer/odx/comparams.py, Apache-2.0): every value is emitted once per
# protocol layer the base variant inherits from.

from odxtools.comparaminstance import ComparamInstance
from odxtools.database import Database

from helper import ref

# Protocol layers referenced by the base variant (see base/*.odx-d).
PROTOCOLS = ("UDS_Ethernet_DoIP", "UDS_Ethernet_DoIP_DOBT")


def generate_comparam_refs(
    database: Database,
    ecu_name: str,
    logical_address: int,
    gateway_address: int,
    functional_address: int,
    tester_address: int,
    p2_max_us: int,
    p2_star_us: int,
) -> list[ComparamInstance]:
    doip = database.comparam_subsets["ISO_13400_2"]
    uds = database.comparam_subsets["ISO_14229_5"]

    simple_values = [
        (doip.comparams["CP_DoIPLogicalGatewayAddress"], gateway_address),
        (doip.comparams["CP_DoIPLogicalFunctionalAddress"], functional_address),
        (doip.comparams["CP_DoIPLogicalTesterAddress"], tester_address),
        (uds.comparams["CP_P2Max"], p2_max_us),
        (uds.comparams["CP_P2Star"], p2_star_us),
    ]
    resp_id_table = doip.complex_comparams["CP_UniqueRespIdTable"]

    refs: list[ComparamInstance] = []
    for protocol in PROTOCOLS:
        for spec, value in simple_values:
            refs.append(
                ComparamInstance(value=str(value), spec_ref=ref(spec), protocol_snref=protocol)
            )
        refs.append(
            ComparamInstance(
                value=[str(logical_address), str(0), ecu_name],
                spec_ref=ref(resp_id_table),
                protocol_snref=protocol,
            )
        )
    return refs
