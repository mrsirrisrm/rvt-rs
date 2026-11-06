- **Flexible ducts, and the ends of every straight duct.** Flex ducts
  (`OST_FlexDuctCurves`) are recovered as `FlexDuct`, each in its system with
  its record box, and their connector pairs are read. A straight duct across
  the plan had no ends where a second connector entry lay 0.001 ft off one of
  them, which read as two cylinders of one length; two whose ends the box
  cannot tell apart now count as one. A square or round duct no longer than it
  is wide, whose box matches more than one axis, now takes its ends from its
  connector entries too. All 1053 straights on Autodesk's Snowdon Towers HVAC
  sample (Revit 2024) have their ends, up from 1032.
