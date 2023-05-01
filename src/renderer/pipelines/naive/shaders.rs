use std::sync::Arc;

use vulkano::{device::Device, shader::ShaderModule};

mod cs {
    vulkano_shaders::shader! {
        ty: "compute",
        src: r"
        #version 460

        layout(local_size_x = 64, local_size_y = 8, local_size_z = 1) in;
        layout(push_constant) uniform constants {
            float aspect_ratio;
            float width;
            float height;
         } push_constants;

        layout(set = 0, binding = 0, rgba8) uniform writeonly image2D img;

        struct Ray {
            vec3 origin;
            vec3 direction;
        };

        struct HitData {
            vec3 point;
            vec3 colour;
            bool hit;
        };

        float sphere_check(Ray ray, vec3 center, float radius) {
            vec3 oc = ray.origin - center;
            float a = dot(ray.direction, ray.direction);
            float b = 2.0 * dot(oc, ray.direction);
            float c = dot(oc, oc) - radius * radius;
            float discriminant = b * b - 4 * a * c;

            if (discriminant < 0) {
                return -1.0;
            } else {
                return (-b - sqrt(discriminant) ) / (2.0*a);
            }
        }

        HitData handle_miss(Ray ray) {
            vec3 dir = normalize(ray.direction);
            float t = 0.5*(dir.y + 1.0);
            vec3 colour = (1.0-t)*vec3(1.0, 1.0, 1.0) + t*vec3(0.5, 0.7, 1.0);

            HitData ret;
            ret.hit = false;
            ret.point = vec3(0,0,0);
            ret.colour = colour;
            return ret;
        }

        HitData raycast(Ray ray, vec2 uv) {
            float d = sphere_check(ray, vec3(0, 0, -1), 0.5);
            if (d == -1.0) {
                return handle_miss(ray);
            } else {
                vec3 normal = normalize(ray.origin + d * ray.direction - vec3(0, 0, -1));
                HitData ret;
                ret.hit = false;
                ret.point = vec3(0,0,0);
                ret.colour = 0.5 * vec3(normal.x + 1, normal.y + 1, normal.z + 1);
                return ret;
            }
        }

        void main() {
            ivec2 pixel = ivec2(gl_GlobalInvocationID.xy);

            vec3 origin = vec3(0, 0, 0);
            float aspect_ratio = push_constants.aspect_ratio;

            float viewport_height = 2.0;
            float viewport_width =  viewport_height / push_constants.aspect_ratio;

            vec3 horizontal = vec3(viewport_height, 0, 0);
            vec3 vertical = vec3(0, viewport_width, 0);
            int focal_length = 1;
            vec3 lower_left = origin - horizontal / 2 - vertical / 2 - vec3(0, 0, focal_length);

            vec2 uv = vec2(1, 1) - vec2(
                gl_GlobalInvocationID.x / (gl_NumWorkGroups.x + 0.0), 
                gl_GlobalInvocationID.y / (gl_NumWorkGroups.y + 0.0));

            Ray ray;
            ray.origin = origin;
            ray.direction = lower_left + uv.x * horizontal + uv.y * vertical - origin;
            
            HitData hit = raycast(ray, uv);
            imageStore(img, pixel, vec4(hit.colour, 0));
        }
        ",
    }
}

pub(crate) fn shader(device: Arc<Device>) -> Arc<ShaderModule> {
    cs::load(device.clone()).expect("failed to create shader module")
}
