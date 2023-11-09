use vokselis::{CameraBinding, GlobalUniformBinding, HdrBackBuffer, Uniform};

use wgpu::util::DeviceExt as _;

pub(crate) struct RaycastPipeline {
    pub(crate) pipeline: wgpu::RenderPipeline,
    vertex_buffer: wgpu::Buffer,
    vertex_count: usize,
}

impl RaycastPipeline {
    pub(crate) fn new(
        device: &wgpu::Device,
        module_desc: wgpu::ShaderModuleDescriptor<'_>,
    ) -> Self {
        let vertices = [
            1., 1., 0., 0., 1., 0., 1., 1., 1., 0., 1., 1., 0., 0., 1., 0., 1., 0., 0., 0., 0., 1.,
            1., 0., 1., 0., 0., 1., 1., 1., 1., 0., 1., 0., 0., 1., 1., 0., 0., 0., 0., 0.,
        ];
        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Volume Vertex Buffer"),
            contents: bytemuck::cast_slice::<f32, _>(&vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });
        let vertex_count = vertices.len() / 3;

        let pipeline = {
            let module = device.create_shader_module(module_desc);
            Self::make_pipeline(device, &module)
        };

        Self {
            pipeline,
            vertex_buffer,
            vertex_count,
        }
    }

    fn make_pipeline(device: &wgpu::Device, module: &wgpu::ShaderModule) -> wgpu::RenderPipeline {
        let layout = {
            let global_bind_group_layout = device.create_bind_group_layout(&Uniform::DESC);
            let camera_bind_group_layout = device.create_bind_group_layout(&CameraBinding::DESC);
            let texture_bind_group_layout =
                device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("Foot BGL"),
                    entries: &[
                        wgpu::BindGroupLayoutEntry {
                            binding: 0,
                            visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                            ty: wgpu::BindingType::Texture {
                                sample_type: wgpu::TextureSampleType::Float { filterable: true },
                                view_dimension: wgpu::TextureViewDimension::D3,
                                multisampled: false,
                            },
                            count: None,
                        },
                        wgpu::BindGroupLayoutEntry {
                            binding: 1,
                            visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                            ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                            count: None,
                        },
                    ],
                });
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Screen Pass Layout"),
                bind_group_layouts: &[
                    &global_bind_group_layout,
                    &camera_bind_group_layout,
                    &texture_bind_group_layout,
                ],
                push_constant_ranges: &[],
            })
        };

        device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Raycast Pipeline"),
            layout: Some(&layout),
            fragment: Some(wgpu::FragmentState {
                module,
                entry_point: "fs_main",
                targets: &[Some(wgpu::ColorTargetState {
                    format: HdrBackBuffer::FORMAT,
                    blend: None,
                    write_mask: Default::default(),
                })],
            }),
            vertex: wgpu::VertexState {
                module,
                entry_point: "vs_main",
                buffers: &[wgpu::VertexBufferLayout {
                    array_stride: 3 * 4,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &wgpu::vertex_attr_array![0 => Float32x3],
                }],
            },
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleStrip,
                cull_mode: Some(wgpu::Face::Front),
                ..Default::default()
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
        })
    }
}

impl<'a> RaycastPipeline {
    pub(crate) fn record<'pass>(
        &'a self,
        rpass: &mut wgpu::RenderPass<'pass>,
        uniform_bind_group: &'a GlobalUniformBinding,
        camera_bind_group: &'a CameraBinding,
        volume_texture: &'a wgpu::BindGroup,
    ) where
        'a: 'pass,
    {
        rpass.set_pipeline(&self.pipeline);

        rpass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        rpass.set_bind_group(0, &uniform_bind_group.binding, &[]);
        rpass.set_bind_group(1, &camera_bind_group.bind_group, &[]);
        rpass.set_bind_group(2, &volume_texture, &[]);
        rpass.draw(0..self.vertex_count as _, 0..1);
    }
}
