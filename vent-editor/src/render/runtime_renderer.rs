use egui::Window;
use vent_runtime::render::{Dimension, RuntimeRenderer};
use wgpu::{
    CommandEncoder, Device, Extent3d, SurfaceConfiguration, SurfaceError, Texture,
    TextureDimension, TextureFormat, TextureUsages, TextureView,
};
use winit::dpi::PhysicalSize;
use vent_common::render::DefaultRenderer;
use vent_runtime::render::app_renderer::AppRenderer;

pub struct EditorRuntimeRenderer {
    texture: Texture,
    app_renderer: AppRenderer,
}

impl EditorRuntimeRenderer {
    pub fn new(default_renderer: &DefaultRenderer, dimension: Dimension, extent: Extent3d) -> Self {
        let texture = default_renderer.device.create_texture(&wgpu::TextureDescriptor {
            label: None,
            size: extent,
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format: default_renderer.config.format,
            usage: default_renderer.config.usage,
            view_formats: &[],
        });
        let app_renderer = AppRenderer::new(dimension, &default_renderer);
        Self { texture, app_renderer }
    }

    pub fn render(
        &self,
        window: &winit::window::Window,
        encoder: &mut CommandEncoder,
    ) -> Result<(), SurfaceError> {
        let view = self.texture.create_view(&wgpu::TextureViewDescriptor {
            label: Some("Runtime View"),
            ..Default::default()
        });

        self.app_renderer.render(encoder, &view);
        Ok(())
    }

    pub fn resize(
        &mut self,
        device: &Device,
        config: &SurfaceConfiguration,
        new_size: &PhysicalSize<u32>,
    ) {
        self.texture = device.create_texture(&wgpu::TextureDescriptor {
            label: None,
            size: Extent3d {
                width: new_size.width,
                height: new_size.height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format: config.format,
            usage: config.usage,
            view_formats: &[],
        });
    }
}
