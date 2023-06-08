#![allow(unused)]

use std::f32::consts::FRAC_PI_2;
use std::mem::{ swap };
use std::sync::Arc;

use crate::types::{ V3, Ray, IntoArc };
use crate::implementation::{ Material, MatRecord, Reflect, Refract, HitRecord, Texture };
use crate::implementation::{ random_normal_reflection_angle };

use rand::{ RngCore };

macro_rules! assert_in_range {
    ($v:ident) => {
        if ($v < 0.0 || $v > 1.0) {
            panic!("{} must be within the range of 0.0 to 1.0", stringify!($v));
        } 
    };
}

//
// Materials
//

#[derive(Clone)]
pub struct MatLambertian {
    reflectivity: f32,
}

impl Default for MatLambertian {
    fn default() -> Self {
        Self { reflectivity: 0.0 }
    }
}

impl MatLambertian {
    pub fn with_reflectivity(mut self, reflectivity: f32) -> Self {
        assert_in_range!(reflectivity);
        self.reflectivity = reflectivity;
        self
    }
}

impl Material for MatLambertian {
    fn scatter(&self, _r: &Ray, hit_record: &HitRecord, rng: &mut dyn RngCore) -> MatRecord {
        let direction = random_normal_reflection_angle(hit_record.normal, rng);
        let ray = Ray::new(hit_record.p.clone(), direction);
        MatRecord {
            reflection: Some(Reflect { ray, intensity: self.reflectivity }),
            refraction: None,
        }
    }
}

#[derive(Clone)]
pub struct MatSpecular {
    reflectiveness: f32,
    fuzz: f32,
}

impl Default for MatSpecular {
    fn default() -> Self {
        Self {
            reflectiveness: 1.0,
            fuzz: 0.0
        }
    }
}

impl MatSpecular {
    pub fn with_reflectivity(mut self, reflectiveness: f32) -> Self {
        assert_in_range!(reflectiveness);
        self.reflectiveness = reflectiveness;
        self
    }

    pub fn with_fuzz(mut self, fuzz: f32) -> Self {
        assert_in_range!(fuzz);
        self.fuzz = fuzz;
        self
    }
}

fn reflect(incident_direction: V3, surface_normal: V3) -> V3 {
    let dir = incident_direction.unit();
    dir - (surface_normal * V3::dot(dir, surface_normal) * 2.0)
}

impl Material for MatSpecular {
    fn scatter(&self, ray: &Ray, hit_record: &HitRecord, rng: &mut dyn RngCore) -> MatRecord {
        let reflected = reflect(ray.direction, hit_record.normal);
        let scattered =
            if self.fuzz == 0.0 {
                reflected
            } else {
                reflected + (random_normal_reflection_angle(hit_record.normal, rng).unit() * self.fuzz)
            };

        let reflection =
            if V3::dot(scattered, hit_record.normal) > 0.0 {
                let ray = Ray::new(hit_record.p, scattered);
                Some(Reflect { ray: ray, intensity: self.reflectiveness })
            } else {
                None
            };

        MatRecord {
            reflection: reflection,
            refraction: None,
        }
    }
}

#[derive(Clone)]
pub struct MatDielectric {
    reflectivity: f32,
    opacity: f32,
    ref_index: f32,
}

impl Default for MatDielectric {
    fn default() -> Self {
        Self {
            reflectivity: 1.0,
            opacity: 0.0,
            ref_index: 1.5,
        }
    }
}

impl MatDielectric {
    pub fn with_reflectivity(mut self, reflectivity: f32) -> Self {
        assert_in_range!(reflectivity);
        self.reflectivity = reflectivity;
        self
    }

    pub fn with_opacity(mut self, opacity: f32) -> Self {
        assert_in_range!(opacity);
        self.opacity = opacity;
        self
    }

    pub fn with_ref_index(mut self, ref_index: f32) -> Self {
        self.ref_index = ref_index;
        self
    }
}

fn refract (v: V3, n: V3, ni_over_nt: f32) -> V3 {
    let uv = v.unit();
    let dt = V3::dot(uv, n);
    let discriminant = 1.0 - ni_over_nt * ni_over_nt * (1.0 - dt * dt);
    if discriminant <= 0.0 {
        V3::ZERO
    } else {
        (uv - (n * dt)) * ni_over_nt - (n * discriminant.sqrt())
    }
}

fn schlick_reflect_prob (cosine: f32, ref_idx: f32) -> f32 {
    let r0 = (1.0 - ref_idx) / (1.0 + ref_idx);
    let r0 = r0 * r0;
    r0 + (1.0 - r0) * (1.0 - cosine).powf(5.0)
}

impl Material for MatDielectric {
    fn scatter (&self, ray: &Ray, hit_record: &HitRecord, rng: &mut dyn RngCore) -> MatRecord {
        let dot = V3::dot(ray.direction, hit_record.normal);
        let (outward_normal, ni_over_nt, cosine) =
            if dot > 0.0 {
                (-hit_record.normal, self.ref_index, self.ref_index * dot / ray.direction.length())
            } else {
                (hit_record.normal, 1.0 / self.ref_index, -dot / ray.direction.length())
            };

        let kr = schlick_reflect_prob(cosine, self.ref_index);

        // compute refraction if it is not a case of total internal reflection
        let refraction = match kr {
            kr if kr >= 1.0 => None, // Total internal reflection
            _ => {
                let refraction_direction = refract(ray.direction, outward_normal, ni_over_nt).unit();
                let refraction = Refract {
                    ray: Ray::new(hit_record.p.clone(), refraction_direction),
                    intensity: (1.0 - kr) * (1.0 - self.opacity)
                };
                Some(refraction)
            }
        };

        let reflection_direction = reflect(ray.direction, hit_record.normal).unit();
        let reflection = Reflect {
            ray: Ray::new(hit_record.p.clone(), reflection_direction),
            intensity: kr * self.reflectivity
        };
        let reflection = Some(reflection);

        MatRecord {
            refraction: refraction,
            reflection: reflection,
        }
    }
}