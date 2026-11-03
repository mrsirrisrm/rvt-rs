- **Duct accessories.** The walk emits `OST_DuctAccessory` instances as
  `DuctAccessory` elements, with the joins their connectors record, and
  the IFC export writes them as `IfcFlowController`, as Revit's IFC2x3
  export does. On the BIM4LCA housing's HVAC model (Revit 2024) the 162
  are its dampers, silencers, filters and accessory fittings: with the
  duct fittings, exactly the `Tag`s of its export's `IfcDuctFitting`,
  `IfcDamper`, `IfcDuctSilencer` and `IfcFilter`. Without them the duct
  network broke at every damper.
