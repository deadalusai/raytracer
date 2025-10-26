use std::sync::Arc;

use crate::bvh::{ Bvh, BvhObject };
use crate::types::{ IntoArc, Ray, V2, V3 };
use crate::viewport::Viewport;

use rand::{ RngCore, Rng };

// Util

const PI: f32 = std::f32::consts::PI;
const TWO_PI: f32 = 2.0 * PI;
const HALF_PI: f32 = PI / 2.0;

/// Given a {normal}, pick a random deflection from that normal of
/// between 0 and 90 degrees, at any angle around the normal
pub fn random_normal_reflection_angle(normal: V3, rng: &mut dyn RngCore) -> V3 {
    let theta1 = rng.gen::<f32>() * HALF_PI; // First angle, deflection from normal 0-90 deg
    let theta2 = rng.gen::<f32>() * TWO_PI; // Second angle, rotation around normal 0-360 deg

    fn arbitrary_perpendicular_vector(v: V3) -> V3 {
        // Pick another arbitrary vector {k} which is not parallel to the input vector {v}
        //
        // The cross product produces a vector which is perpendicular to the input vector {v}
        // unless {v} and {v2} are parallel in which case it produces (0,0,0).
        //
        // The orientation of the final vector depends on the relationship between {v} and {v2} and so is arbitrary
        let p = V3::cross(v, V3::POS_X);
        if p != V3::ZERO { p } else { V3::cross(v, V3::POS_Y) }
    }

    normal
        // Deflect along the axis perpendicular to the normal
        .rotate_about_axis(arbitrary_perpendicular_vector(normal), theta1)
        // Rotate around the original normal
        .rotate_about_axis(normal, theta2)
}

// AABB / Bounding Boxes

#[derive(Clone, Debug, Default)]
pub struct AABB {
    pub min: V3,
    pub max: V3,
}

impl AABB {
    /// Creates a bounding box from the given min/max vertices
    pub fn from_min_max(min: V3, max: V3) -> AABB {
        AABB { min, max }
    }

    /// Finds the axis-aligned bounding box which fully contains the given list of vertices
    pub fn from_vertices(vertices: &[V3]) -> AABB {
        AABB::from_vertices_iter(vertices.iter().cloned())
    }

    /// Creates a bounding box which fully contains the given two vertices
    pub fn surrounding(b0: AABB, b1: AABB) -> AABB {
        AABB::from_vertices_iter([b0.min, b0.max, b1.min, b1.max])
    }

    /// Finds the axis-aligned bounding box which fully contains the given sequence of vertices
    pub fn from_vertices_iter(vertices: impl IntoIterator<Item=V3>) -> AABB {
        let mut iter = vertices.into_iter();

        let mut min = iter.next().expect("Cannot create AABB from empty vertex list").clone();
        let mut max = min.clone();

        for vert in iter {
            min.0 = f32::min(min.0, vert.0);
            min.1 = f32::min(min.1, vert.1);
            min.2 = f32::min(min.2, vert.2);

            max.0 = f32::max(max.0, vert.0);
            max.1 = f32::max(max.1, vert.1);
            max.2 = f32::max(max.2, vert.2);
        }

        AABB::from_min_max(min, max)
    }

    pub fn hit_aabb(&self, ray: Ray, mut t_min: f32, mut t_max: f32) -> bool {
        // Algorithm from "Ray Tracing - The Next Weekend"
        // Attempt to determine if this ray intersects with this AABB in all three dimensions
        let ray_origin = ray.origin.xyz();
        let ray_direction = ray.direction.xyz();
        let min = self.min.xyz();
        let max = self.max.xyz();
        for dimension in 0..=2 {
            let inv_d = 1.0 / ray_direction[dimension];
            let mut t0 = (min[dimension] - ray_origin[dimension]) * inv_d;
            let mut t1 = (max[dimension] - ray_origin[dimension]) * inv_d;
            if inv_d < 0.0 {
                std::mem::swap(&mut t0, &mut t1);
            }
            t_min = if t0 > t_min { t0 } else { t_min };
            t_max = if t1 < t_max { t1 } else { t_max };
            if t_max < t_min {
                // No intersection on this dimension
                return false;
            }
        }

        true
    }

    pub fn corners(&self) -> [V3; 8] {
        [
            self.min,
            V3(self.min.0, self.min.1, self.max.2),
            V3(self.min.0, self.max.1, self.min.2),
            V3(self.max.0, self.min.1, self.min.2),
            self.max,
            V3(self.max.0, self.max.1, self.min.2),
            V3(self.max.0, self.min.1, self.max.2),
            V3(self.min.0, self.max.1, self.max.2),
        ]
    }
}

// Materials

pub struct Reflect {
    pub ray: Ray,
    pub intensity: f32,
}

pub struct Refract {
    pub ray: Ray,
    pub intensity: f32,
}

pub struct MatRecord {
    pub reflection: Option<Reflect>,
    pub refraction: Option<Refract>,
}

pub trait Material: Send + Sync {
    fn scatter(&self, ray: Ray, hit_record: &HitRecord, rng: &mut dyn RngCore) -> MatRecord;
}

crate::types::derive_into_arc!(trait Material);

// Textures

// TODO
// pub struct TexRecord

pub trait Texture: Send + Sync {
    fn value(&self, hit_record: &HitRecord) -> V3;
}

crate::types::derive_into_arc!(trait Texture);

// Hitables

pub struct HitRecord {
    pub entity_id: Option<u32>,
    pub t: f32,
    pub p: V3,
    pub normal: V3,
    pub uv: V2,
    pub mat_id: MatId,
    pub tex_id: TexId,
    pub tex_key: Option<usize>,
}

pub trait Hitable: Send + Sync {
    fn hit(&self, ray: Ray, t_min: f32, t_max: f32) -> Option<HitRecord>;
    /// Returns the AABB bounding box of this hitable in worldspace coordinates.
    /// The worldspace origin is assumed to be 0,0,0
    fn aabb(&self) -> AABB;
}

crate::types::derive_into_arc!(trait Hitable);

// Light sources

pub struct LightRecord {
    pub t: f32,
    pub direction: V3,
    pub color: V3,
    pub intensity: f32,
}

pub trait LightSource: Send + Sync {
    fn get_direction_and_intensity(&self, p: V3) -> Option<LightRecord>;
}

crate::types::derive_into_arc!(trait LightSource);

// Scene

pub enum SceneSky {
    Day,
    #[allow(unused)]
    Black,
}

#[derive(Clone, Copy)]
pub struct Translation {
    pub offset: V3,
}

#[derive(Clone, Copy)]
pub struct Rotation {
    pub axis: V3,
    pub theta: f32,
}

#[derive(Clone)]
pub struct Entity {
    id: Option<u32>,
    hitable: Arc<dyn Hitable>,
    translations: Vec<Translation>,
    rotations: Vec<Rotation>,
}

impl Entity {

    pub fn new(hitable: impl IntoArc<dyn Hitable>) -> Self {
        Self {
            id: None,
            hitable: hitable.into_arc(),
            translations: vec![],
            rotations: vec![],
        }
    }

    pub fn id(mut self, id: u32) -> Self {
        self.id = Some(id);
        self
    }

    pub fn translate(mut self, offset: V3) -> Self {
        self.translations.push(Translation { offset });
        self
    }

    pub fn rotate(mut self, axis: V3, theta: f32) -> Self {
        self.rotations.push(Rotation { axis, theta });
        self
    }

    fn calculate_origin_aabb(&self) -> (V3, AABB) {
        // HACK: rotate the bounding box directly and find the new min/max.
        // NOTE: this may leave an AABB with lots of extra empty space
        let mut aabb = self.hitable.aabb();
        let mut corners = aabb.corners();
        for t in self.rotations.iter() {
            // Rotates about 0,0,0
            for c in corners.iter_mut() {
                *c = (*c).rotate_about_axis(t.axis, t.theta);
            }
        }

        let mut origin = V3::ZERO;
        for t in self.translations.iter() {
            for c in corners.iter_mut() {
                *c = *c + t.offset;
            }
            origin = origin + t.offset;
        }

        aabb = AABB::from_vertices(&corners);
        (origin, aabb)
    }

    fn hit(&self, mut ray: Ray, t_min: f32, t_max: f32) -> Option<HitRecord> {
        // Transform ray into entity frame of reference
        for t in self.rotations.iter() {
            ray.origin = ray.origin.rotate_about_axis(t.axis, t.theta);
            ray.direction = ray.direction.rotate_about_axis(t.axis, t.theta);
        }
        for t in self.translations.iter() {
            ray.origin = ray.origin - t.offset;
        }

        // Hit entity
        let mut hit = self.hitable.hit(ray, t_min, t_max)?;

        // Reverse transforms on result
        for t in self.translations.iter().rev() {
            hit.p = hit.p + t.offset;
        }
        for t in self.rotations.iter().rev() {
            hit.p = hit.p.rotate_about_axis(t.axis, -t.theta);
            hit.normal = hit.normal.rotate_about_axis(t.axis, -t.theta);
        }

        hit.entity_id = self.id;
        Some(hit)
    }
}

crate::types::derive_into_arc!(struct Entity);

pub struct Scene {
    camera: Camera,
    sky: SceneSky,
    entities: Vec<Arc<Entity>>,
    lights: Vec<Arc<dyn LightSource>>,
    materials: Vec<Arc<dyn Material>>,
    textures: Vec<Arc<dyn Texture>>,
    // Constructed from scene entities before raytracing begins (see build_bvh)
    bvh_root: Option<EntityBvhRoot>,
}

#[derive(Clone, Copy)]
pub struct MatId(usize);

#[derive(Clone, Copy)]
pub struct TexId(usize);

pub struct RenderSettings {
    pub max_reflections: u32,
    pub samples_per_pixel: u32,
}

impl Scene {
    pub fn new(camera: Camera, sky: SceneSky) -> Scene {
        Scene {
            camera: camera,
            sky: sky,
            entities: vec![],
            lights: vec![],
            materials: vec![],
            textures: vec![],
            bvh_root: None,
        }
    }

    pub fn add_entity(&mut self, entity: impl IntoArc<Entity>) {
        self.entities.push(entity.into_arc());
    }

    pub fn add_material(&mut self, material: impl IntoArc<dyn Material>) -> MatId {
        let id = self.materials.len();
        self.materials.push(material.into_arc());
        MatId(id)
    }

    pub fn add_texture(&mut self, texture: impl IntoArc<dyn Texture>) -> TexId {
        let id = self.textures.len();
        self.textures.push(texture.into_arc());
        TexId(id)
    }

    pub fn add_light(&mut self, light: impl IntoArc<dyn LightSource>) {
        self.lights.push(light.into_arc());
    }

    fn hit_closest(&self, ray: Ray, t_min: f32, t_max: f32) -> Option<HitRecord> {
        let root = self.bvh_root.as_ref().expect("Scene bounding volume hierachy not constructed");
        root.try_hit(ray, t_min, t_max)
    }

    pub fn build_bvh(&mut self) {
        let bvh_entities = self.entities.iter()
            .map(|e| EntityBvh(e.clone()))
            .collect();

        self.bvh_root = Some(EntityBvhRoot::new(bvh_entities));
    }

    fn get_mat(&self, mat_id: MatId) -> &dyn Material {
        self.materials.get(mat_id.0).unwrap().as_ref()
    }

    fn get_tex(&self, tex_id: TexId) -> &dyn Texture {
        self.textures.get(tex_id.0).unwrap().as_ref()
    }
}

//
// Adapt Entities into BVH
//

pub struct EntityBvhRoot {
    bvh: Bvh,
    entities: Vec<EntityBvh>,
}

impl EntityBvhRoot {
    fn new(entities: Vec<EntityBvh>) -> Self {
        Self {
            bvh: Bvh::from(&entities),
            entities,
        }
    }

    pub fn try_hit(&self, ray: Ray, t_min: f32, t_max: f32) -> Option<HitRecord> {
        self.bvh
            .hit_candidates(ray, t_min, t_max)
            .filter_map(|candidate| self.entities[candidate.object_index].0.hit(ray, t_min, t_max))
            .reduce(|closest, next| if next.t < closest.t { next } else { closest })
    }
}

impl Hitable for EntityBvhRoot {
    fn hit(&self, ray: Ray, t_min: f32, t_max: f32) -> Option<HitRecord> {
        self.try_hit(ray, t_min, t_max)
    }

    fn aabb(&self) -> AABB {
        self.bvh.aabb()
    }
}

// Allow any Entity to be wrapped and used with the Bvh algorithm

struct EntityBvh(Arc<Entity>);

impl BvhObject for EntityBvh {
    fn calculate_centroid_aabb(&self) -> (V3, AABB) {
        // HACK: Use the entity's origin as its centroid
        self.0.calculate_origin_aabb()
    }
}

//
// Camera
//

// NOTE:
//   Y-axis goes up
//   X-axis goes right
//   Z-axis goes towards the camera (negative into the screen)

pub struct Camera {
    lower_left_corner: V3,
    horizontal: V3,
    vertical: V3,
    origin: V3,
    u: V3,
    v: V3,
    lens_radius: f32,
}

impl Camera {
    pub fn new(look_from: V3, look_at: V3, v_fov: f32, aspect_ratio: f32, lens_radius: f32, focus_dist: f32) -> Camera {
        // NOTE: Hard code v_up as vertical for now
        let v_up = V3(0.0, 1.0, 0.0);
        let theta = v_fov * PI / 180.0;
        let half_height = (theta / 2.0).tan();
        let half_width = aspect_ratio * half_height;
        let w = (look_from - look_at).unit(); // Vector from target to camera origin
        let u = V3::cross(v_up, w).unit();    // Vector from camera origin to camera right
        let v = V3::cross(w, u);              // Vector from camera origin to camera top
        Camera {
            lower_left_corner: look_from - (u * half_width * focus_dist) - (v * half_height * focus_dist) - (w * focus_dist),
            horizontal: u * (2.0 * half_width * focus_dist),
            vertical: v * (2.0 * half_height * focus_dist),
            origin: look_from,
            u,
            v,
            lens_radius,
        }
    }

    pub fn get_ray(&self, x: f32, y: f32, lens_deflection: V2) -> Ray {
        // Deflect the origin point of the ray.x
        // By casting multiple rays for the same pixel in this way we can simulate camera focus.
        let lens_deflection_x = lens_deflection.x() * self.lens_radius;
        let lens_deflection_y = lens_deflection.y() * self.lens_radius;
        let offset = (self.u * lens_deflection_x) + (self.v * lens_deflection_y);
        let origin = self.origin + offset;
        let direction = self.lower_left_corner + (self.horizontal * x) + (self.vertical * y) - self.origin - offset;
        Ray::new(origin, direction)
    }
}

//
// Core raytracing routine
//

const BIAS: f32 = 0.004;

// Sky

fn color_sky_black() -> V3 {
    V3::ZERO
}

fn color_sky_day(ray: Ray) -> V3 {
    let unit_direction = ray.direction.unit();
    let t = 0.5 * (unit_direction.y() + 1.0);
    let white = V3(1.0, 1.0, 1.0);
    let sky_blue = V3(0.5, 0.7, 1.0);
    white * (1.0 - t) + (sky_blue * t)
}

fn color_sky(ray: Ray, scene: &Scene) -> V3 {
    match scene.sky {
        SceneSky::Day => color_sky_day(ray),
        SceneSky::Black => color_sky_black(),
    }
}

// Lights and shadows

// Casts a ray *back* towards a lamp, testing for possibly shadowing objects
fn cast_light_ray_to_lamp(hit_point: V3, light_record: &LightRecord, scene: &Scene, rng: &mut dyn RngCore) -> V3 {

    // Test to see if there is any shape blocking light from this lamp by casting a ray from the shadow back to the light source
    let light_ray = Ray::new(hit_point, -light_record.direction);
    // Ignore any hits from behind this light source
    let t_max = light_record.t;

    let mut light_color = light_record.color * light_record.intensity;
    let mut closest_so_far = BIAS;

    // Perform hit tests until we escape
    while let Some(shadow_hit) = scene.hit_closest(light_ray, closest_so_far, t_max) {

        let shadow_mat = scene.get_mat(shadow_hit.mat_id).scatter(light_ray, &shadow_hit, rng);
        if let Some(shadow_refraction) = shadow_mat.refraction {
            // Hit transparent object
            // Hack: simulate colored shadows by taking the albedo of transparent materials.
            let albedo = scene.get_tex(shadow_hit.tex_id).value(&shadow_hit);
            light_color = light_color * (albedo * shadow_refraction.intensity);
            closest_so_far = shadow_hit.t + BIAS;
            continue;
        }

        // Hit opaque object (in shadow)
        return V3::ZERO;
    }

    // Escaped.
    return light_color;
}

/// Determines the color which the given ray resolves to.
fn cast_ray(ray: Ray, scene: &Scene, rng: &mut dyn RngCore, max_reflections: u32) -> V3 {

    // Internal implementation
    fn cast_ray_recursive(ray: Ray, scene: &Scene, rng: &mut dyn RngCore, recurse_limit: u32) -> V3 {

        // Exceeded our recusion limit?
        if recurse_limit == 0 {
            return color_sky(ray, scene);
        }

        // Hit anything in the scene?
        match scene.hit_closest(ray, BIAS, std::f32::MAX) {
            // Hit the sky instead
            None => color_sky(ray, scene),
            // Hit an object
            Some(hit_record) => {

                let mat_record = scene.get_mat(hit_record.mat_id).scatter(ray, &hit_record, rng);

                // We may need to recurse more than once, depending on the material we hit.
                // In this case, split the recursion limit to avoid doubling our work.
                let (reflect_limit, refract_limit) = {
                    let recurse_limit = recurse_limit - 1;
                    match (&mat_record.reflection, &mat_record.refraction) {
                        (&Some(_), &Some(_)) => {
                            let reflect_limit = recurse_limit / 2;
                            let refract_limit = recurse_limit - reflect_limit;
                            (reflect_limit, refract_limit)
                        },
                        (&Some(_), &None) => (recurse_limit, 0),
                        (&None, &Some(_)) => (0, recurse_limit),
                        (&None, &None)    => (0, 0)
                    }
                };

                // Determine color from material reflection.
                let (color_from_reflection, reflection_intensity) = match mat_record.reflection {
                    Some(ref reflect) if reflect.intensity > 0.0 => {
                        (cast_ray_recursive(reflect.ray, scene, rng, reflect_limit), reflect.intensity)
                    },
                    _ => Default::default(),
                };

                // Determine color from material refraction.
                let (color_from_refraction, refraction_intensity) = match mat_record.refraction {
                    Some(ref refract) if refract.intensity > 0.0 => {
                        (cast_ray_recursive(refract.ray, scene, rng, refract_limit), refract.intensity)
                    },
                    _ => Default::default(),
                };

                // NOTE: Move hit point slightly above p along surface normal to avoid "shadow acne"
                let hit_point = hit_record.p + (hit_record.normal * BIAS);

                // Determine color from lights in the scene.
                let mut color_from_lights = V3::ZERO;
                for light in scene.lights.iter() {
                    if let Some(light_record) = light.get_direction_and_intensity(hit_point) {
                        let light_color =
                            cast_light_ray_to_lamp(hit_point, &light_record, scene, rng) *
                            // Adjust intensity as reflection normal changes
                            f32::max(0.0, V3::dot(hit_record.normal, -light_record.direction));

                        color_from_lights = color_from_lights + light_color;
                    }
                }

                // HACK: Scale the light intensity further for highly reflective or refractive objects
                // This makes sure that color from lights doesn't overwhelm reflective or refractive materials
                let lights_intensity = f32::max(0.0, 1.0 - (reflection_intensity + refraction_intensity));
                let albedo = scene.get_tex(hit_record.tex_id).value(&hit_record);

                ((color_from_reflection * reflection_intensity) +
                 (color_from_refraction * refraction_intensity) +
                 (color_from_lights * lights_intensity)) * albedo
            }
        }
    }

    cast_ray_recursive(ray, scene, rng, max_reflections).clamp()
}

pub fn cast_rays_into_scene(settings: &RenderSettings, scene: &Scene, viewport: &Viewport, x: usize, y: usize, rng: &mut dyn RngCore) -> V3 {
    let mut col = V3(0.0, 0.0, 0.0);
    // Implement anti-aliasing by taking the average color of ofsett rays cast around these x, y coordinates.
    for _ in 0..settings.samples_per_pixel {
        // NOTE:
        // View coordinates are from upper left corner, but World coordinates are from lower left corner.
        // Need to convert coordinate systems with (height - y)
        let u = x as f32 / viewport.width as f32;
        let v = (viewport.height - y) as f32 / viewport.height as f32;
        // Apply lens deflection for focus blur
        let lens_deflection = if settings.samples_per_pixel > 1 {
            V2(rng.gen::<f32>() * 2.0 - 1.0,
               rng.gen::<f32>() * 2.0 - 1.0)
        } else {
            V2::ZERO
        };
        // Cast a ray, and determine the color
        let ray = scene.camera.get_ray(u, v, lens_deflection);
        col = col + cast_ray(ray, scene, rng, settings.max_reflections);
    }
    // Find the average
    col = col / settings.samples_per_pixel as f32;
    col // RGB color in the range 0.0 - 1.0
}
