#![allow(unused)]

pub use raytracer::types::{ Vec3, vec3_dot, Ray };
pub use raytracer::implementation::{ Material, MatRecord, Reflect, Refract, HitRecord };

use rand::{ Rng, thread_rng };

//
// Functions
//

fn reflect (v: &Vec3, n: &Vec3) -> Vec3 {
    v.sub(&n.mul_f(vec3_dot(v, n)).mul_f(2.0))
}

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
        Some(MatRecord {
            reflection: Some(Reflect { direction: direction, intensity: intensity }),
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

impl Material for MatMetal {
    fn scatter (&self, ray: &Ray, hit_record: &HitRecord) -> Option<MatRecord> {
        let reflected = reflect(&ray.direction.unit_vector(), &hit_record.normal);
        let scattered = match self.fuzz {
            x if x > 0.0 => reflected.add(&random_point_in_unit_sphere().mul_f(self.fuzz)),
            _            => reflected
        };
        if vec3_dot(&scattered, &hit_record.normal) <= 0.0 {
            // TODO: Return None? Or return no reflection component?
            return None;
        }
        Some(MatRecord {
            reflection: Some(Reflect { direction: scattered, intensity: 1.0 - self.attenuation }),
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

fn refract (v: &Vec3, n: &Vec3, ni_over_nt: f32) -> Option<Vec3> {
    let uv = v.unit_vector();
    let dt = vec3_dot(&uv, n);
    let discriminant = 1.0 - ni_over_nt * ni_over_nt * (1.0 - dt * dt);
    if discriminant > 0.0 {
        let refracted = uv.sub(&n.mul_f(dt)).mul_f(ni_over_nt).sub(&n.mul_f(discriminant.sqrt()));
        return Some(refracted);
    }
    None
}

fn schlick_reflect_prob (cosine: f32, ref_idx: f32) -> f32 {
    let r0 = (1.0 - ref_idx) / (1.0 + ref_idx);
    let r0 = r0 * r0;
    r0 + (1.0 - r0) * (1.0 - cosine).powf(5.0)
}

/// Fresnel equation: ration of reflected light for a given incident direction and surface normal
// fn fresnel (indicent_direction: &Vec3, normal: &Vec3) -> f32 {
// 
// }

impl Material for MatDielectric {
    fn scatter (&self, ray: &Ray, hit_record: &HitRecord) -> Option<MatRecord> {
        let mut rng = thread_rng();

        let dot = vec3_dot(&ray.direction, &hit_record.normal);
        let (outward_normal, ni_over_nt, cosine) =
            if dot > 0.0 {
                (hit_record.normal.negate(), self.ref_index, self.ref_index * dot / ray.direction.length())
            } else {
                (hit_record.normal.clone(), 1.0 / self.ref_index, -dot / ray.direction.length())
            };

        let reflect_prob = schlick_reflect_prob(cosine, self.ref_index);

        let refract = refract(&ray.direction, &outward_normal, ni_over_nt)
                        .map(|r| Refract { direction: r, intensity: 1.0 - reflect_prob });

        let reflect = Some(Reflect {
            direction: reflect(&ray.direction, &hit_record.normal),
            intensity: reflect_prob
        });

        // TODO: Apply attenuation?
        Some(MatRecord {
            refraction: refract,
            reflection: reflect,
            albedo: self.albedo.clone()
        })
    }
}