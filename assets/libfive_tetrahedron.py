import studio
import numpy as np
studio.set_bounds([-10, -10, -10], [10, 10, 10])
studio.set_quality(8)
studio.set_resolution(10)

from libfive.stdlib import *

def tetrahedron():
    return intersection(
        -Shape.X(),
        -Shape.Y(),
        -Shape.Z(),
        (Shape.X()) + (Shape.Y()) + (Shape.Z()) - 1
    )


matrix = np.linalg.inv(np.array([[1, 1, 1], [0, 3, 2], [0, 0, 1]]))

tetrahedron().remap(
    Shape.X() * matrix[0][0] + Shape.Y() * matrix[1][0] + Shape.Z() * matrix[2][0],
    Shape.X() * matrix[0][1] + Shape.Y() * matrix[1][1] + Shape.Z() * matrix[2][1],
    Shape.X() * matrix[0][2] + Shape.Y() * matrix[1][2] + Shape.Z() * matrix[2][2]
)