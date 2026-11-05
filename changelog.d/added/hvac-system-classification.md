- **HVAC systems' classification: supply, return or exhaust air.** An HVAC
  system's object holds the id of its `RbsHvacSystemType`, and the type
  holds Revit's `MEPSystemClassification` after its abbreviation and
  overrides pointer (1 supply air, 2 return air, 3 exhaust air). Each
  element of a system now carries `m_mep_classification_<system id>` beside
  `m_mep_system_<system id>`. On Autodesk's Snowdon Towers HVAC sample
  (Revit 2024), the RME advanced sample (Revit 2025) and the BIM4LCA
  housing's MagiCAD model (Revit 2024) every system reads as its name says,
  MagiCAD's Finnish names included.
