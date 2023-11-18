use std::{
    fs::{self, File},
    io::BufReader,
    mem::size_of,
    path::Path,
    sync, thread,
};

use ash::vk;
use vent_rendering::{
    buffer::VulkanBuffer, image::VulkanImage, instance::VulkanInstance, Vertex3D,
};

use crate::Model3D;

use super::{Light, Material, Mesh3D, ModelError};

pub(crate) struct GLTFLoader {}

impl GLTFLoader {
    pub async fn load(instance: &VulkanInstance, path: &Path) -> Result<Model3D, ModelError> {
        let doc = gltf::Gltf::from_reader(fs::File::open(path).unwrap()).unwrap();

        let path = path.parent().unwrap_or_else(|| Path::new("./"));

        let buffer_data = gltf::import_buffers(&doc, Some(path), doc.blob.clone())
            .expect("Failed to Load glTF Buffers");

        let mut meshes = Vec::new();
        doc.scenes().for_each(|scene| {
            scene.nodes().for_each(|node| {
                Self::load_node(instance, path, node, &buffer_data, &mut meshes);
            })
        });

        Ok(Model3D { meshes })
    }

    fn load_node(
        instance: &VulkanInstance,
        model_dir: &Path,
        node: gltf::Node<'_>,
        buffer_data: &[gltf::buffer::Data],
        meshes: &mut Vec<Mesh3D>,
    ) {
        if let Some(mesh) = node.mesh() {
            Self::load_mesh_multithreaded(instance, model_dir, mesh, buffer_data, meshes);
        }

        node.children()
            .for_each(|child| Self::load_node(instance, model_dir, child, buffer_data, meshes))
    }

    fn load_mesh_multithreaded(
        instance: &VulkanInstance,
        model_dir: &Path,
        mesh: gltf::Mesh,
        buffer_data: &[gltf::buffer::Data],
        meshes: &mut Vec<Mesh3D>,
    ) {
        let primitive_len = mesh.primitives().size_hint().0;
        let (tx, rx) = sync::mpsc::sync_channel(primitive_len); // Create bounded channels

        // Spawn threads to load mesh primitive
        thread::scope(|s| {
            let tx = tx.clone();
            for primitive in mesh.primitives() {
                let tx = tx.clone();
                let mesh = mesh.clone();
                let instance = instance;
                let model_dir = model_dir;
                let buffer_data = buffer_data;

                s.spawn(move || {
                    let loaded_material =
                        Self::load_material(instance, model_dir, primitive.material(), buffer_data);

                    let loaded_mesh = Self::load_primitive(
                        instance,
                        loaded_material,
                        mesh.name(),
                        buffer_data,
                        primitive,
                    );
                    tx.send(loaded_mesh).unwrap();
                });
            }
        });
        for _ in 0..primitive_len {
            let mesh = rx.recv().unwrap();
            meshes.push(mesh);
        }
    }

    fn load_material(
        instance: &VulkanInstance,
        model_dir: &Path,
        material: gltf::Material<'_>,
        buffer_data: &[gltf::buffer::Data],
        // image_data: &[gltf::image::Data],
    ) -> Vec<vk::DescriptorSet> {
        let pbr = material.pbr_metallic_roughness();

        let diffuse_texture = if let Some(texture) = pbr.base_color_texture() {
            match texture.texture().source().source() {
                gltf::image::Source::View {
                    view,
                    mime_type: img_type,
                } => {
                    let image = image::load_from_memory_with_format(
                        &buffer_data[view.buffer().index()],
                        image::ImageFormat::from_mime_type(img_type).expect("TODO: Error Handling"),
                    )
                    .unwrap();
                    VulkanImage::from_image(
                        &instance.device,
                        image,
                        instance.command_pool,
                        &instance.memory_allocator,
                        instance.graphics_queue,
                        None,
                    )
                }
                gltf::image::Source::Uri { uri, mime_type } => {
                    let sampler = texture.texture().sampler();
                    let sampler = Self::convert_sampler(&sampler);
                    let image = if let Some(mime_type) = mime_type {
                        image::load(
                            BufReader::new(File::open(model_dir.join(uri)).unwrap()),
                            image::ImageFormat::from_mime_type(mime_type)
                                .expect("TODO: Error Handling"),
                        )
                        .unwrap()
                    } else {
                        image::open(model_dir.join(uri)).unwrap()
                    };

                    VulkanImage::from_image(
                        &instance.device,
                        image,
                        instance.command_pool,
                        &instance.memory_allocator,
                        instance.graphics_queue,
                        Some(sampler),
                    )
                }
            }
        } else {
            VulkanImage::from_color(
                &instance.device,
                [255, 255, 255, 255],
                vk::Extent2D {
                    width: 128,
                    height: 128,
                },
            )
        };

        let binding = Material {
            base_color: pbr.base_color_factor(),
        };

        let mut uniform_buffers = Self::create_uniform_buffers(instance, &binding);

        Self::write_sets(instance, diffuse_texture, &uniform_buffers)
    }

    fn create_uniform_buffers(instance: &VulkanInstance, material: &Material) -> Vec<VulkanBuffer> {
        let mut uniform_buffers = vec![];
        for _ in 0..instance.swapchain_images.len() {
            let buffer = unsafe {
                VulkanBuffer::new_init_type(
                    &instance.device,
                    &instance.memory_allocator,
                    size_of::<Material>() as vk::DeviceSize,
                    vk::BufferUsageFlags::UNIFORM_BUFFER,
                    material,
                )
            };
            uniform_buffers.push(buffer)
        }
        uniform_buffers
    }

    fn write_sets(
        instance: &VulkanInstance,
        diffuse_texture: VulkanImage,
        uniforms_buffers: &Vec<VulkanBuffer>,
    ) -> Vec<vk::DescriptorSet> {
        let descriptor_sets = VulkanInstance::allocate_descriptor_sets(
            &instance.device,
            instance.descriptor_pool,
            instance.descriptor_set_layout,
            uniforms_buffers.len(),
        );

        for (i, &_descritptor_set) in descriptor_sets.iter().enumerate() {
            let image_info = vk::DescriptorImageInfo::builder()
                .image_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL)
                .image_view(diffuse_texture.image_view)
                .sampler(diffuse_texture.sampler)
                .build();

            let material_buffer_info = vk::DescriptorBufferInfo::builder()
                .buffer(uniforms_buffers[i].buffer)
                .offset(0)
                .range(size_of::<Material>() as vk::DeviceSize)
                .build();

            let light_buffer_info = vk::DescriptorBufferInfo::builder()
                .buffer(uniforms_buffers[i].buffer)
                .offset(0)
                .range(size_of::<Light>() as vk::DeviceSize)
                .build();

            let desc_sets = [
                // Vertex
                vk::WriteDescriptorSet {
                    dst_set: descriptor_sets[0],
                    dst_binding: 0,
                    descriptor_count: 1,
                    descriptor_type: vk::DescriptorType::UNIFORM_BUFFER,
                    p_buffer_info: &buffer_info,
                    ..Default::default()
                },
                // Fragment
                vk::WriteDescriptorSet {
                    dst_set: descriptor_sets[0],
                    descriptor_count: 1,
                    descriptor_type: vk::DescriptorType::COMBINED_IMAGE_SAMPLER,
                    p_image_info: &image_info,
                    ..Default::default()
                },
                vk::WriteDescriptorSet {
                    dst_set: descriptor_sets[0],
                    dst_binding: 1,
                    descriptor_count: 1,
                    descriptor_type: vk::DescriptorType::UNIFORM_BUFFER,
                    p_buffer_info: &material_buffer_info,
                    ..Default::default()
                },
                vk::WriteDescriptorSet {
                    dst_set: descriptor_sets[0],
                    dst_binding: 2,
                    descriptor_count: 1,
                    descriptor_type: vk::DescriptorType::UNIFORM_BUFFER,
                    p_buffer_info: &light_buffer_info,
                    ..Default::default()
                },
            ];

            unsafe {
                instance
                    .device
                    .update_descriptor_sets(&fragment_desc_sets, &[]);
            }
        }
        descriptor_sets
    }

    /// Converts an gltf Texture Sampler into Vulkan Sampler Info
    #[must_use]
    fn convert_sampler(sampler: &gltf::texture::Sampler) -> vk::SamplerCreateInfo {
        let mag_filter = sampler.mag_filter().map_or(
            VulkanImage::DEFAULT_TEXTURE_FILTER,
            |filter| match filter {
                gltf::texture::MagFilter::Nearest => vk::Filter::NEAREST,
                gltf::texture::MagFilter::Linear => vk::Filter::LINEAR,
            },
        );

        let (min_filter, mipmap_filter) = sampler.min_filter().map_or(
            (
                VulkanImage::DEFAULT_TEXTURE_FILTER,
                vk::SamplerMipmapMode::LINEAR,
            ),
            |filter| match filter {
                gltf::texture::MinFilter::Nearest => {
                    (vk::Filter::NEAREST, vk::SamplerMipmapMode::NEAREST)
                }
                gltf::texture::MinFilter::Linear
                | gltf::texture::MinFilter::LinearMipmapNearest => {
                    (vk::Filter::LINEAR, vk::SamplerMipmapMode::NEAREST)
                }
                gltf::texture::MinFilter::NearestMipmapNearest => {
                    (vk::Filter::NEAREST, vk::SamplerMipmapMode::NEAREST)
                }
                gltf::texture::MinFilter::LinearMipmapLinear => {
                    (vk::Filter::LINEAR, vk::SamplerMipmapMode::LINEAR)
                }
                _ => unimplemented!(),
            },
        );

        let address_mode_u = Self::conv_wrapping_mode(sampler.wrap_s());
        let address_mode_v = Self::conv_wrapping_mode(sampler.wrap_t());

        vk::SamplerCreateInfo::builder()
            .mag_filter(mag_filter)
            .min_filter(min_filter)
            .mipmap_mode(mipmap_filter)
            .address_mode_u(address_mode_u)
            .address_mode_v(address_mode_v)
            .build()
    }

    #[must_use]
    const fn conv_wrapping_mode(mode: gltf::texture::WrappingMode) -> vk::SamplerAddressMode {
        match mode {
            gltf::texture::WrappingMode::ClampToEdge => vk::SamplerAddressMode::CLAMP_TO_EDGE,
            gltf::texture::WrappingMode::MirroredRepeat => vk::SamplerAddressMode::MIRRORED_REPEAT,
            gltf::texture::WrappingMode::Repeat => vk::SamplerAddressMode::REPEAT,
        }
    }

    fn load_primitive(
        instance: &VulkanInstance,
        bind_group: Vec<vk::DescriptorSet>,
        name: Option<&str>,
        buffer_data: &[gltf::buffer::Data],
        primitive: gltf::Primitive,
    ) -> Mesh3D {
        let reader = primitive.reader(|buffer| Some(&buffer_data[buffer.index()]));

        let mut vertices: Vec<Vertex3D> = reader
            .read_positions()
            .unwrap()
            .map(|position| Vertex3D {
                position,
                tex_coord: Default::default(),
                normal: Default::default(),
            })
            .collect();

        if let Some(normal_attribute) = reader.read_normals() {
            for (normal_index, normal) in normal_attribute.enumerate() {
                vertices[normal_index].normal = normal;
            }
        }

        if let Some(tex_coord_attribute) = reader.read_tex_coords(0).map(|v| v.into_f32()) {
            for (tex_coord_index, tex_coord) in tex_coord_attribute.enumerate() {
                vertices[tex_coord_index].tex_coord = tex_coord;
            }
        }

        let indices: Vec<_> = reader.read_indices().unwrap().into_u32().collect();

        Mesh3D::new(
            &instance.device,
            &instance.memory_allocator,
            &vertices,
            &indices,
            Some(bind_group),
            name,
        )
    }
}
