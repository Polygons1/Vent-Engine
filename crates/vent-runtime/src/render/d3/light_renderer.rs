use vent_assets::Mesh3D;

#[allow(dead_code)]
pub struct LightRenderer {
    light_uniform: LightUBO,
    // light_buffer: wgpu::Buffer,
    // pub light_bind_group_layout: wgpu::BindGroupLayout,
    // pub light_bind_group: wgpu::BindGroup,
    // light_render_pipeline: wgpu::RenderPipeline,
}

#[allow(dead_code)]
impl LightRenderer {
    pub fn new(// device: &wgpu::Device,
        // camera_bind_group_layout: &wgpu::BindGroupLayout,
        // format: wgpu::TextureFormat,
    ) -> Self {
        todo!()
        // let light_uniform = LightUBO {
        //     position: [2.0, 100.0, 2.0],
        //     _padding: 0,
        //     color: [1.0, 1.0, 1.0],
        //     _padding2: 0,
        // };

        // let light_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        //     label: Some("Light VB"),
        //     contents: bytemuck::cast_slice(&[light_uniform]),
        //     usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        // });

        // let light_bind_group_layout =
        //     device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        //         entries: &[wgpu::BindGroupLayoutEntry {
        //             binding: 0,
        //             visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
        //             ty: wgpu::BindingType::Buffer {
        //                 ty: wgpu::BufferBindingType::Uniform,
        //                 has_dynamic_offset: false,
        //                 min_binding_size: wgpu::BufferSize::new(
        //                     mem::size_of::<LightUBO>() as wgpu::BufferAddress
        //                 ),
        //             },
        //             count: None,
        //         }],
        //         label: None,
        //     });

        // let light_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        //     layout: &light_bind_group_layout,
        //     entries: &[wgpu::BindGroupEntry {
        //         binding: 0,
        //         resource: light_buffer.as_entire_binding(),
        //     }],
        //     label: None,
        // });

        // let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        //     label: Some("Light Pipeline Layout"),
        //     bind_group_layouts: &[camera_bind_group_layout, &light_bind_group_layout],
        //     push_constant_ranges: &[],
        // });
        // let shader = device.create_shader_module(wgpu::include_wgsl!(concat!(
        //     env!("CARGO_MANIFEST_DIR"),
        //     "/res/shaders/app/3D/light.wgsl"
        // )));
        // let vertex_buffers = [Vertex3D::LAYOUT];

        // let light_render_pipeline =
        //     device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        //         label: Some("Light Renderer Pipeline"),
        //         layout: Some(&pipeline_layout),
        //         vertex: wgpu::VertexState {
        //             module: &shader,
        //             entry_point: "vs_main",
        //             buffers: &vertex_buffers,
        //         },
        //         fragment: Some(wgpu::FragmentState {
        //             module: &shader,
        //             entry_point: "fs_main",
        //             targets: &[Some(format.into())],
        //         }),
        //         primitive: wgpu::PrimitiveState {
        //             cull_mode: Some(wgpu::Face::Back),
        //             front_face: wgpu::FrontFace::Cw,
        //             ..Default::default()
        //         },
        //         depth_stencil: Some(wgpu::DepthStencilState {
        //             format: vent_assets::Texture::DEPTH_FORMAT,
        //             depth_write_enabled: true,
        //             depth_compare: wgpu::CompareFunction::Less,
        //             stencil: wgpu::StencilState::default(),
        //             bias: wgpu::DepthBiasState::default(),
        //         }),
        //         multisample: wgpu::MultisampleState::default(),
        //         multiview: None,
        //     });

        // Self {
        //     light_uniform,
        //     light_buffer,
        //     light_bind_group_layout,
        //     light_bind_group,
        //     light_render_pipeline,
        // }
    }

    pub fn render(
        &self,
        // camera_bind_group: &'rp wgpu::BindGroup,
        _mesh: &Mesh3D,
    ) {
        // rpass.set_pipeline(&self.light_render_pipeline);
        // rpass.set_bind_group(0, camera_bind_group, &[]);
        // rpass.set_bind_group(1, &self.light_bind_group, &[]);
        // mesh.bind(rpass, false);
        // mesh.draw(rpass);
    }
}
