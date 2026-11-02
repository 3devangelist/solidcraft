# Why features should be able to end at sketch points

This note is the rationale for the pull request *Extrude and Hole: To/From Object can take a sketch or construction point by reference*. The technical details, tests and screenshots are in the pull request itself.

## In short

- **Top-down design:** you fix the key levels and sizes in a few layout sketches, and the features take their start and end from that layout, not from faces of other solids.
- **A point is a level:** Extrude To, Extrude Start and Hole To can now take a point of any sketch, or a construction point. The point counts as a level, measured along the normal of the profile's sketch, and it is kept by reference. Move the point, and the feature follows.
- **Built for AI:** a named point such as `Layout · p4` is something an AI assistant driving SolidCraft over MCP can read, write and check. It cannot click on a face.

## Who I am

I'm Ricardo Pereira, an industrial designer from Porto, Portugal.

- **Product development (since 2007):** I design parts for additive, subtractive and plastic manufacturing, for companies such as Winegrid and MAKE IT, mostly under NDA. I co-invented two US patents ([US11187626B2](https://patents.google.com/patent/US11187626B2)).
- **Teaching:** I teach additive manufacturing, manufacturing processes and materials science at the [Polytechnic University of Cávado and Ave (UPCA)](https://ipca.pt). My CAD course, which works from the general to the particular, is public: [CAD-Designer](https://github.com/3devangelist/CAD-Designer).
- **Tools:** I model in Fusion, Rhinoceros, SolidWorks, CATIA, AutoCAD (2D and 3D), 3ds Max and Blender. I am a founding member and former president of Problender, the Portuguese Blender association.

**I am not a programmer.** I built this change with an AI coding assistant, as an **MVP**: a working starting point that shows what I need as a designer. You know the code far better than I do, so please judge it, change it or reimplement it in the way that fits SolidCraft best.

[LinkedIn](https://www.linkedin.com/in/ricardopereiradesigner/) · [GitHub](https://github.com/3devangelist)

## Bottom-up and top-down

**Bottom-up** is the workflow that face-only inputs push you towards:

1. You sketch and extrude a first solid.
2. You sketch the next feature on a face of that solid, or extrude up to one of its faces.
3. Each feature hangs on the one before.

It is quick and fine for small parts whose shape is already known. It struggles when the design changes:

- Changing an early feature can move or break the features built on its faces.
- A key height ends up spread across an extrude distance, a face reference and a typed number. No single place holds it.
- You can only refer to what already exists. To stop at a level where nothing is built yet, you first have to build something there.

**Top-down** works the other way round:

1. You draw a **layout**: one to four sketches with the decisions that matter, such as levels, heights and where each part starts and stops.
2. You build every feature **from the layout**, not from other solids.
3. When the design changes, you edit the layout, and everything that refers to it follows.

The layout is the single source of truth, and the solids are results.

## Top-down is the older way

- **Architecture has always worked from the general to the particular:** site, then plans and sections, then details, with the scale going down at each step. Level marks are the layout. "End this wall at level +3.20" is exactly what a point reference gives an extrusion.
- **On the drawing board, mechanical design was top-down too:** the assembly drawing came first, and the part drawings were taken from it.
- **Bottom-up became the habit with feature-based 3D CAD,** where each feature is attached to a face of the one before. The tool shaped the method.

So this change does not bring a new idea. It brings back a classic way of designing that face-only inputs make hard.

## Why a face is not enough

- **The face may not exist yet.** In a layout-driven model, the level where an extrusion must stop is often only a point in a sketch. Today you have to build a helper body or plane just to have something to click.
- **The wrong thing drives the model.** If feature B stops at a face of feature A, B depends on how A is built. With a layout point, A and B both depend on the layout and not on each other.
- **A face along the extrusion has no clear answer.** A point on a wall or on the floor lies on a face that runs parallel to the extrusion. Taken as a face, "up to this face" fails with *nothing lies between the profile and the To object*. Taken as a level, the answer is clear.
- **A typed distance goes stale.** It is copied from the layout and does not follow when the layout changes.

Before writing this, we checked how SolidCraft names faces. Faces have persistent names from the feature history (`crates/doc/src/naming.rs`, `docs/book/src/dev/naming.md`), so a face reference survives, for example, a fillet added on another edge. The gain here is not that face references break. The gain is that a layout point **does not depend on any solid**: it exists before the solids, later features never split it, and only the user moves it.

## What a point means

A point is a **level**, measured along the normal of the profile's sketch plane:

- **Extrude To:** the extrusion ends at the plane, parallel to the profile, that passes through the point.
- **Extrude Start:** the extrusion starts at that plane.
- **Hole To:** the hole bottom is at that level. The thread depth stays as it was set.

Only the point's height above the profile's sketch plane counts. A point on the floor, on a wall or in empty space all work the same way.

The point is stored by reference: the sketch and the point id, or the construction point's name. Each recompute reads the point's current position. If the point is deleted, the feature keeps the position where it was picked and shows a warning, so the model does not fail.

## Example

A wall profile is drawn on a side plane. A layout sketch marks point 1, where the wall starts, and point 2, where it ends.

- **Before:** you type the offset and the length, or build a helper body to click. Moving point 2 changes nothing.
- **With this change:** Start = point 1 and To = point 2. Move either point and the wall follows. Other features can use the same points and stay in step.

In the same way, a cut can be bounded by the corners of a rectangle drawn on top of the part, from top to bottom or from bottom to top, without hiding or building anything extra.

## Why it matters for AI

For me, the most disruptive thing about SolidCraft is that **an AI assistant can be connected straight to it** through its commands and MCP. The assistant can read the model, create features and change parameters. Blender users have worked like this since March 2025, when the community project [BlenderMCP](https://github.com/ahujasid/blender-mcp) appeared.

Blender became the place where AI tools meet because it is free, open and scriptable. AI mesh generators such as Meshy, Tripo and Rodin all ship Blender add-ons. Parametric CAD has no open equivalent yet. SolidCraft can be that place: an AI there creates and edits real parts, with dimensions, references and history. I hope the project goes in this direction, and I want to help.

Named points make this work better:

- **An AI cannot click.** "End at the face I picked" means guessing a 3D position. "End at `Layout · p4`" is a name it can read, write and check.
- **The layout is a plan the person and the AI share.** The person fixes the levels, the AI builds against those names, and a change to the layout flows through everything the AI made.
- **The command is plain data:** `"to_ref": { "sketch": "Layout", "point": "p4" }` is easy to generate, review and replay.

## From practice

- **Master sketches in cascade.** I start with one general sketch of the assembly, seen from the top and from the front, with the overall sizes. Each component has its own master sketch, tied to the limits that the assembly sketch gives it. Change the assembly sketch and the components follow. Inside a SolidCraft part, this needs features that start and end at sketch points.
- **Reference sketches, not 3D geometry.** In tools like SolidWorks, the robust practice is to reference master sketches, never faces or edges of other parts. The SolidWorks documentation names external references and slower rebuilds as the costs of top-down design with in-context features. Sketch points avoid both.
- **Few sketches, shared.** A good layout is one to four sketches that many features use. That is less reference geometry than every part carrying its own. A point is also simple to resolve: this change reads its position and skips the target-face lookup.
- **What I see in class.** Students learn to think feature by feature. Then they struggle to find which feature to change. The real difficulty comes first, though: defining the part and seeing it in space. A layout with a few points trains exactly that.

## Other CAD tools

- **Onshape:** Extrude has an *Up to vertex* end type, which takes a selected point or mate connector ([Onshape help](https://cad.onshape.com/help/Content/PartStudio/extrude.htm)).
- **Fusion:** an Autodesk tutorial sets the *To Object* extent by clicking the end point of a sketch line ([Autodesk tutorial](https://help.autodesk.com/cloudhelp/ENU/Fusion-Model/files/GUID-8C1CCB59-87E3-4E06-A53C-7D5449673721.htm)).
- **Inventor:** it calls top-down work *skeletal modeling*, with a layout sketch as the root document ([Inventor help](https://help.autodesk.com/cloudhelp/2023/ENU/Inventor-Help/files/GUID-0435FE9D-6283-4CE0-813B-BD9B0BE5778C.htm)).

## Scope

- UI, document and engine only. There are no changes in `crates/kernel/` or `crates/geom/`.
- Nothing changes by default. Faces, vertices and distances behave exactly as before, and points are an extra option in the same inputs.
- The new fields (`from_ref`, and `to_ref` for Extrude) are optional, so older designs load and save unchanged.

## References

- Chu, D., Lyu, G., Chu, X., Shen, J. (2016). *Multi-skeleton model for top-down design of complex products.* Advances in Mechanical Engineering 8(6). https://doi.org/10.1177/1687814016651210
- Otey, J., Company, P., Contero, M., Camba, J. (2018). *Revisiting the design intent concept in the context of mechanical CAD education.* Computer-Aided Design and Applications 15(1), 47–60. https://doi.org/10.1080/16864360.2017.1353733
- González-Lluch, C., Company, P., Contero, M., Camba, J., Plumed, R. (2017). *A survey on 3D CAD model quality assurance and testing tools.* Computer-Aided Design 83, 64–79. https://doi.org/10.1016/j.cad.2016.10.003
- SolidWorks Help, *Top-Down Design.* https://help.solidworks.com/2021/english/SolidWorks/sldworks/c_Top-Down_Design_Overview.htm
- Autodesk Inventor Help, *Use skeletal modeling.* https://help.autodesk.com/cloudhelp/2023/ENU/Inventor-Help/files/GUID-0435FE9D-6283-4CE0-813B-BD9B0BE5778C.htm
- BlenderMCP (Siddharth Ahuja), first published March 2025. https://github.com/ahujasid/blender-mcp

## Appendix: next feature idea, showing the name of what is under the cursor

Not part of this pull request. It would be a separate proposal, and I may follow it with a mockup.

Working with an AI also needs the other direction: the user must be able to tell the AI exactly which point, edge or face they mean.

- **Blender does this:** with *Python Tooltips* on, a tooltip shows the Python path of a button or property, and *Copy Data Path* copies it. With *Developer Extras*, the *Indices* overlay shows the numbers of mesh elements.
- **SolidCraft already has the names:**
  - faces have persistent names, such as `F5:blend:<edge>`;
  - edges are named by the two faces they separate, such as `A|B`;
  - sketch points have ids, such as `p4`;
  - the control server used by MCP already has `ui.selection`.
- **What is missing is showing the names:**
  - hovering a point, edge or face shows its name;
  - a *Copy reference* entry in the right-click menu copies it;
  - `ui.selection` also returns the persistent name.

Then the user could write "the edge `F2:top|F2:side:l4` is wrong", and the AI would see the same thing.
