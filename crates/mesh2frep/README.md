# mesh2frep

This is a program that converts manifold surface meshes (in particular, STLs that can be tetrahedronalized by TetGen) into
[Signed Distance Functions](https://en.wikipedia.org/wiki/Signed_distance_function) (SDFs) parsable by [Fidget](https://github.com/mkeeter/fidget).

## Algorithm Overview

This program uses a relatively complex representation of the tetrahedronalized mesh to produce a SDF. The process is as follows:
- Tetrahedronalize the input STL such that there is a list of tetrahedrons and a list of surface triangles.
- For each tetrahedron, create a piecewise function that returns 0 if we are inside the tetrahedron and 1 if we are outside the tetrahedron.
- For each triangle, create a piecewise function with the following cases:
  - If we are outside the prism formed by extending the triangle infinitely along its surface normal in both directions:
    - Create a union of three zero radius capsules, where each capsule is positioned along a unique triangle edge. Note that these capsules are defined as piecewise functions,
      so they do not increase in length when stepping away from the zero surface of the field. These capsules have unit gradients.
  - Else, we are inside the prism:
    - Create a plane where the triangle is, with unit gradients.
- Union the triangles via the min function.
- Create one last piecewise function, such that if the product of the tetrahedron piecewise functions is zero we negate the union of triangles, else we simply return the union
  of triangles.

You may think of this union of triangles as an infinitely thin zero surface where the manifold surface mesh exists with unit gradients. The trick used is to simply negate the
field when inside the mesh, thus resulting in a nicely behaving SDF.

## Future Work

Here are a list of improvements that could be made:
- Add a mode to normalize the internal gradients differently for the purposes of slicing them. That is, the current approach used by WeekendSlicer is fundamentally flawed,
  as setting `z` to a constant value and then adding some distance does not guarantee that distance step in the `x`,`y`-plane. It may be possible to account for this by using
  a different gradient depending on if we are inside or outside the model. This leaves some questions about how edges would be handled, as well. It may make sense to
  evaluate if there is a better way to slice SDFs.
- Use fewer edges in the representation. Currently, each edge is represented twice - it may be possible to create a less dense expression if this is reduced. It may also be
  possible to get Fidget to handle this deduplication at expression parse time, if we cleverly reuse the edges instead of creating new ones for each triangle that may only be
  eequivalent up to floating point errors.
