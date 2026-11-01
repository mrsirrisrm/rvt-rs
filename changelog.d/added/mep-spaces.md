- **MEP spaces, with their number, name, level and box.**
  `rvt::elements::spaces::spaces_lossy` reads the placed `OST_MEPSpaces`
  element records by the instance rule rooms use, their number and name
  from the room parameter entries, and their bounding box in feet. An MEP
  model's spaces are what its systems serve; the walk does not emit them.
  On Autodesk's Snowdon Towers HVAC sample (Revit 2024) it reads 67 spaces,
  66 named, and 429 of the model's 509 air terminals stand inside one; the
  rest stand where the model has no space or above a space's top. Not yet
  held to an IFC export's `IfcSpace`s.
