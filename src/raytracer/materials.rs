#![allow(unused)]

use std::mem::{ swap };

pub use raytracer::types::{ Vec3, vec3_dot, Ray };
pub use raytracer::implementation::{ Material, MatRecord, Reflect, Refract, HitRecord };

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
    intensity: f32,
}

impl MatLambertian {
    pub fn with_albedo (albedo: Vec3) -> MatLambertian {
        MatLambertian { 
            albedo: albedo,
            intensity: 0.5,
        }
    }

    pub fn with_intensity (mut self, intensity: f32) -> MatLambertian {
        assert_in_range!(intensity);
        self.intensity = intensity;
        self
    }
}

fn random_point_in_unit_sphere (rng: &mut Rng) -> Vec3 {
    let unit = Vec3::new(1.0, 1.0, 1.0);
    loop {
        let random_point = Vec3::new(rng.next_f32(), rng.next_f32(), rng.next_f32());
        let p = random_point.mul_f(2.0).sub(&unit);
        // Inside our sphere?
        if p.length_squared() < 1.0 {
            return p;
        }
    }
}

impl Material for MatLambertian {
    fn scatter (&self, _r: &Ray, hit_record: &HitRecord, rng: &mut Rng) -> Option<MatRecord> {
        let target = hit_record.p.add(&hit_record.normal).add(&random_point_in_unit_sphere(rng));
        let direction = target.sub(&hit_record.p);
        let ray = Ray::new(hit_record.p.clone(), direction);
        Some(MatRecord {
            reflection: Some(Reflect { ray: ray, intensity: self.intensity }),
            refraction: None,
            albedo: self.albedo.clone()
        })
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

fn reflect (incident_direction: &Vec3, surface_normal: &Vec3) -> Vec3 {
    let dir = incident_direction.unit_vector();
    dir.sub(&surface_normal.mul_f(vec3_dot(&dir, &surface_normal)).mul_f(2.0))
}

impl Material for MatMetal {
    fn scatter (&self, ray: &Ray, hit_record: &HitRecord, rng: &mut Rng) -> Option<MatRecord> {
        let reflected = reflect(&ray.direction, &hit_record.normal);
        let scattered =
            if self.fuzz == 0.0 {
                reflected
            } else {
                reflected.add(&random_point_in_unit_sphere(rng).mul_f(self.fuzz))
            };
        if vec3_dot(&scattered, &hit_record.normal) <= 0.0 {
            // TODO: Return None? Or return no reflection component?
            return None;
        }
        let ray = Ray::new(hit_record.p.clone(), scattered);
        Some(MatRecord {
            reflection: Some(Reflect { ray: ray, intensity: self.reflectiveness }),
            refraction: None,
            albedo: self.albedo.clone()
        })
    }
}

#[derive(Clone)]
pub struct MatDielectric {
    albedo: Vec3,
    reflectivity: f32,
    opaqueness: f32,
    ref_index: f32,
}

impl MatDielectric {
    pub fn with_albedo (albedo: Vec3) -> MatDielectric {
        MatDielectric {
            albedo: albedo,
            reflectivity: 1.0,
            opaqueness: 0.0,
            ref_index: 1.5,
        }
    }

    pub fn with_reflectivity (mut self, reflectivity: f32) -> MatDielectric {
        assert_in_range!(reflectivity);
        self.reflectivity = reflectivity;
        self
    }

    pub fn with_opaqueness (mut self, opaqueness: f32) -> MatDielectric {
        assert_in_range!(opaqueness);
        self.opaqueness = opaqueness;
        self
    }

    pub fn with_ref_index (mut self, ref_index: f32) -> MatDielectric {
        assert_in_range!(ref_index);
        self.ref_index = ref_index;
        self
    }
}

fn refract (v: &Vec3, n: &Vec3, ni_over_nt: f32) -> Vec3 {
    let uv = v.unit_vector();
    let dt = vec3_dot(&uv, n);
    let discriminant = 1.0 - ni_over_nt * ni_over_nt * (1.0 - dt * dt);
    if discriminant <= 0.0 {
        Vec3::zero()
    } else {
        uv.sub(&n.mul_f(dt)).mul_f(ni_over_nt).sub(&n.mul_f(discriminant.sqrt()))
    }
}

fn schlick_reflect_prob (cosine: f32, ref_idx: f32) -> f32 {
    let r0 = (1.0 - ref_idx) / (1.0 + ref_idx);
    let r0 = r0 * r0;
    r0 + (1.0 - r0) * (1.0 - cosine).powf(5.0)
}

impl Material for MatDielectric {
    fn scatter (&self, ray: &Ray, hit_record: &HitRecord, rng: &mut Rng) -> Option<MatRecord> {
        let dot = vec3_dot(&ray.direction, &hit_record.normal);
        let (outward_normal, ni_over_nt, cosine) =
            if dot > 0.0 {
                (hit_record.normal.negate(), self.ref_index, self.ref_index * dot / ray.direction.length())
            } else {
                (hit_record.normal.clone(), 1.0 / self.ref_index, -dot / ray.direction.length())
            };

        let kr = schlick_reflect_prob(cosine, self.ref_index);

        // compute refraction if it is not a case of total internal reflection
        let refraction = match kr {
            kr if kr >= 1.0 => None, // Total internal reflection
            _ => {
                let refraction_direction = refract(&ray.direction, &outward_normal, ni_over_nt).unit_vector();
                let refraction = Refract {
                    ray: Ray::new(hit_record.p.clone(), refraction_direction),
                    intensity: (1.0 - kr) * (1.0 - self.opaqueness)
                };
                Some(refraction)
            }
        };

        let reflection_direction = reflect(&ray.direction, &hit_record.normal).unit_vector();
        let reflection = Reflect {
            ray: Ray::new(hit_record.p.clone(), reflection_direction),
            intensity: kr * self.reflectivity
        };
        let reflection = Some(reflection);

        Some(MatRecord {
            refraction: refraction,
            reflection: reflection,
            albedo: self.albedo.clone()
        })
    }
}