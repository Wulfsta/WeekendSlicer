// Conditional nonzero function that does not propagate NaN from either parameter a or b.
fn if_nonzero_else(condition, a, b) {
    let lhs = and(condition, a);
    let n_condition = not(condition);
    let rhs = and(n_condition, b);
    or(lhs, rhs)
}

// A version of the Heaviside step function where H(0) = 1.
fn heaviside_step_function(a) {
    min(0.0, compare(a, 0.0)) + 1.0
}

// A function that returns a if condition is nonnegative, b otherwise.
fn if_nonnegative_else(condition, a, b) {
    let res = heaviside_step_function(condition);
    if_nonzero_else(res, a, b)
}

// A function that creates of vertical capsule of a fixed height.
fn vertical_capsule(radius, height) {
    if_nonnegative_else(-z,
        sphere(radius),
    if_nonnegative_else(z + height,
        sphere(radius),
    // Else
    sqrt(x*x + y*y) - radius
    ))
}

// Helper to create infinitely thin triangles with a unit gradient. When subtracting a value, a, from this
//  function, it behaves like the Minkowski sum of the triangle and a sphere of diameter a. The remapping
//  functions are the inverse transformation of the map that takes the unit vectors in x, y, z and maps them
//  to edge one, two, and the unit normal of the given triangle.
//
// remap_x_param: remapping of the x axis for the triangle
// remap_y_param: remapping of the y axis for the triangle
// remap_z_param: remapping of the z axis for the triangle
// edge_function: union of three vertical capsules of radius zero and height equal to the length of the
//  triangle edge it correponds to, oriented in space at the edge.
fn unit_gradient_function_triangle_helper(remap_x_param, remap_y_param, remap_z_param, edge_function) {
    let planes = [
        -x,
        -y,
        (x + y - 1.0),
    ];

    // Iterate over the planes and count what region we are in.
    let region_marker = 0;
    for plane in planes {
        region_marker += if_nonnegative_else(plane, 1, 0);
    }

    // If the region_marker equal to 1 or 2 (greater than 0.5).
    if_nonnegative_else(remap(region_marker - 0.5, remap_x_param, remap_y_param, remap_z_param),
        // We do not remap the edge function, as it would stretch the cylanders and spheres, ruining the UGF property.
        edge_function,
    // Else: we are in the positive region.
    remap(abs(z), remap_x_param, remap_y_param, remap_z_param)
    )
}
