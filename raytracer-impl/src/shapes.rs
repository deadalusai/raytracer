use crate::types::{ V3, Ray };
use crate::implementation::{ Material, Hitable, HitRecord, AABB };

//
// Shapes
//

fn intersect_sphere(ray: &Ray, origin: V3, radius: f32) -> Option<[f32; 2]> {
    let oc = ray.origin - origin;
    let a = V3::dot(ray.direction, ray.direction);
    let b = V3::dot(oc, ray.direction);
    let c = V3::dot(oc, oc) - radius * radius;
    let discriminant = b * b - a * c;
    if discriminant > 0.0 {
        // Every ray must necessarily intersect with the sphere twice
        let t0 = (-b - discriminant.sqrt()) / a;
        let t1 = (-b + discriminant.sqrt()) / a;
        return Some([t0, t1]);
    }
    None
}

pub struct Sphere {
    object_id: Option<u32>,
    origin: V3,
    radius: f32,
    material: Box<dyn Material>,
}

impl Sphere {
    pub fn new<M>(origin: V3, radius: f32, material: M) -> Self
        where M: Material + 'static
    {
        Sphere { object_id: None, origin, radius: radius, material: Box::new(material) }
    }

    #[allow(unused)]
    pub fn with_id(mut self, id: u32) -> Self {
        self.object_id = Some(id);
        self
    }
}

impl Hitable for Sphere {
    fn hit<'a>(&'a self, ray: &Ray, t_min: f32, t_max: f32) -> Option<HitRecord<'a>> {
        let object_id = self.object_id;
        let material = self.material.as_ref();

        if let Some(ts) = intersect_sphere(ray, self.origin, self.radius) {
            // Identify the best candidate intersection point
            let t = ts.iter().cloned().filter(|&t| t_min < t && t < t_max).reduce(f32::min);
            if let Some(t) = t {
                let p = ray.point_at_parameter(t);
                let normal = ((p - self.origin) / self.radius).unit();
                return Some(HitRecord { object_id, t, p, normal, material });
            }
        }

        None
    }

    fn bounding_box(&self) -> Option<AABB> {
        // Find the bounding box for a sphere
        Some(AABB::from_min_max(self.origin - self.radius, self.origin + self.radius))
    }
}

fn intersect_plane(ray: &Ray, origin: V3, normal: V3) -> Option<f32> {
    // intersection of ray with a plane at point `t`
    // t = ((plane_origin - ray_origin) . plane_normal) / (ray_direction . plane_normal)
    let denominator = V3::dot(ray.direction, normal);
    // When the plane and ray are nearing parallel the denominator approaches zero.
    if denominator.abs() < 1.0e-6 {
        return None;
    }
    let numerator = V3::dot(origin - ray.origin, normal);
    let t = numerator / denominator;
    // NOTE: A negative `t` value indicates the plane is behind the ray origin.
    // Filter for intersections inside the range we're testing for
    Some(t)
}

pub struct Plane {
    object_id: Option<u32>,
    origin: V3,
    normal: V3,
    material: Box<dyn Material>,
    radius: Option<f32>,
}

impl Plane {
    pub fn new<M>(origin: V3, normal: V3, material: M) -> Self
        where M: Material + 'static
    {
        Plane { object_id: None, origin, normal: normal.unit(), material: Box::new(material), radius: None }
    }

    pub fn with_radius(mut self, radius: f32) -> Plane {
        self.radius = Some(radius);
        self
    }

    #[allow(unused)]
    pub fn with_id(mut self, id: u32) -> Self {
        self.object_id = Some(id);
        self
    }
}

// Ref: https://www.scratchapixel.com/lessons/3d-basic-rendering/minimal-ray-tracer-rendering-simple-shapes/ray-plane-and-ray-disk-intersection
impl Hitable for Plane {
    fn hit<'a>(&'a self, ray: &Ray, t_min: f32, t_max: f32) -> Option<HitRecord<'a>> {
        let t = intersect_plane(ray, self.origin, self.normal)?;
        if t < t_min || t > t_max {
            return None;
        }
        let p = ray.point_at_parameter(t);
        // If this is a disk plane, ensure the point p falls within the radius
        if let Some(radius) = self.radius {
            if (self.origin - p).length() > radius {
                return None;
            }
        }
        let object_id = self.object_id;
        let material = self.material.as_ref();
        // If this plane is facing away from the ray we want to flip the reported normal
        // so that reflections work in both directions.
        let normal = if V3::dot(ray.direction, self.normal) > 0.0 { -self.normal } else { self.normal };
        return Some(HitRecord { object_id, t, p, normal, material });
    }

    fn bounding_box(&self) -> Option<AABB> {
        // No bounding box for a plane
        None
    }
}

// Ref: https://www.scratchapixel.com/lessons/3d-basic-rendering/ray-tracing-rendering-a-triangle/ray-triangle-intersection-geometric-solution
fn intersect_tri(ray: &Ray, tri: &MeshTriangle) -> Option<(V3, V3, f32)> {
    // Find the normal of the triangle, using v0 as the origin
    let normal = V3::cross(tri.1 - tri.0, tri.2 - tri.0).unit();
    // Find the intesection `p` with the tiangle plane
    let t = intersect_plane(ray, tri.0, normal)?;
    // `p` is a point on the same plane as all three vertices of the triangle
    let p = ray.point_at_parameter(t);
    // Test if `p` is a point inside the triangle by determining if it is "left" of each edge.
    // (The cross product of the angle of `p` with each point should align with the normal)
    if V3::dot(normal, V3::cross(tri.1 - tri.0, p - tri.0)) < 0.0 ||
        V3::dot(normal, V3::cross(tri.2 - tri.1, p - tri.1)) < 0.0 ||
        V3::dot(normal, V3::cross(tri.0 - tri.2, p - tri.2)) < 0.0 {
        return None;
    }
    Some((p, normal, t))
}

/// Find the bounding box of a single mesh triangle
fn tri_aabb(origin: V3, tri: &MeshTriangle) -> AABB {
    AABB::from_vertices(&[origin + tri.0, origin + tri.1, origin + tri.2])
}

pub struct Triangle {
    object_id: Option<u32>,
    origin: V3,
    vertices: MeshTriangle,
    material: Box<dyn Material>,
}

impl Triangle {
    pub fn new<M>(origin: V3, vertices: MeshTriangle, material: M) -> Self
        where M: Material + 'static
    {
        Triangle { object_id: None, origin, vertices, material: Box::new(material) }
    }

    #[allow(unused)]
    pub fn with_id(mut self, id: u32) -> Self {
        self.object_id = Some(id);
        self
    }
}

impl Hitable for Triangle {
    fn hit<'a>(&'a self, ray: &Ray, t_min: f32, t_max: f32) -> Option<HitRecord<'a>> {
        let (p, normal, t) = intersect_tri(ray, &self.vertices)?;
        if t < t_min || t > t_max {
            return None;
        }
        let object_id = self.object_id;
        let material = self.material.as_ref();
        // If this plane is facing away from the ray we want to flip the reported normal
        // so that reflections work in both directions.
        let normal = if V3::dot(ray.direction, normal) > 0.0 { -normal } else { normal };
        Some(HitRecord { object_id, p, t, normal, material })
    }

    fn bounding_box(&self) -> Option<AABB> {
        Some(tri_aabb(self.origin, &self.vertices))
    }
}

pub type MeshTriangle = (V3, V3, V3);
pub type MeshTriangleList = Box<[MeshTriangle]>;

pub struct Mesh {
    object_id: Option<u32>,
    origin: V3,
    triangles: MeshTriangleList,
    material: Box<dyn Material>,
    aabb: Option<AABB>,
}

impl Mesh {
    pub fn new<M>(origin: V3, triangles: MeshTriangleList, material: M) -> Self
        where M: Material + 'static
    {
        let aabb = triangles.iter()
            .map(|tri| tri_aabb(origin, tri))
            .reduce(AABB::surrounding);

        Mesh { object_id: None, origin, triangles, material: Box::new(material), aabb }
    }

    #[allow(unused)]
    pub fn with_id(mut self, id: u32) -> Self {
        self.object_id = Some(id);
        self
    }

    fn hit_nearest_triangle(&self, ray: &Ray, t_min: f32, t_max: f32) -> Option<(V3, V3, f32)> {

        let mut nearest_t = std::f32::MAX;
        let mut nearest = None;

        for &(v0, v1, v2) in self.triangles.iter() {
            if let Some((p, normal, t)) = intersect_tri(ray, &(self.origin + v0, self.origin + v1, self.origin + v2)) {
                // Is this triangle in our search range?
                // Is this triangle closer than the last one?
                if t_min < t && t < t_max && t < nearest_t {
                    nearest_t = t;
                    nearest = Some((p, normal, t));
                }
            }
        }

        nearest
    }
}

impl Hitable for Mesh {
    fn hit<'a> (&'a self, ray: &Ray, t_min: f32, t_max: f32) -> Option<HitRecord<'a>> {
        let hit_bounding_box = self.aabb.as_ref().map(|x| x.hit_aabb(ray, t_min, t_max)).unwrap_or(false);
        if !hit_bounding_box {
            return None;
        }
        let (p, normal, t) = self.hit_nearest_triangle(ray, t_min, t_max)?;
        let object_id = self.object_id;
        let material = self.material.as_ref();
        // If this plane is facing away from the ray we want to flip the reported normal
        // so that reflections work in both directions.
        let normal = if V3::dot(ray.direction, normal) > 0.0 { -normal } else { normal };
        Some(HitRecord { object_id, p, t, normal, material })
    }

    fn bounding_box(&self) -> Option<AABB> {
        self.aabb.clone()
    }
}