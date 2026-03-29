import matplotlib.pyplot as plt
import numpy as np


def main():
    # cos(t) = a/h => h = a / cos(t)
    half_base_height = (1.0 / 2.0) / np.cos(np.pi / 6)

    # sin(t) = o/h => t = arcsin(o/h)
    rotation_from_octet_to_tetrahedron_vertex = -np.arcsin(half_base_height / 1.0)

    # create vector that can be easily manipulated by rotating positive y
    base_vec = np.array([[np.cos(-np.pi / 4), -np.sin(-np.pi / 4), 0],[np.sin(-np.pi / 4), np.cos(-np.pi / 4), 0],[0, 0, 1]]) @ (np.array([1.0, 1.0, 1.0]) / np.sqrt(3))

    # rotate vector upwards
    rotated_base_vec = np.array([[np.cos(rotation_from_octet_to_tetrahedron_vertex), 0, np.sin(rotation_from_octet_to_tetrahedron_vertex)],[0, 1, 0],[-np.sin(rotation_from_octet_to_tetrahedron_vertex), 0, np.cos(rotation_from_octet_to_tetrahedron_vertex)]]) @ base_vec

    # finally, rotate the vector into place of the tetrahedron vertex
    z_tet_vertex = np.array([[np.cos(np.pi / 4), -np.sin(np.pi / 4), 0],[np.sin(np.pi / 4), np.cos(np.pi / 4), 0],[0, 0, 1]]) @ rotated_base_vec

    # create the other vertices by rearranging the vector elements
    x_tet_vertex = np.roll(z_tet_vertex, 1)
    y_tet_vertex = np.roll(x_tet_vertex, 1)

    origin_vertex = np.array([0.0, 0.0, 0.0])

    print('check that the edges are all distance 1.0')
    print(np.linalg.norm(x_tet_vertex - z_tet_vertex))
    print(np.linalg.norm(x_tet_vertex - y_tet_vertex))
    print(np.linalg.norm(y_tet_vertex - z_tet_vertex))
    print(np.linalg.norm(x_tet_vertex - origin_vertex))
    print(np.linalg.norm(y_tet_vertex - origin_vertex))
    print(np.linalg.norm(z_tet_vertex - origin_vertex))

    print(f'x vertex: {x_tet_vertex}')
    print(f'y vertex: {y_tet_vertex}')
    print(f'z vertex: {z_tet_vertex}')

    tetrahedrons = [
        (origin_vertex, [x_tet_vertex, y_tet_vertex, z_tet_vertex]),
        (x_tet_vertex, [origin_vertex, y_tet_vertex, z_tet_vertex]),
        (y_tet_vertex, [origin_vertex, z_tet_vertex, x_tet_vertex]),
        (z_tet_vertex, [origin_vertex, x_tet_vertex, y_tet_vertex]),
    ]

    print('    let unit_tetrahedron_parts = [')
    for origin, axes_map in tetrahedrons:
        tet_orig = origin
        itm = np.transpose(np.array([v - tet_orig for v in axes_map]))
        itm = np.linalg.inv(itm)

        x0, y0, z0 = (itm[(0, 0)], itm[(0, 1)], itm[(0, 2)])
        x1, y1, z1 = (itm[(1, 0)], itm[(1, 1)], itm[(1, 2)])
        x2, y2, z2 = (itm[(2, 0)], itm[(2, 1)], itm[(2, 2)])
        tx, ty, tz = (tet_orig[0], tet_orig[1], tet_orig[2])

        print(f"        remap(smoothed_octant(), (x - {tx}) * {x0} + (y - {ty}) * {y0} + (z - {tz}) * {z0}, (x - {tx}) * {x1} + (y - {ty}) * {y1} + (z - {tz}) * {z1}, (x - {tx}) * {x2} + (y - {ty}) * {y2} + (z - {tz}) * {z2}),")
    print("    ];")
    print()
    itm = np.transpose(np.array(tetrahedrons[0][1]))

    x0, y0, z0 = (itm[(0, 0)], itm[(0, 1)], itm[(0, 2)])
    x1, y1, z1 = (itm[(1, 0)], itm[(1, 1)], itm[(1, 2)])
    x2, y2, z2 = (itm[(2, 0)], itm[(2, 1)], itm[(2, 2)])

    print(f"    remap(base_tetrahedron(), x * {x0} + y * {y0} + z * {z0}, x * {x1} + y * {y1} + z * {z1}, x * {x2} + y * {y2} + z * {z2})")

    vertices = np.array([x_tet_vertex, y_tet_vertex, z_tet_vertex, origin_vertex])

    fig = plt.figure()
    ax = fig.add_subplot(111, projection='3d')
    ax.set_box_aspect([1, 1, 1])

    # plot vertices
    ax.scatter(*vertices.T, s=50)

    # plot edges
    for i, v0 in enumerate(vertices):
        for v1 in vertices[i+1:]:
            ax.plot(*zip(v0, v1), 'b-', alpha=0.4)

    ax.set_title('Tetrahedron')
    plt.tight_layout()
    plt.show()

if __name__ == '__main__':
    main()
