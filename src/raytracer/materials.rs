#![allow(unused)]

use std::mem::{ swap };

pub use raytracer::types::{ Vec3, Ray };
pub use raytracer::implementation::{ Material, MatRecord, Reflect, Refract, HitRecord, random_point_in_unit_sphere };

use rand::{ Rng };

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
    albedo: Vec3,
    reflectivity: f32,
}

impl MatLambertian {
    pub fn with_albedo (albedo: Vec3) -> MatLambertian {
        MatLambertian { 
            albedo: albedo,
            reflectivity: 0.01,
        }
    }

    pub fn with_reflectivity (mut self, reflectivity: f32) -> MatLambertian {
        assert_in_range!(reflectivity);
        self.reflectivity = reflectivity;
        self
    }
}

impl Material for MatLambertian {
    fn scatter (&self, _r: &Ray, hit_record: &HitRecord, rng: &mut Rng) -> MatRecord {
        let target = hit_record.p + hit_record.normal + random_point_in_unit_sphere(rng);
        let direction = target - hit_record.p;
        let ray = Ray::new(hit_record.p.clone(), direction);
        MatRecord {
            reflection: Some(Reflect { ray: ray, intensity: self.reflectivity }),
            refraction: None,
            albedo: self.albedo.clone()
        }
    }
}

#[derive(Clone)]
pub struct MatMetal {
    albedo: Vec3,
    reflectiveness: f32,
    fuzz: f32,
}

impl MatMetal {
    pub fn with_albedo (albedo: Vec3) -> MatMetal {
        MatMetal {
            albedo: albedo,
            reflectiveness: 0.99,
            fuzz: 0.0
        }
    }

    pub fn with_reflectiveness (mut self, reflectiveness: f32) -> MatMetal {
        assert_in_range!(reflectiveness);
        self.reflectiveness = reflectiveness;
        self
    }

    pub fn with_fuzz (mut self, fuzz: f32) -> MatMetal {
        assert_in_range!(fuzz);
        self.fuzz = fuzz;
        self
    }
}

fn reflect (incident_direction: Vec3, surface_normal: Vec3) -> Vec3 {
    let dir = incident_direction.unit_vector();
    dir - (surface_normal * Vec3::dot(dir, surface_normal) * 2.0)
}

impl Material for MatMetal {
    fn scatter (&self, ray: &Ray, hit_record: &HitRecord, rng: &mut Rng) -> MatRecord {
        let reflected = reflect(ray.direction, hit_record.normal);
        let scattered =
            if self.fuzz == 0.0 {
                reflected
            } else {
                reflected + (random_point_in_unit_sphere(rng) * self.fuzz)
            };

        let reflection =
            if Vec3::dot(scattered, hit_record.normal) > 0.0 {
                let ray = Ray::new(hit_record.p, scattered);
                Some(Reflect { ray: ray, intensity: self.reflectiveness })
            } else {
                None
            };

        MatRecord {
            reflection: reflection,
            refraction: None,
            albedo: self.albedo.clone()
        }
    }
}

#[derive(Clone)]
pub struct MatDielectric {
    albedo: Vec3,
    reflectivity: f32,
    opacity: f32,
    ref_index: f32,
}

impl MatDielectric {
    pub fn with_albedo (albedo: Vec3) -> MatDielectric {
        MatDielectric {
            albedo: albedo,
            reflectivity: 1.0,
            opacity: 0.0,
            ref_index: 1.5,
        }
    }

    pub fn with_reflectivity (mut self, reflectivity: f32) -> MatDielectric {
        assert_in_range!(reflectivity);
        self.reflectivity = reflectivity;
        self
    }

    pub fn with_opacity (mut self, opacity: f32) -> MatDielectric {
        assert_in_range!(opacity);
        self.opacity = opacity;
        self
    }

    pub fn with_ref_index (mut self, ref_index: f32) -> MatDielectric {
        self.ref_index = ref_index;
        self
    }
}

fn refract (v: Vec3, n: Vec3, ni_over_nt: f32) -> Vec3 {
    let uv = v.unit_vector();
    let dt = Vec3::dot(uv, n);
    let discriminant = 1.0 - ni_over_nt * ni_over_nt * (1.0 - dt * dt);
    if discriminant <= 0.0 {
        Vec3::zero()
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
    fn scatter (&self, ray: &Ray, hit_record: &HitRecord, rng: &mut Rng) -> MatRecord {
        let dot = Vec3::dot(ray.direction, hit_record.normal);
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
                let refraction_direction = refract(ray.direction, outward_normal, ni_over_nt).unit_vector();
                let refraction = Refract {
                    ray: Ray::new(hit_record.p.clone(), refraction_direction),
                    intensity: (1.0 - kr) * (1.0 - self.opacity)
                };
                Some(refraction)
            }
        };

        let reflection_direction = reflect(ray.direction, hit_record.normal).unit_vector();
        let reflection = Reflect {
            ray: Ray::new(hit_record.p.clone(), reflection_direction),
            intensity: kr * self.reflectivity
        };
        let reflection = Some(reflection);

        MatRecord {
            refraction: refraction,
            reflection: reflection,
            albedo: self.albedo.clone()
        }
    }
}