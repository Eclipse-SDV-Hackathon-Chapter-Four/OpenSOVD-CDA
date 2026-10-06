# SPDX-License-Identifier: Apache-2.0
# This file is 100% AI-generated (Claude Code, Claude Opus 5.5).
#
# Physical dimensions and units used by the AZ3166 data identifiers.
# Unit definitions follow the Eclipse OpenSOVD Classic Diagnostic Adapter test
# container (testcontainer/odx/shared_units.py, Apache-2.0); only the units
# needed by docs/diagnostics.md are defined here.

from odxtools.diaglayers.diaglayerraw import DiagLayerRaw
from odxtools.nameditemlist import NamedItemList
from odxtools.physicaldimension import PhysicalDimension
from odxtools.unit import Unit
from odxtools.unitspec import UnitSpec

from helper import derived_id, ref

# Standard gravity, used to express "mg" (milli-g) in SI units.
_G_N = 9.80665


def add_units(dlr: DiagLayerRaw):
    def pdim(name: str, **exps) -> PhysicalDimension:
        return PhysicalDimension(odx_id=derived_id(dlr, f"PDIM.{name}"), short_name=name, **exps)

    pdim_raw = pdim("Raw")
    pdim_time = pdim("Time", time_exp=1)
    pdim_temperature = pdim("ThermodynamicTemperature", temperature_exp=1)
    pdim_pressure = pdim("Pressure", length_exp=-1, mass_exp=1, time_exp=-2)
    pdim_acceleration = pdim("Acceleration", length_exp=1, time_exp=-2)
    pdim_angular_velocity = pdim("AngularVelocity", time_exp=-1)
    # Magnetic flux density (T = kg / (A * s^2))
    pdim_magnetic_flux_density = pdim(
        "MagneticFluxDensity", mass_exp=1, time_exp=-2, current_exp=-1
    )

    def unit(name: str, display: str, factor: float, offset: float, dim: PhysicalDimension):
        return Unit(
            odx_id=derived_id(dlr, f"UNIT.{name}"),
            short_name=name,
            display_name=display,
            factor_si_to_unit=factor,
            offset_si_to_unit=offset,
            physical_dimension_ref=ref(dim),
        )

    units = [
        unit("Second", "s", 1, 0, pdim_time),
        unit("DegreeCelsius", "°C", 1, 273.15, pdim_temperature),
        unit("PerCentRelativeHumidity", "%RH", 0.01, 0, pdim_raw),
        unit("HectoPascal", "hPa", 100, 0, pdim_pressure),
        unit("MilliG", "mg", _G_N / 1000, 0, pdim_acceleration),
        unit("DegreePerSecond", "dps", 0.017453292519943296, 0, pdim_angular_velocity),
        unit("MilliGauss", "mG", 1e-7, 0, pdim_magnetic_flux_density),
    ]

    dlr.diag_data_dictionary_spec.unit_spec = UnitSpec(
        physical_dimensions=NamedItemList(
            [
                pdim_raw,
                pdim_time,
                pdim_temperature,
                pdim_pressure,
                pdim_acceleration,
                pdim_angular_velocity,
                pdim_magnetic_flux_density,
            ]
        ),
        units=NamedItemList(units),
    )
