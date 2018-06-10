
pub use raytracer::types::{ Vec3, vec3_dot, Ray };
pub use raytracer::implementation::{ Material, MatRecord, HitRecord };

use rand::{ Rng, thread_rng };

//
// Materials
//

pub struct MatLambertian {
    albedo: Vec3,
}

impl MatLambertian {
    pub fn with_albedo (albedo: Vec3) -> Box<MatLambertian> {
        Box::new(MatLambertian { albedo: albedo })
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
        let scattered = Ray::new(hit_record.p.clone(), target.sub(&hit_record.p));
        Some(MatRecord { scattered: scattered, attenuation: self.albedo.clone() })
        // TODO?
        // We could just as well scatter with some probability p and have attenuation be albedo / p
    }
}

pub struct MatMetal {
    albedo: Vec3,
    fuzz: f32,
}

impl MatMetal {
    pub fn with_albedo_and_fuzz (albedo: Vec3, fuzz: f32) -> Box<MatMetal> {
        Box::new(MatMetal { albedo: albedo, fuzz: fuzz })
    }
}

fn reflect (v: &Vec3, n: &Vec3) -> Vec3 {
    v.sub(&n.mul_f(vec3_dot(v, n)).mul_f(2.0))
}

impl Material for MatMetal {
    fn scatter (&self, ray: &Ray, hit_record: &HitRecord) -> Option<MatRecord> {
        let reflected = reflect(&ray.direction.unit_vector(), &hit_record.normal);
        let scattered = Ray::new(hit_record.p.clone(), reflected.add(&random_point_in_unit_sphere().mul_f(self.fuzz)));
        if vec3_dot(&scattered.direction, &hit_record.normal) > 0.0 {
            return Some(MatRecord { scattered: scattered, attenuation: self.albedo.clone() });
        }
        None
    }
}

pub struct MatDielectric {
    albedo: Vec3,
    ref_index: f32,
}

impl MatDielectric {
    pub fn with_albedo_and_refractive_index (albedo: Vec3, ref_index: f32) -> Box<MatDielectric> {
        Box::new(MatDielectric { albedo: albedo, ref_index: ref_index })
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

        // If prob value <= rand, reflect
        // If prob value > rand, refract
        let reflect_prob = schlick_reflect_prob(cosine, self.ref_index);

        let direction =
            refract(&ray.direction, &outward_normal, ni_over_nt)
                .filter(|_| reflect_prob < rng.next_f32())
                .unwrap_or_else(|| reflect(&ray.direction, &hit_record.normal));
        
        let scattered = Ray::new(hit_record.p.clone(), direction);
        
        // NOTE: Use albedo of 1.0 for perfectly transparent
        Some(MatRecord { scattered: scattered, attenuation: self.albedo.clone() })
    }
}