use invoke_selis::{
    GlobalUniformBinding, Uniform,
};

pub struct XorCompute {
    pipeline: wgpu::ComputePipeline,
    _xor_texture: wgpu::Texture,
    _normal_texture: wgpu::Texture,
    pub storage_bind_group: wgpu::BindGroup,
    pub render_bind_group: wgpu::BindGroup,
}

impl XorCompute {
    pub const DESC_COMPUTE: wgpu::BindGroupLayoutDescriptor<'static> =
        wgpu::BindGroupLayoutDescriptor {
            label: Some("Compute Storage Texture Layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::StorageTexture {
                        access: wgpu::StorageTextureAccess::ReadWrite,
                        format: wgpu::TextureFormat::Rgba16Float,
                        view_dimension: wgpu::TextureViewDimension::D3,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::StorageTexture {
                        access: wgpu::StorageTextureAccess::ReadWrite,
                        format: wgpu::TextureFormat::Rgba16Float,
                        view_dimension: wgpu::TextureViewDimension::D3,
                    },
                    count: None,
                },
            ],
        };
    pub const DESC_RENDER: wgpu::BindGroupLayoutDescriptor<'static> =
        wgpu::BindGroupLayoutDescriptor {
            label: Some("Render Storage Texture Layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX_FRAGMENT
                        .union(wgpu::ShaderStages::COMPUTE),
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D3,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::VERTEX_FRAGMENT
                        .union(wgpu::ShaderStages::COMPUTE),
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D3,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::VERTEX_FRAGMENT
                        .union(wgpu::ShaderStages::COMPUTE),
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        };

    pub fn new_with_module(
        device: &wgpu::Device,
        module_desc: wgpu::ShaderModuleDescriptor<'_>,
    ) -> Self {
        let size = wgpu::Extent3d {
            width: 256,
            height: 256,
            depth_or_array_layers: 256,
        };
        let xor_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("XOR Texture"),
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D3,
            format: wgpu::TextureFormat::Rgba16Float,
            usage: wgpu::TextureUsages::STORAGE_BINDING | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let xor_view = xor_texture.create_view(&Default::default());
        let normal_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("XOR Normal Texture"),
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D3,
            format: wgpu::TextureFormat::Rgba16Float,
            usage: wgpu::TextureUsages::STORAGE_BINDING | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let normal_view = normal_texture.create_view(&Default::default());

        let module = device.create_shader_module(module_desc);
        let pipeline = Self::make_pipeline(device, module);
        let storage_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("XOR Compute Bind Group"),
            layout: &device.create_bind_group_layout(&Self::DESC_COMPUTE),
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&xor_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&normal_view),
                },
            ],
        });

        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("Volume Sampler"),
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });
        let render_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("XOR Render Bind Group"),
            layout: &device.create_bind_group_layout(&Self::DESC_RENDER),
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&xor_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&normal_view),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Sampler(&sampler),
                },
            ],
        });

        Self {
            pipeline,
            _xor_texture: xor_texture,
            _normal_texture: normal_texture,
            storage_bind_group,
            render_bind_group,
        }
    }

    fn make_pipeline(device: &wgpu::Device, module: wgpu::ShaderModule) -> wgpu::ComputePipeline {
        let global_bind_group_layout = device.create_bind_group_layout(&Uniform::DESC);
        let storage_texture_layout = device.create_bind_group_layout(&Self::DESC_COMPUTE);
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("XOR Pipeline Layout"),
            bind_group_layouts: &[&global_bind_group_layout, &storage_texture_layout],
            push_constant_ranges: &[],
        });
        device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("Generate XOR Texture"),
            layout: Some(&pipeline_layout),
            module: &module,
            entry_point: "cs_main",
        })
    }
}

impl<'a> XorCompute {
    pub fn record<'pass>(
        &'a self,
        cpass: &mut wgpu::ComputePass<'pass>,
        uniform_bind_group: &'a GlobalUniformBinding,
    ) where
        'a: 'pass,
    {
        cpass.set_pipeline(&self.pipeline);

        cpass.set_bind_group(0, &uniform_bind_group.binding, &[]);
        cpass.set_bind_group(1, &self.storage_bind_group, &[]);
        cpass.dispatch_workgroups(32, 32, 32);
    }
}
