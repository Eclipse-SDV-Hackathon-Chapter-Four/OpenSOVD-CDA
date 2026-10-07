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
# Generates AZ3166.pdx (ODX 2.2.0) for the AZ3166 ECU simulator (FLXC1000 fork) from
# docs/diagnostics.md. Approach and layout follow the Eclipse OpenSOVD Classic
# Diagnostic Adapter test container (testcontainer/odx/generate.py,
# Apache-2.0): odxtools builds the database in Python, the PDX is then
# converted to MDD with eclipse-opensovd/odx-converter for the CDA.
#
# Layers:
#   BV AZ3166          services common to both variants
#   EV AZ3166_Boot    variant ID FF 00 00 (DID F100)
#   EV AZ3166_App    variant ID 00 01 01 (DID F100)

import os
import sys

import odxtools
from odxtools.database import Database
from odxtools.diagdatadictionaryspec import DiagDataDictionarySpec
from odxtools.diaglayercontainer import DiagLayerContainer
from odxtools.diaglayers.basevariant import BaseVariant
from odxtools.diaglayers.basevariantraw import BaseVariantRaw
from odxtools.diaglayers.diaglayertype import DiagLayerType
from odxtools.diaglayers.ecuvariant import EcuVariant
from odxtools.diaglayers.ecuvariantraw import EcuVariantRaw
from odxtools.ecuvariantpattern import EcuVariantPattern
from odxtools.matchingparameter import MatchingParameter
from odxtools.odxlink import DocType, OdxDocFragment, OdxLinkId
from odxtools.parentref import ParentRef

from comparams import generate_comparam_refs
from dids import add_app_dids, add_common_dids
from dtc_services import add_dtc_clear_services, add_dtc_read_services
from helper import ref
from metadata import (
    add_additional_audiences,
    add_admin_data,
    add_company_datas,
    add_functional_classes,
)
from reset import add_reset_service
from routines import add_routine_control_services
from security_access import add_security_access_services, add_state_chart_security_access
from sessions import add_session_service, add_state_chart_session
from shared import add_common_datatypes, add_tester_present_service
from transferdata import add_transfer_services
from units import add_units

SCRIPT_DIR = os.path.dirname(os.path.realpath(__file__))

ECU_NAME = "AZ3166"
LOGICAL_ADDRESS = 0x1001
# The ECU is its own DoIP entity, so the gateway address is its logical address.
GATEWAY_ADDRESS = 0x1001
FUNCTIONAL_ADDRESS = 0xFFFF
TESTER_ADDRESS = 0x0E00
P2_MAX_US = 50_000
P2_STAR_US = 5_000_000

BOOT_VARIANT = ("Boot", 0xFF0000)
APP_VARIANT = ("App", 0x000101)

BASE_ODX_FILES = (
    "base/ISO_13400_2.odx-cs",
    "base/ISO_14229_5.odx-cs",
    "base/ISO_14229_5_on_ISO_13400_2.odx-c",
    "base/UDS_Ethernet_DoIP.odx-d",
    "base/UDS_Ethernet_DoIP_DOBT.odx-d",
)
PROTOCOL_LAYERS = ("UDS_Ethernet_DoIP", "UDS_Ethernet_DoIP_DOBT")


def protocol_parent_ref(name: str) -> ParentRef:
    return ParentRef(
        layer_ref=ref(
            OdxLinkId(
                local_id=f"PROTO.{name}",
                doc_fragments=(OdxDocFragment(doc_name=name, doc_type=DocType.CONTAINER),),
            )
        )
    )


def add_base_variant(dlc: DiagLayerContainer, database: Database) -> BaseVariantRaw:
    base = BaseVariantRaw(
        odx_id=OdxLinkId(local_id=f"BV.{ECU_NAME}", doc_fragments=dlc.odx_id.doc_fragments),
        short_name=ECU_NAME,
        long_name="AZ3166 ECU simulator (MXCHIP AZ3166 IoT DevKit)",
        variant_type=DiagLayerType.BASE_VARIANT,
        comparam_refs=generate_comparam_refs(
            database=database,
            ecu_name=ECU_NAME,
            logical_address=LOGICAL_ADDRESS,
            gateway_address=GATEWAY_ADDRESS,
            functional_address=FUNCTIONAL_ADDRESS,
            tester_address=TESTER_ADDRESS,
            p2_max_us=P2_MAX_US,
            p2_star_us=P2_STAR_US,
        ),
        parent_refs=[protocol_parent_ref(p) for p in PROTOCOL_LAYERS],
        diag_data_dictionary_spec=DiagDataDictionarySpec(),
    )

    add_functional_classes(base)
    add_units(base)
    add_common_datatypes(base)
    # State charts live on the base variant so that the CDA can seed the
    # default session / security state before variant detection.
    add_state_chart_session(base)
    add_state_chart_security_access(base)

    # Services common to App and Boot
    # 10 01, 10 03
    add_session_service(base, base, "Default", ["Default", "Programming", "Extended"])
    add_session_service(base, base, "Extended", ["Default", "Programming", "Extended"])
    # 11 01
    add_reset_service(base, base, "HardReset")
    # 22 F100 / F186 / F18C / F195
    add_common_dids(base)
    # 3E 00
    add_tester_present_service(base, base)

    dlc.base_variants.append(BaseVariant(diag_layer_raw=base))
    return base


def new_ecu_variant(
    dlc: DiagLayerContainer, base: BaseVariantRaw, variant: tuple[str, int]
) -> EcuVariantRaw:
    name = f"{ECU_NAME}_{variant[0]}"
    return EcuVariantRaw(
        odx_id=OdxLinkId(local_id=f"EV.{ECU_NAME}.{name}", doc_fragments=dlc.odx_id.doc_fragments),
        short_name=name,
        variant_type=DiagLayerType.ECU_VARIANT,
        ecu_variant_patterns=[
            EcuVariantPattern(
                matching_parameters=[
                    MatchingParameter(
                        expected_value=str(variant[1]),
                        diag_comm_snref="Identification_Read",
                        out_param_if_snref="Identification",
                    )
                ]
            )
        ],
        parent_refs=[ParentRef(layer_ref=ref(base))],
        diag_data_dictionary_spec=DiagDataDictionarySpec(),
    )


def add_boot_variant(dlc: DiagLayerContainer, base: BaseVariantRaw):
    boot = new_ecu_variant(dlc, base, BOOT_VARIANT)
    # 10 02
    add_session_service(base, boot, "Programming", ["Default", "Programming", "Extended"])
    # 11 03
    add_reset_service(base, boot, "SoftReset")
    # 27 03 / 27 04 in sessions 02, 03
    add_security_access_services(base, boot, sessions=["Programming", "Extended"])
    # 34 / 36 / 37 in session 02 with security level 03
    add_transfer_services(base, boot, session="Programming", security="Level_3")
    dlc.ecu_variants.append(EcuVariant(diag_layer_raw=boot))


def add_app_variant(dlc: DiagLayerContainer, base: BaseVariantRaw):
    app = new_ecu_variant(dlc, base, APP_VARIANT)
    # 22 / 2E App DIDs
    add_app_dids(base, app)
    # 19 02
    add_dtc_read_services(base, app)
    # 14
    add_dtc_clear_services(base, app)
    # 31 xx 10 01 in session 03
    add_routine_control_services(base, app)
    dlc.ecu_variants.append(EcuVariant(diag_layer_raw=app))


def generate(output: str):
    print(f"Generating {ECU_NAME} -> {output}")
    database = Database()
    database.short_name = ECU_NAME

    for odx_filename in BASE_ODX_FILES:
        database.add_odx_file(os.path.join(SCRIPT_DIR, odx_filename))
    database.refresh()

    dlc = DiagLayerContainer(
        odx_id=OdxLinkId(
            f"DLC.{ECU_NAME}", doc_fragments=(OdxDocFragment(ECU_NAME, DocType.CONTAINER),)
        ),
        short_name=ECU_NAME,
    )
    add_admin_data(dlc)
    add_company_datas(dlc)
    add_additional_audiences(dlc)

    base = add_base_variant(dlc, database)
    add_boot_variant(dlc, base)
    add_app_variant(dlc, base)

    database.diag_layer_containers.append(dlc)
    database.refresh()
    odxtools.write_pdx_file(output, database)


if __name__ == "__main__":
    generate(sys.argv[1] if len(sys.argv) > 1 else os.path.join(SCRIPT_DIR, f"{ECU_NAME}.pdx"))
