use vulkano::{
    buffer::BufferContents,
    shader::{SpecializationConstants, SpecializationMapEntry},
};

#[derive(BufferContents)]
#[repr(C)]
pub struct RendererConstants {
    pub(crate) aspect_ratio: f32,
    pub(crate) width: u32,
    pub(crate) height: u32,
    pub(crate) min_depth: f32,
    pub(crate) max_depth: f32,
}

unsafe impl SpecializationConstants for RendererConstants {
    fn descriptors() -> &'static [SpecializationMapEntry] {
        static DESCRIPTORS: [SpecializationMapEntry; 5] = [
            // XXX: SAFETY CHECK THIS PLS TY; VERY UNSAFE
            SpecializationMapEntry {
                constant_id: 0,
                offset: 0,
                size: 4,
            },
            SpecializationMapEntry {
                constant_id: 1,
                offset: 4,
                size: 4,
            },
            SpecializationMapEntry {
                constant_id: 2,
                offset: 8,
                size: 4,
            },
            SpecializationMapEntry {
                constant_id: 3,
                offset: 12,
                size: 4,
            },
            SpecializationMapEntry {
                constant_id: 4,
                offset: 16,
                size: 4,
            },
        ];

        &DESCRIPTORS
    }
}
