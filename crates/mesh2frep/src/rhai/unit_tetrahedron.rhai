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

fn unit_tetrahedron_base() {
    max( -x,
    max( -y,
    max( -z,
         (x + y + z - 1.0) / sqrt(3)
    )))
}

fn octant() {
    max( -x,
    max( -y,
         -z
    ))
}

fn vertical_capsule(radius) {
    if_nonnegative_else(-z,
        sphere(radius),
    // Else
    sqrt(x*x + y*y) - radius
    )
}

fn octant_edges() {
    min( vertical_capsule(0),
    min( remap(vertical_capsule(0), z, y, x),
         remap(vertical_capsule(0), x, z, y)
    ))

}

fn smoothed_octant() {
    let planes = [
        -x,
        -y,
        -z,
    ];

    // Iterate over the planes and count what region we are in.
    let region_marker = 0;
    for plane in planes {
        region_marker += if_nonnegative_else(plane, 1, 0);
    }

    // If the region_marker equal to 2 or 3 (greater than 1.5).
    if_nonnegative_else(region_marker - 1.5,
        octant_edges(),
    // Else: we are in the positive region.
    octant()
    )
}

// Tetrahedron that behaves like a Minkowski sum with a sphere under normal SDF operations.
fn base_tetrahedron() {
    let unit_tetrahedron_parts = [
        remap(smoothed_octant(), (x - 0.0) * 1.1785113019775793 + (y - 0.0) * -0.23570226039551584 + (z - 0.0) * -0.23570226039551584, (x - 0.0) * -0.2357022603955158 + (y - 0.0) * 1.178511301977579 + (z - 0.0) * -0.23570226039551578, (x - 0.0) * -0.2357022603955158 + (y - 0.0) * -0.23570226039551578 + (z - 0.0) * 1.178511301977579),
        remap(smoothed_octant(), (x - 0.9428090415820635) * -0.7071067811865476 + (y - 0.23570226039551587) * -0.7071067811865476 + (z - 0.23570226039551587) * -0.7071067811865475, (x - 0.9428090415820635) * -0.23570226039551584 + (y - 0.23570226039551587) * 1.1785113019775793 + (z - 0.23570226039551587) * -0.2357022603955158, (x - 0.9428090415820635) * -0.2357022603955158 + (y - 0.23570226039551587) * -0.2357022603955158 + (z - 0.23570226039551587) * 1.178511301977579),
        remap(smoothed_octant(), (x - 0.23570226039551587) * -0.7071067811865475 + (y - 0.9428090415820635) * -0.7071067811865476 + (z - 0.23570226039551587) * -0.7071067811865476, (x - 0.23570226039551587) * -0.2357022603955158 + (y - 0.9428090415820635) * -0.23570226039551584 + (z - 0.23570226039551587) * 1.1785113019775793, (x - 0.23570226039551587) * 1.178511301977579 + (y - 0.9428090415820635) * -0.2357022603955158 + (z - 0.23570226039551587) * -0.2357022603955158),
        remap(smoothed_octant(), (x - 0.23570226039551587) * -0.7071067811865476 + (y - 0.23570226039551587) * -0.7071067811865475 + (z - 0.9428090415820635) * -0.7071067811865476, (x - 0.23570226039551587) * 1.1785113019775793 + (y - 0.23570226039551587) * -0.2357022603955158 + (z - 0.9428090415820635) * -0.23570226039551584, (x - 0.23570226039551587) * -0.2357022603955158 + (y - 0.23570226039551587) * 1.178511301977579 + (z - 0.9428090415820635) * -0.2357022603955158),
    ];

    let intersection = unit_tetrahedron_parts[0];
    for i in 1..unit_tetrahedron_parts.len() {
        intersection = max(intersection, unit_tetrahedron_parts[i]);
    }
    intersection
}

// A linear transformation of the above base_tetrahedron such that the vertices are at the origin, and +1 on each axis. It does not have the same property as above due to the gradients changing.
fn unit_tetrahedron() {
    remap(base_tetrahedron(), x * 0.9428090415820635 + y * 0.23570226039551587 + z * 0.23570226039551587, x * 0.23570226039551587 + y * 0.9428090415820635 + z * 0.23570226039551587, x * 0.23570226039551587 + y * 0.23570226039551587 + z * 0.9428090415820635)
}
