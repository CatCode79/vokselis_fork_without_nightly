use invoke_selis::{
    camera::CameraBinding,
    context::{HdrBackBuffer, Uniform},
    NonZeroSized,
};

use crate::xor_compute;

pub struct RaycastPipeline {
    pub pipeline: wgpu::ComputePipeline,
    _entry_point: String,
}

impl RaycastPipeline {
    pub const OFFSET_BUFFER_DESC: wgpu::BindGroupLayoutDescriptor<'static> =
        wgpu::BindGroupLayoutDescriptor {
            label: Some("Offset Buffer Bind Group Layout"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Storage { read_only: true },
                    has_dynamic_offset: true,
                    min_binding_size: Some(crate::Offset::SIZE),
                },
                count: None,
            }],
        };

    pub fn new_with_module(
        device: &wgpu::Device,
        module_desc: wgpu::ShaderModuleDescriptor<'_>,
        entry_point: &str,
    ) -> Self {
        let module = device.create_shader_module(module_desc);
        let pipeline = Self::make_pipeline(device, module, entry_point);
        Self {
            pipeline,
            _entry_point: entry_point.to_string(),
        }
    }

    fn make_pipeline(
        device: &wgpu::Device,
        module: wgpu::ShaderModule,
        entry_point: &str,
    ) -> wgpu::ComputePipeline {
        let global_bind_group_layout = device.create_bind_group_layout(&Uniform::DESC);
        let camera_bind_group_layout = device.create_bind_group_layout(&CameraBinding::DESC);
        let volume_bind_group_layout =
            device.create_bind_group_layout(&xor_compute::XorCompute::DESC_COMPUTE);
        let output_texture_bind_group_layot =
            device.create_bind_group_layout(&HdrBackBuffer::DESC_COMPUTE);
        let offset_buffer_bind_group = device.create_bind_group_layout(&Self::OFFSET_BUFFER_DESC);
        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Compute Raycast Pass Layout"),
            bind_group_layouts: &[
                &global_bind_group_layout,
                &camera_bind_group_layout,
                &volume_bind_group_layout,
                &output_texture_bind_group_layot,
                &offset_buffer_bind_group,
            ],
            push_constant_ranges: &[],
        });
        device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("Compute Raycast Pipeline"),
            layout: Some(&layout),
            module: &module,
            entry_point,
        })
    }
}
