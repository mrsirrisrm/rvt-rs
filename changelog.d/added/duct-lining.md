- **Insulation and lining, with the element each covers and its thickness.**
  `rvt::elements::insulation::coverings_lossy` reads every
  `RbsDuctInsulation`, `RbsPipeInsulation` and `RbsDuctLining` record: its
  host, the one element its connectors name; its type, `m_idType`; and its
  thickness, the `m_thickness` of `RbsInsulationLiningBase`, in feet. None
  of these is a saved parameter. On the BIM4LCA housing's HVAC model
  (Revit 2024) the 173 duct and 1,190 pipe insulation elements are the
  1,363 `IfcCovering`s of Revit's own export, each host is the element the
  export covers, and each of the 1,306 thicknesses the export writes is
  Revit's. Lining reads the same way but no measured model holds any.
