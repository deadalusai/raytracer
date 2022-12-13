use crate::types::{V3, Ray};
use crate::implementation::{Hitable, AABB, HitRecord};

// Translation

pub trait Translatable: Hitable + Sized {
    fn translated(self, translation: V3) -> Translated<Self>;
}

#[derive(Clone)]
pub struct Translated<T: Translatable> {
    inner: T,
    translation: V3,
}

impl<T: Hitable + Sized> Translatable for T {
    fn translated(self, translation: V3) -> Translated<Self> {
        Translated { inner: self, translation }
    }
}

impl<T: Hitable + Sized> Hitable for Translated<T> {
    fn hit<'a>(&'a self, ray: &Ray, t_min: f32, t_max: f32) -> Option<crate::implementation::HitRecord<'a>> {
        let translated_ray = Ray::new(ray.origin - self.translation, ray.direction.clone());
        self.inner
            .hit(&translated_ray, t_min, t_max)
            .map(|hit| HitRecord { p: hit.p + self.translation, ..hit })
    }

    fn origin(&self) -> V3 {
        self.inner.origin()
    }

    fn bounding_box(&self) -> Option<crate::implementation::AABB> {
        self.inner.bounding_box()
            .map(|aabb| AABB::from_min_max(aabb.min + self.translation, aabb.max + self.translation))
    }
}

// Rotation

pub trait Rotatable: Hitable + Sized {
    fn rotated(self, axis: V3, theta: f32) -> Rotated<Self>;
}

#[derive(Clone)]
pub struct Rotated<T: Rotatable> {
    inner: T,
    axis: V3,
    theta: f32,
}

impl<T: Hitable + Sized> Rotatable for T {
    fn rotated(self, axis: V3, theta: f32) -> Rotated<Self> {
        Rotated { inner: self, axis, theta }
    }
}

impl<T: Hitable + Sized> Hitable for Rotated<T> {
    fn hit<'a>(&'a self, ray: &Ray, t_min: f32, t_max: f32) -> Option<HitRecord<'a>> {
        // Shift ray into local frame of reference
        let origin = self.origin();
        let ray_origin = origin + (ray.origin - origin).rotate_about_axis(self.axis, self.theta);
        let ray_direction = ray.direction.rotate_about_axis(self.axis, self.theta);
        let ray = Ray::new(ray_origin, ray_direction);

        self.inner.hit(&ray, t_min, t_max).map(|hit| {
            // Shift hit point and normal back into global frame of reference
            HitRecord {
                p: origin + (hit.p - origin).rotate_about_axis(self.axis, -self.theta),
                normal: hit.normal.rotate_about_axis(self.axis, -self.theta),
                ..hit
            }
        })
    }

    fn origin(&self) -> V3 {
        self.inner.origin()
    }

    fn bounding_box(&self) -> Option<AABB> {
        // rotate the bounding box and find the new min/max
        // this may leave an AABB with lots of extra empty space, but is faster to recompute?
        let aabb = self.inner.bounding_box()?;
        let origin = self.origin();
        let min = aabb.min - origin;
        let max = aabb.max - origin;
        let corners = [
            origin + min.rotate_about_axis(self.axis, self.theta),
            origin + V3(min.0, min.1, max.2).rotate_about_axis(self.axis, self.theta),
            origin + V3(min.0, max.1, min.2).rotate_about_axis(self.axis, self.theta),
            origin + V3(max.0, min.1, min.2).rotate_about_axis(self.axis, self.theta),
            origin + V3(max.0, max.1, min.2).rotate_about_axis(self.axis, self.theta),
            origin + V3(max.0, min.1, max.2).rotate_about_axis(self.axis, self.theta),
            origin + V3(min.0, max.1, max.2).rotate_about_axis(self.axis, self.theta),
            origin + max.rotate_about_axis(self.axis, self.theta),
        ];
        Some(AABB::from_vertices(&corners))
    }
}
