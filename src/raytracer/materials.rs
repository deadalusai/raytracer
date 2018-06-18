#![allow(unused)]

use std::mem::{ swap };

pub use raytracer::types::{ Vec3, vec3_dot, Ray };
pub use raytracer::implementation::{ Material, MatRecord, Reflect, Refract, HitRecord };

use rand::{ Rng, thread_rng };

//
// Materials
//

#[derive(Clone)]
pub struct MatLambertian {
    albedo: Vec3,
    attenuation: f32,
}

impl MatLambertian {
    pub fn with_albedo (albedo: Vec3) -> MatLambertian {
        MatLambertian { 
            albedo: albedo,
            attenuation: 0.9,
        }
    }

    pub fn with_attenuation (mut self, attenuation: f32) -> MatLambertian {
        self.attenuation = attenuation;
        self
    }
}

fn random_point_in_unit_sphere () -> Vec3 {
    let unit = Vec3::new(1.0, 1.0, 1.0);
    let mut rng = thread_rng();
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
    fn scatter (&self, _r: &Ray, hit_record: &HitRecord) -> Option<MatRecord> {
        let target = hit_record.p.add(&hit_record.normal).add(&random_point_in_unit_sphere());
        let direction = target.sub(&hit_record.p);
        let intensity = 1.0 - self.attenuation;
        let ray = Ray::new(hit_record.p.clone(), direction);
        Some(MatRecord {
            reflection: Some(Reflect { ray: ray, intensity: intensity }),
            refraction: None,
            albedo: self.albedo.clone()
        })
    }
}

#[derive(Clone)]
pub struct MatMetal {
    albedo: Vec3,
    attenuation: f32,
    fuzz: f32,
}

impl MatMetal {
    pub fn with_albedo (albedo: Vec3) -> MatMetal {
        MatMetal {
            albedo: albedo,
            attenuation: 0.01,
            fuzz: 0.0
        }
    }

    pub fn with_attenuation (mut self, attenuation: f32) -> MatMetal {
        self.attenuation = attenuation;
        self
    }

    pub fn with_fuzz (mut self, fuzz: f32) -> MatMetal {
        self.fuzz = fuzz;
        self
    }
}

fn reflect (incident_ray: &Ray, surface_normal: &Vec3) -> Vec3 {
    let dir = incident_ray.direction.unit_vector();
    dir.sub(&surface_normal.mul_f(vec3_dot(&dir, &surface_normal)).mul_f(2.0))
}

impl Material for MatMetal {
    fn scatter (&self, ray: &Ray, hit_record: &HitRecord) -> Option<MatRecord> {
        let reflected = reflect(&ray, &hit_record.normal);
        let scattered = match self.fuzz {
            x if x > 0.0 => reflected.add(&random_point_in_unit_sphere().mul_f(self.fuzz)),
            _            => reflected
        };
        if vec3_dot(&scattered, &hit_record.normal) <= 0.0 {
            // TODO: Return None? Or return no reflection component?
            return None;
        }
        let ray = Ray::new(hit_record.p.clone(), scattered);
        Some(MatRecord {
            reflection: Some(Reflect { ray: ray, intensity: 1.0 - self.attenuation }),
            refraction: None,
            albedo: self.albedo.clone()
        })
    }
}

#[derive(Clone)]
pub struct MatDielectric {
    albedo: Vec3,
    attenuation: f32,
    ref_index: f32,
}

impl MatDielectric {
    pub fn with_albedo (albedo: Vec3) -> MatDielectric {
        MatDielectric {
            albedo: albedo,
            attenuation: 0.0,
            ref_index: 1.5,
        }
    }

    pub fn with_attenuation (mut self, attenuation: f32) -> MatDielectric {
        self.attenuation = attenuation;
        self
    }

    pub fn with_ref_index (mut self, ref_index: f32) -> MatDielectric {
        self.ref_index = ref_index;
        self
    }
}

fn refract (incident_ray: &Ray, surface_normal: &Vec3, ref_index: f32) -> Vec3 {
    let cosi = vec3_dot(&incident_ray.direction, surface_normal); //.min(1.0).max(-1.0);
    let (eta, outward_normal) =
        if cosi < 0.0 {
            (1.0 / ref_index, surface_normal.clone())
        } else {
            (ref_index / 1.0, surface_normal.negate())
        };
    let cosi = cosi.abs();
    let k = 1.0 - eta * eta * (1.0 - cosi * cosi);
    if k < 0.0 {
        Vec3::zero()
    } else {
        incident_ray.direction.mul_f(eta).add(&outward_normal.mul_f(eta * cosi - k.sqrt()))
    }
}

/// Fresnel equation: ration of reflected light for a given incident direction and surface normal
fn fresnel (incident_ray: &Ray, normal: &Vec3, ref_index: f32) -> f32 {
    let cosi = vec3_dot(&incident_ray.direction, normal).min(1.0).max(-1.0);
    let (etai, etat) =
        if cosi > 0.0 {
            (ref_index, 1.0)
        } else {
            (1.0, ref_index)
        };
    // Compute sint using Snell's law
    let sint = etai / etat * (1.0 - cosi * cosi).max(0.0).sqrt();
    if sint >= 1.0 {
        // Total internal reflection
        return 1.0;
    }
    let cost = (1.0 - sint * sint).max(0.0).sqrt();
    let cosi = cosi.abs();
    let rs = ((etat * cosi) - (etai * cost)) / ((etat * cosi) + (etai * cost));
    let rp = ((etai * cosi) - (etat * cost)) / ((etai * cosi) + (etat * cost));
    let kr = (rs * rs + rp * rp) / 2.0;
    // As a consequence of the conservation of energy, transmittance is given by:
    // kt = 1 - kr;
    return kr;
}

impl Material for MatDielectric {
    fn scatter (&self, ray: &Ray, hit_record: &HitRecord) -> Option<MatRecord> {
        // Compute reflection/refraction ratio
        let kr = fresnel(ray, &hit_record.normal, self.ref_index);

        // Add bias to reflection/refraction ray origins to avoid acne
        const BIAS: f32 = 0.001;
        let outside = vec3_dot(&ray.direction, &hit_record.normal) < 0.0;
        let bias = hit_record.normal.mul_f(BIAS);

        // compute refraction if it is not a case of total internal reflection
        let refraction = match kr {
            kr if kr >= 1.0 => None, // Total internal reflection
            _ => {
                let refraction_origin = if outside { hit_record.p.sub(&bias) } else { hit_record.p.add(&bias) };
                let refraction_direction = refract(&ray, &hit_record.normal, self.ref_index).unit_vector();
                let refraction = Refract {
                    ray: Ray::new(refraction_origin, refraction_direction),
                    intensity: 1.0 - kr
                };
                Some(refraction)
            }
        };

        let reflection_origin = if outside { hit_record.p.add(&bias) } else { hit_record.p.sub(&bias) };
        let reflection_direction = reflect(&ray, &hit_record.normal).unit_vector();
        let reflection = Reflect {
            ray: Ray::new(reflection_origin, reflection_direction),
            intensity: kr
        };

        Some(MatRecord {
            refraction: refraction,
            reflection: Some(reflection),
            albedo: self.albedo.clone()
        })
    }
}