#[repr(u32)]
#[derive(Clone, Copy)]
pub enum LightType {
    Directional = 0,
    Point = 1,
    Spot = 2,
}

#[repr(C)]
#[rustfmt::skip]
#[derive(Clone, Copy)]
pub struct GPULightData {
    pub position:    [f32; 4], // xyz = world space position, w = LightType
    pub direction:   [f32; 4], // xyz = world space direction, w is ignored
    pub color:       [f32; 4], // rgb = colro, w = intensity
    pub attenuation: [f32; 4], // x = constant, y = linear, z = quadratic, w = range

    pub spot_light_data: [f32; 4], // x = inner cone cos, y = outer cone cos
}

#[derive(Clone, Copy)]
#[rustfmt::skip]
pub struct LightData {
    pub type_of_light: LightType,

    pub position: [f32; 3],
    pub direction: [f32; 3],

    pub color: [f32; 3],
    pub color_intensity: f32,

    pub constant_attenuation: f32,
    pub linear_attenuation: f32,
    pub quadratic_attenuation: f32,
    pub attenuation_range: f32,

    pub spot_light_inner_cone_cos: f32,
    pub spot_light_outer_cone_cos: f32,
}

impl LightData {
    pub fn to_gpu_data(&self) -> GPULightData {
        let position = [
            self.position[0],
            self.position[1],
            self.position[2],
            self.type_of_light as u32 as f32,
        ];

        let direction = [self.direction[0], self.direction[1], self.direction[2], 0f32];

        let color = [self.color[0], self.color[1], self.color[2], self.color_intensity];

        let attenuation = [
            self.constant_attenuation,
            self.linear_attenuation,
            self.quadratic_attenuation,
            self.attenuation_range,
        ];

        let spot_data = [
            self.spot_light_inner_cone_cos,
            self.spot_light_outer_cone_cos,
            0f32,
            0f32,
        ];

        GPULightData {
            position: position,
            direction: direction,
            color: color,
            attenuation: attenuation,
            spot_light_data: spot_data,
        }
    }

    pub fn new_point_light(position: [f32; 3], color: [f32; 3], color_intensity: f32, attenuation_range: f32) -> Self {
        Self {
            type_of_light: LightType::Point,
            position: position,
            color: color,
            color_intensity: color_intensity,
            constant_attenuation: 0f32,
            linear_attenuation: 0f32,
            quadratic_attenuation: 0f32,
            attenuation_range: attenuation_range,
            direction: [0f32; 3],
            spot_light_inner_cone_cos: 0f32,
            spot_light_outer_cone_cos: 0f32,
        }
    }

    pub fn new_directional_light(direction: [f32; 3], color: [f32; 3], color_intensity: f32) -> Self {
        Self {
            type_of_light: LightType::Directional,
            position: [0f32; 3],
            direction: direction,
            color: color,
            color_intensity: color_intensity,
            constant_attenuation: 0f32,
            linear_attenuation: 0f32,
            quadratic_attenuation: 0f32,
            attenuation_range: 0f32,
            spot_light_inner_cone_cos: 0f32,
            spot_light_outer_cone_cos: 0f32,
        }
    }

    pub fn new(
        type_of_light: LightType,
        position: [f32; 3],
        direction: [f32; 3],
        color: [f32; 3],
        color_intensity: f32,
        constant_attenuation: f32,
        linear_attenuation: f32,
        quadratic_attenuation: f32,
        attenuation_range: f32,
        spot_light_inner_cone_cos: f32,
        spot_light_outer_cone_cos: f32,
    ) -> Self {
        Self {
            type_of_light,
            position,
            direction,
            color,
            color_intensity,
            constant_attenuation,
            linear_attenuation,
            quadratic_attenuation,
            attenuation_range,
            spot_light_inner_cone_cos,
            spot_light_outer_cone_cos,
        }
    }
}
