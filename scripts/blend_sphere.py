import studio
studio.set_bounds([-10, -10, -10], [10, 10, 10])
studio.set_quality(2)
studio.set_resolution(1)

from libfive.stdlib import *
blend(0.2, sphere(1), sphere(1).remap(Shape.X(), Shape.Y(), Shape.Z() - 1))