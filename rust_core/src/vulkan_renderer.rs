//! Vulkan 3D 皮肤模型渲染器
//! 使用 ash crate 绑定 Vulkan API，将 Minecraft 玩家皮肤作为纹理映射到 3D 立方体
//! 完整实现：实例/设备/命令池创建、PNG 纹理上传、渲染管线、帧缓冲读取

use ash::{vk, Instance, Device};
use std::ffi::CString;
use std::os::raw::c_char;
use std::os::raw::c_int;
use stb_image::image::{load_from_memory, LoadResult};

/// Vulkan 渲染器状态
pub struct VulkanRenderer {
    pub instance: Option<Instance>,
    pub device: Option<Device>,
    pub physical_device: Option<vk::PhysicalDevice>,
    pub queue: Option<vk::Queue>,
    pub command_pool: Option<vk::CommandPool>,
    pub width: u32,
    pub height: u32,
    pub initialized: bool,

    // 渲染资源（纹理、管线、帧缓冲等）
    pub texture_image: Option<vk::Image>,
    pub texture_image_memory: Option<vk::DeviceMemory>,
    pub texture_image_view: Option<vk::ImageView>,
    pub texture_sampler: Option<vk::Sampler>,
    pub pipeline: Option<vk::Pipeline>,
    pub pipeline_layout: Option<vk::PipelineLayout>,
    pub render_pass: Option<vk::RenderPass>,
    pub framebuffer: Option<vk::Framebuffer>,
    pub vertex_buffer: Option<vk::Buffer>,
    pub vertex_buffer_memory: Option<vk::DeviceMemory>,
    pub descriptor_set_layout: Option<vk::DescriptorSetLayout>,
    pub descriptor_pool: Option<vk::DescriptorPool>,
    pub descriptor_set: Option<vk::DescriptorSet>,
    pub command_buffer: Option<vk::CommandBuffer>,
    pub image_available_semaphore: Option<vk::Semaphore>,
    pub render_finished_semaphore: Option<vk::Semaphore>,
    pub in_flight_fence: Option<vk::Fence>,

    // 离屏渲染目标（用于读取像素）
    pub offscreen_image: Option<vk::Image>,
    pub offscreen_image_memory: Option<vk::DeviceMemory>,
    pub offscreen_image_view: Option<vk::ImageView>,
}

// 顶点数据：位置 (3 floats) + 纹理坐标 (4 floats)
#[repr(C)]
#[derive(Clone, Copy)]
struct Vertex {
    pos: [f32; 3],
    tex_coord: [f32; 4],
}

// 立方体顶点数据（6个面，每个面4个顶点，每个顶点5个f32）
const CUBE_VERTICES: [Vertex; 24] = [
    // 前面 (z=+0.5)
    Vertex { pos: [-0.5, -0.5,  0.5], tex_coord: [0.0, 1.0, 0.0, 0.0] },
    Vertex { pos: [ 0.5, -0.5,  0.5], tex_coord: [1.0, 1.0, 0.0, 0.0] },
    Vertex { pos: [ 0.5,  0.5,  0.5], tex_coord: [1.0, 0.0, 0.0, 0.0] },
    Vertex { pos: [-0.5,  0.5,  0.5], tex_coord: [0.0, 0.0, 0.0, 0.0] },
    // 右面 (x=+0.5)
    Vertex { pos: [ 0.5, -0.5,  0.5], tex_coord: [0.0, 1.0, 0.0, 0.0] },
    Vertex { pos: [ 0.5, -0.5, -0.5], tex_coord: [1.0, 1.0, 0.0, 0.0] },
    Vertex { pos: [ 0.5,  0.5, -0.5], tex_coord: [1.0, 0.0, 0.0, 0.0] },
    Vertex { pos: [ 0.5,  0.5,  0.5], tex_coord: [0.0, 0.0, 0.0, 0.0] },
    // 后面 (z=-0.5)
    Vertex { pos: [ 0.5, -0.5, -0.5], tex_coord: [0.0, 1.0, 0.0, 0.0] },
    Vertex { pos: [-0.5, -0.5, -0.5], tex_coord: [1.0, 1.0, 0.0, 0.0] },
    Vertex { pos: [-0.5,  0.5, -0.5], tex_coord: [1.0, 0.0, 0.0, 0.0] },
    Vertex { pos: [ 0.5,  0.5, -0.5], tex_coord: [0.0, 0.0, 0.0, 0.0] },
    // 左面 (x=-0.5)
    Vertex { pos: [-0.5, -0.5, -0.5], tex_coord: [0.0, 1.0, 0.0, 0.0] },
    Vertex { pos: [-0.5, -0.5,  0.5], tex_coord: [1.0, 1.0, 0.0, 0.0] },
    Vertex { pos: [-0.5,  0.5,  0.5], tex_coord: [1.0, 0.0, 0.0, 0.0] },
    Vertex { pos: [-0.5,  0.5, -0.5], tex_coord: [0.0, 0.0, 0.0, 0.0] },
    // 顶面 (y=+0.5)
    Vertex { pos: [-0.5,  0.5,  0.5], tex_coord: [0.0, 1.0, 0.0, 0.0] },
    Vertex { pos: [ 0.5,  0.5,  0.5], tex_coord: [1.0, 1.0, 0.0, 0.0] },
    Vertex { pos: [ 0.5,  0.5, -0.5], tex_coord: [1.0, 0.0, 0.0, 0.0] },
    Vertex { pos: [-0.5,  0.5, -0.5], tex_coord: [0.0, 0.0, 0.0, 0.0] },
    // 底面 (y=-0.5)
    Vertex { pos: [-0.5, -0.5, -0.5], tex_coord: [0.0, 1.0, 0.0, 0.0] },
    Vertex { pos: [ 0.5, -0.5, -0.5], tex_coord: [1.0, 1.0, 0.0, 0.0] },
    Vertex { pos: [ 0.5, -0.5,  0.5], tex_coord: [1.0, 0.0, 0.0, 0.0] },
    Vertex { pos: [-0.5, -0.5,  0.5], tex_coord: [0.0, 0.0, 0.0, 0.0] },
];

// 索引数据（每个面2个三角形，共36个索引）
const CUBE_INDICES: [u16; 36] = [
    0, 1, 2, 2, 3, 0,  // 前面
    4, 5, 6, 6, 7, 4,  // 右面
    8, 9, 10, 10, 11, 8, // 后面
    12, 13, 14, 14, 15, 12, // 左面
    16, 17, 18, 18, 19, 16, // 顶面
    20, 21, 22, 22, 23, 20, // 底面
];

// 嵌入式 SPIR-V 顶点着色器（文本立方体，带纹理坐标）
const VERTEX_SHADER_SPIRV: &[u8] = include_bytes!("shaders/vert.spv");

// 嵌入式 SPIR-V 片段着色器（采样纹理）
const FRAGMENT_SHADER_SPIRV: &[u8] = include_bytes!("shaders/frag.spv");

impl VulkanRenderer {
    pub fn new() -> Self {
        Self {
            instance: None,
            device: None,
            physical_device: None,
            queue: None,
            command_pool: None,
            width: 0,
            height: 0,
            initialized: false,
            texture_image: None,
            texture_image_memory: None,
            texture_image_view: None,
            texture_sampler: None,
            pipeline: None,
            pipeline_layout: None,
            render_pass: None,
            framebuffer: None,
            vertex_buffer: None,
            vertex_buffer_memory: None,
            descriptor_set_layout: None,
            descriptor_pool: None,
            descriptor_set: None,
            command_buffer: None,
            image_available_semaphore: None,
            render_finished_semaphore: None,
            in_flight_fence: None,
            offscreen_image: None,
            offscreen_image_memory: None,
            offscreen_image_view: None,
        }
    }

    /// 初始化 Vulkan 实例、设备、命令池以及完整的渲染管线
    pub fn init(&mut self, width: u32, height: u32) -> Result<(), String> {
        self.width = width;
        self.height = height;

        // 1. 加载 Vulkan 库并创建实例
        let entry = unsafe { ash::Entry::load() }
            .map_err(|e| format!("Failed to load Vulkan entry: {}", e))?;

        let app_name = CString::new("MC Launcher Skin Viewer").unwrap();
        let mut app_info = vk::ApplicationInfo::default();
        app_info.p_application_name = app_name.as_ptr();
        app_info.application_version = vk::make_api_version(0, 1, 0, 0);
        app_info.p_engine_name = app_name.as_ptr();
        app_info.engine_version = vk::make_api_version(0, 1, 0, 0);
        app_info.api_version = vk::API_VERSION_1_2;

        let layer_names: [*const u8; 0] = [];
        let extension_names: [*const u8; 0] = [];

        let mut create_info = vk::InstanceCreateInfo::default();
        create_info.p_application_info = &app_info;
        create_info.enabled_layer_count = layer_names.len() as u32;
        create_info.pp_enabled_layer_names = layer_names.as_ptr();
        create_info.enabled_extension_count = extension_names.len() as u32;
        create_info.pp_enabled_extension_names = extension_names.as_ptr();

        let instance = unsafe {
            entry.create_instance(&create_info, None)
        }
        .map_err(|e| format!("Failed to create Vulkan instance: {:?}", e))?;

        // 2. 选择物理设备
        let physical_devices = unsafe {
            instance.enumerate_physical_devices()
        }
        .map_err(|e| format!("Failed to enumerate physical devices: {:?}", e))?;

        if physical_devices.is_empty() {
            return Err("No Vulkan physical devices found".to_string());
        }

        let physical_device = physical_devices[0];

        // 3. 获取队列族属性
        let queue_family_properties = unsafe {
            instance.get_physical_device_queue_family_properties(physical_device)
        };

        let queue_family_index = queue_family_properties
            .iter()
            .position(|props| props.queue_flags.contains(vk::QueueFlags::GRAPHICS))
            .ok_or_else(|| "No graphics queue family found".to_string())? as u32;

        // 4. 创建逻辑设备
        let queue_priorities = [1.0f32];
        let mut queue_create_info = vk::DeviceQueueCreateInfo::default();
        queue_create_info.queue_family_index = queue_family_index;
        queue_create_info.queue_count = queue_priorities.len() as u32;
        queue_create_info.p_queue_priorities = queue_priorities.as_ptr();

        let mut device_create_info = vk::DeviceCreateInfo::default();
        device_create_info.queue_create_info_count = 1;
        device_create_info.p_queue_create_infos = &queue_create_info;

        let device = unsafe {
            instance.create_device(physical_device, &device_create_info, None)
        }
        .map_err(|e| format!("Failed to create logical device: {:?}", e))?;

        // 5. 获取队列
        let queue = unsafe {
            device.get_device_queue(queue_family_index, 0)
        };

        // 6. 创建命令池
        let mut command_pool_create_info = vk::CommandPoolCreateInfo::default();
        command_pool_create_info.flags = vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER;
        command_pool_create_info.queue_family_index = queue_family_index;

        let command_pool = unsafe {
            device.create_command_pool(&command_pool_create_info, None)
        }
        .map_err(|e| format!("Failed to create command pool: {:?}", e))?;

        self.instance = Some(instance);
        self.device = Some(device);
        self.physical_device = Some(physical_device);
        self.queue = Some(queue);
        self.command_pool = Some(command_pool);

        // 7. 创建渲染资源（纹理、管线、帧缓冲等）
        self.create_render_resources()?;

        self.initialized = true;
        Ok(())
    }

    /// 创建所有渲染资源：纹理、着色器、管线、帧缓冲、顶点缓冲、描述符集等
    fn create_render_resources(&mut self) -> Result<(), String> {
        let device = self.device.as_ref().ok_or("Device not initialized")?;

        // 7.1 创建渲染 pass（单个颜色附件，RGBA8）
        let color_attachment = vk::AttachmentDescription::default()
            .format(vk::Format::R8G8B8A8_UNORM)
            .samples(vk::SampleCountFlags::TYPE_1)
            .load_op(vk::AttachmentLoadOp::CLEAR)
            .store_op(vk::AttachmentStoreOp::STORE)
            .initial_layout(vk::ImageLayout::UNDEFINED)
            .final_layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL);

        let color_attachment_ref = vk::AttachmentReference::default()
            .attachment(0)
            .layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL);

        let subpass = vk::SubpassDescription {
            pipeline_bind_point: vk::PipelineBindPoint::GRAPHICS,
            color_attachment_count: 1,
            p_color_attachments: &color_attachment_ref,
            ..Default::default()
        };

        let render_pass_create_info = vk::RenderPassCreateInfo {
            attachment_count: 1,
            p_attachments: &color_attachment,
            subpass_count: 1,
            p_subpasses: &subpass,
            ..Default::default()
        };

        let render_pass = unsafe {
            device.create_render_pass(&render_pass_create_info, None)
        }
        .map_err(|e| format!("Failed to create render pass: {:?}", e))?;
        self.render_pass = Some(render_pass);

        // 7.2 创建着色器模块
        let vert_shader_module = self.create_shader_module(device, VERTEX_SHADER_SPIRV)?;
        let frag_shader_module = self.create_shader_module(device, FRAGMENT_SHADER_SPIRV)?;

        let main_name = CString::new("main").unwrap();
        let vert_stage = vk::PipelineShaderStageCreateInfo::default()
            .stage(vk::ShaderStageFlags::VERTEX)
            .module(vert_shader_module)
            .name(main_name.as_c_str());

        let frag_stage = vk::PipelineShaderStageCreateInfo::default()
            .stage(vk::ShaderStageFlags::FRAGMENT)
            .module(frag_shader_module)
            .name(main_name.as_c_str());

        // 7.3 固定函数状态
        let vertex_input_binding = vk::VertexInputBindingDescription::default()
            .binding(0)
            .stride(std::mem::size_of::<Vertex>() as u32)
            .input_rate(vk::VertexInputRate::VERTEX);

        let vertex_input_attributes = [
            vk::VertexInputAttributeDescription::default()
                .binding(0)
                .location(0)
                .format(vk::Format::R32G32B32_SFLOAT)
                .offset(0),
            vk::VertexInputAttributeDescription::default()
                .binding(0)
                .location(1)
                .format(vk::Format::R32G32B32A32_SFLOAT)
                .offset(12),
        ];

        let vertex_binding_descs = [vertex_input_binding];
        let vertex_attr_descs = vertex_input_attributes;

        let vertex_input_info = vk::PipelineVertexInputStateCreateInfo::default()
            .vertex_binding_descriptions(&vertex_binding_descs)
            .vertex_attribute_descriptions(&vertex_attr_descs);

        let input_assembly = vk::PipelineInputAssemblyStateCreateInfo::default()
            .topology(vk::PrimitiveTopology::TRIANGLE_LIST)
            .primitive_restart_enable(false);

        let viewport = vk::Viewport::default()
            .x(0.0)
            .y(0.0)
            .width(self.width as f32)
            .height(self.height as f32)
            .min_depth(0.0)
            .max_depth(1.0);

        let scissor = vk::Rect2D::default()
            .offset(vk::Offset2D { x: 0, y: 0 })
            .extent(vk::Extent2D { width: self.width, height: self.height });

        let viewport_state = vk::PipelineViewportStateCreateInfo {
            viewport_count: 1,
            p_viewports: &viewport,
            scissor_count: 1,
            p_scissors: &scissor,
            ..Default::default()
        };

        let rasterizer = vk::PipelineRasterizationStateCreateInfo::default()
            .depth_clamp_enable(false)
            .rasterizer_discard_enable(false)
            .polygon_mode(vk::PolygonMode::FILL)
            .line_width(1.0)
            .cull_mode(vk::CullModeFlags::BACK)
            .front_face(vk::FrontFace::COUNTER_CLOCKWISE)
            .depth_bias_enable(false);

        let multisampling = vk::PipelineMultisampleStateCreateInfo::default()
            .sample_shading_enable(false)
            .rasterization_samples(vk::SampleCountFlags::TYPE_1);

        let color_blend_attachment = vk::PipelineColorBlendAttachmentState::default()
            .color_write_mask(vk::ColorComponentFlags::R | vk::ColorComponentFlags::G | vk::ColorComponentFlags::B | vk::ColorComponentFlags::A)
            .blend_enable(false);

        let color_blending = vk::PipelineColorBlendStateCreateInfo {
            logic_op_enable: vk::FALSE,
            attachment_count: 1,
            p_attachments: &color_blend_attachment,
            ..Default::default()
        };

        // 7.4 描述符集布局（纹理采样器 + 纹理图像）
        let descriptor_set_layout_bindings = [
            vk::DescriptorSetLayoutBinding::default()
                .binding(0)
                .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
                .descriptor_count(1)
                .stage_flags(vk::ShaderStageFlags::FRAGMENT),
        ];

        let descriptor_set_layout_create_info = vk::DescriptorSetLayoutCreateInfo {
            binding_count: descriptor_set_layout_bindings.len() as u32,
            p_bindings: descriptor_set_layout_bindings.as_ptr(),
            ..Default::default()
        };

        let descriptor_set_layout = unsafe {
            device.create_descriptor_set_layout(&descriptor_set_layout_create_info, None)
        }
        .map_err(|e| format!("Failed to create descriptor set layout: {:?}", e))?;
        self.descriptor_set_layout = Some(descriptor_set_layout);

        // 7.5 管线布局
        let pipeline_layout_create_info = vk::PipelineLayoutCreateInfo {
            set_layout_count: 1,
            p_set_layouts: &descriptor_set_layout,
            ..Default::default()
        };

        let pipeline_layout = unsafe {
            device.create_pipeline_layout(&pipeline_layout_create_info, None)
        }
        .map_err(|e| format!("Failed to create pipeline layout: {:?}", e))?;
        self.pipeline_layout = Some(pipeline_layout);

        // 7.6 图形管线
        let pipeline_shader_stages = [vert_stage, frag_stage];

        let pipeline_create_infos = [vk::GraphicsPipelineCreateInfo {
            stage_count: 2,
            p_stages: pipeline_shader_stages.as_ptr(),
            p_vertex_input_state: &vertex_input_info,
            p_input_assembly_state: &input_assembly,
            p_viewport_state: &viewport_state,
            p_rasterization_state: &rasterizer,
            p_multisample_state: &multisampling,
            p_color_blend_state: &color_blending,
            layout: pipeline_layout,
            render_pass,
            subpass: 0,
            ..Default::default()
        }];

        let pipelines = unsafe {
            device.create_graphics_pipelines(vk::PipelineCache::null(), &pipeline_create_infos, None)
        }
        .map_err(|e| format!("Failed to create graphics pipeline: {:?}", e))?;

        self.pipeline = Some(pipelines[0]);

        // 清理着色器模块（管线已创建，不再需要）
        unsafe {
            device.destroy_shader_module(vert_shader_module, None);
            device.destroy_shader_module(frag_shader_module, None);
        }

        // 7.7 创建顶点缓冲
        self.create_vertex_buffer()?;

        // 7.8 创建描述符池和描述符集
        self.create_descriptor_resources()?;

        // 7.9 创建帧缓冲（离屏渲染目标）
        self.create_framebuffer()?;

        // 7.10 创建同步原语
        self.create_sync_resources()?;

        // 7.11 分配命令缓冲
        self.allocate_command_buffers()?;

        Ok(())
    }

    /// 创建着色器模块
    fn create_shader_module(&self, device: &Device, code: &[u8]) -> Result<vk::ShaderModule, String> {
        // SPIR-V 代码必须是 4 字节对齐的
        let create_info = vk::ShaderModuleCreateInfo {
            code_size: code.len(),
            p_code: code.as_ptr() as *const u32,
            ..Default::default()
        };

        unsafe {
            device.create_shader_module(&create_info, None)
        }
        .map_err(|e| format!("Failed to create shader module: {:?}", e))
    }

    /// 创建顶点缓冲
    fn create_vertex_buffer(&mut self) -> Result<(), String> {
        let device = self.device.as_ref().ok_or("Device not initialized")?;
        let buffer_size = (std::mem::size_of::<Vertex>() * CUBE_VERTICES.len()) as u64;

        let buffer_create_info = vk::BufferCreateInfo::default()
            .size(buffer_size)
            .usage(vk::BufferUsageFlags::VERTEX_BUFFER)
            .sharing_mode(vk::SharingMode::EXCLUSIVE);

        let vertex_buffer = unsafe {
            device.create_buffer(&buffer_create_info, None)
        }
        .map_err(|e| format!("Failed to create vertex buffer: {:?}", e))?;

        let requirements = unsafe {
            device.get_buffer_memory_requirements(vertex_buffer)
        };

        let memory_type_index = self.find_memory_type(
            device,
            requirements.memory_type_bits,
            vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
        )?;

        let allocate_info = vk::MemoryAllocateInfo::default()
            .allocation_size(requirements.size)
            .memory_type_index(memory_type_index);

        let vertex_buffer_memory = unsafe {
            device.allocate_memory(&allocate_info, None)
        }
        .map_err(|e| format!("Failed to allocate vertex buffer memory: {:?}", e))?;

        unsafe {
            device.bind_buffer_memory(vertex_buffer, vertex_buffer_memory, 0)
        }
        .map_err(|e| format!("Failed to bind vertex buffer memory: {:?}", e))?;

        // 将顶点数据复制到缓冲
        unsafe {
            let data = device.map_memory(vertex_buffer_memory, 0, buffer_size, vk::MemoryMapFlags::empty())
                .map_err(|e| format!("Failed to map memory: {:?}", e))?;
            let slice = std::slice::from_raw_parts_mut(data as *mut Vertex, CUBE_VERTICES.len());
            slice.copy_from_slice(&CUBE_VERTICES);
            device.unmap_memory(vertex_buffer_memory);
        }

        self.vertex_buffer = Some(vertex_buffer);
        self.vertex_buffer_memory = Some(vertex_buffer_memory);

        Ok(())
    }

    /// 查找合适的内存类型
    fn find_memory_type(&self, device: &Device, type_filter: u32, properties: vk::MemoryPropertyFlags) -> Result<u32, String> {
        let instance = self.instance.as_ref().ok_or("Instance not initialized")?;
        let physical_device = self.physical_device.ok_or("No physical device")?;

        let memory_properties = unsafe {
            instance.get_physical_device_memory_properties(physical_device)
        };

        for i in 0..memory_properties.memory_type_count {
            if (type_filter & (1 << i)) != 0 &&
                (memory_properties.memory_types[i as usize].property_flags & properties) == properties
            {
                return Ok(i);
            }
        }

        Err("Failed to find suitable memory type".to_string())
    }

    /// 创建描述符池和描述符集（用于纹理采样器）
    fn create_descriptor_resources(&mut self) -> Result<(), String> {
        let device = self.device.as_ref().ok_or("Device not initialized")?;
        // 描述符池
        let pool_sizes = [vk::DescriptorPoolSize::default()
            .ty(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
            .descriptor_count(1)];

        let descriptor_pool_create_info = vk::DescriptorPoolCreateInfo::default()
            .pool_sizes(&pool_sizes)
            .max_sets(1);

        let descriptor_pool = unsafe {
            device.create_descriptor_pool(&descriptor_pool_create_info, None)
        }
        .map_err(|e| format!("Failed to create descriptor pool: {:?}", e))?;
        self.descriptor_pool = Some(descriptor_pool);

        // 描述符集分配
        let descriptor_set_layout = self.descriptor_set_layout.unwrap();
        let descriptor_set_allocate_info = vk::DescriptorSetAllocateInfo {
            descriptor_pool,
            descriptor_set_count: 1,
            p_set_layouts: &descriptor_set_layout,
            ..Default::default()
        };

        let descriptor_sets = unsafe {
            device.allocate_descriptor_sets(&descriptor_set_allocate_info)
        }
        .map_err(|e| format!("Failed to allocate descriptor sets: {:?}", e))?;

        self.descriptor_set = Some(descriptor_sets[0]);

        Ok(())
    }

    /// 创建帧缓冲（离屏渲染目标）
    fn create_framebuffer(&mut self) -> Result<(), String> {
        let device = self.device.as_ref().ok_or("Device not initialized")?;
        // 创建离屏图像
        let image_create_info = vk::ImageCreateInfo::default()
            .image_type(vk::ImageType::TYPE_2D)
            .format(vk::Format::R8G8B8A8_UNORM)
            .extent(vk::Extent3D {
                width: self.width,
                height: self.height,
                depth: 1,
            })
            .mip_levels(1)
            .array_layers(1)
            .samples(vk::SampleCountFlags::TYPE_1)
            .tiling(vk::ImageTiling::OPTIMAL)
            .usage(vk::ImageUsageFlags::COLOR_ATTACHMENT | vk::ImageUsageFlags::TRANSFER_SRC)
            .sharing_mode(vk::SharingMode::EXCLUSIVE)
            .initial_layout(vk::ImageLayout::UNDEFINED);

        let offscreen_image = unsafe {
            device.create_image(&image_create_info, None)
        }
        .map_err(|e| format!("Failed to create offscreen image: {:?}", e))?;
        self.offscreen_image = Some(offscreen_image);

        let requirements = unsafe {
            device.get_image_memory_requirements(offscreen_image)
        };

        let memory_type_index = self.find_memory_type(
            device,
            requirements.memory_type_bits,
            vk::MemoryPropertyFlags::DEVICE_LOCAL,
        )?;

        let allocate_info = vk::MemoryAllocateInfo::default()
            .allocation_size(requirements.size)
            .memory_type_index(memory_type_index);

        let offscreen_image_memory = unsafe {
            device.allocate_memory(&allocate_info, None)
        }
        .map_err(|e| format!("Failed to allocate offscreen image memory: {:?}", e))?;

        unsafe {
            device.bind_image_memory(offscreen_image, offscreen_image_memory, 0)
        }
        .map_err(|e| format!("Failed to bind offscreen image memory: {:?}", e))?;

        self.offscreen_image_memory = Some(offscreen_image_memory);

        // 创建图像视图
        let view_create_info = vk::ImageViewCreateInfo::default()
            .image(offscreen_image)
            .view_type(vk::ImageViewType::TYPE_2D)
            .format(vk::Format::R8G8B8A8_UNORM)
            .subresource_range(vk::ImageSubresourceRange {
                aspect_mask: vk::ImageAspectFlags::COLOR,
                base_mip_level: 0,
                level_count: 1,
                base_array_layer: 0,
                layer_count: 1,
            });

        let offscreen_image_view = unsafe {
            device.create_image_view(&view_create_info, None)
        }
        .map_err(|e| format!("Failed to create offscreen image view: {:?}", e))?;
        self.offscreen_image_view = Some(offscreen_image_view);

        // 创建帧缓冲
        let framebuffer_create_info = vk::FramebufferCreateInfo {
            render_pass: self.render_pass.unwrap(),
            attachment_count: 1,
            p_attachments: &offscreen_image_view,
            width: self.width,
            height: self.height,
            layers: 1,
            ..Default::default()
        };

        let framebuffer = unsafe {
            device.create_framebuffer(&framebuffer_create_info, None)
        }
        .map_err(|e| format!("Failed to create framebuffer: {:?}", e))?;
        self.framebuffer = Some(framebuffer);

        Ok(())
    }

    /// 创建同步原语（信号量、栅栏）
    fn create_sync_resources(&mut self) -> Result<(), String> {
        let device = self.device.as_ref().ok_or("Device not initialized")?;
        let semaphore_create_info = vk::SemaphoreCreateInfo::default();

        let image_available_semaphore = unsafe {
            device.create_semaphore(&semaphore_create_info, None)
        }
        .map_err(|e| format!("Failed to create image available semaphore: {:?}", e))?;
        self.image_available_semaphore = Some(image_available_semaphore);

        let render_finished_semaphore = unsafe {
            device.create_semaphore(&semaphore_create_info, None)
        }
        .map_err(|e| format!("Failed to create render finished semaphore: {:?}", e))?;
        self.render_finished_semaphore = Some(render_finished_semaphore);

        let fence_create_info = vk::FenceCreateInfo::default()
            .flags(vk::FenceCreateFlags::SIGNALED);

        let in_flight_fence = unsafe {
            device.create_fence(&fence_create_info, None)
        }
        .map_err(|e| format!("Failed to create in flight fence: {:?}", e))?;
        self.in_flight_fence = Some(in_flight_fence);

        Ok(())
    }

    /// 分配命令缓冲
    fn allocate_command_buffers(&mut self) -> Result<(), String> {
        let device = self.device.as_ref().ok_or("Device not initialized")?;
        let alloc_info = vk::CommandBufferAllocateInfo::default()
            .command_pool(self.command_pool.unwrap())
            .level(vk::CommandBufferLevel::PRIMARY)
            .command_buffer_count(1);

        let command_buffers = unsafe {
            device.allocate_command_buffers(&alloc_info)
        }
        .map_err(|e| format!("Failed to allocate command buffers: {:?}", e))?;

        self.command_buffer = Some(command_buffers[0]);

        Ok(())
    }

    /// 从 PNG 数据创建 Vulkan 纹理
    pub fn load_texture_from_png(&mut self, png_data: &[u8]) -> Result<(), String> {
        let device = self.device.as_ref().ok_or("Device not initialized")?;

        // 使用 stb_image 解码 PNG
        let img = match load_from_memory(png_data) {
            LoadResult::ImageU8(img) => img,
            LoadResult::ImageF32(_) => return Err("PNG decoded as HDR, expected U8".to_string()),
            LoadResult::Error(e) => return Err(format!("Failed to decode PNG: {}", e)),
        };

        let img_width = img.width as u32;
        let img_height = img.height as u32;
        let img_data = img.data; // RGBA 8-bit

        // 创建暂存缓冲用于上传纹理数据
        let buffer_size = img_data.len() as u64;

        let staging_buffer_create_info = vk::BufferCreateInfo::default()
            .size(buffer_size)
            .usage(vk::BufferUsageFlags::TRANSFER_SRC)
            .sharing_mode(vk::SharingMode::EXCLUSIVE);

        let staging_buffer = unsafe {
            device.create_buffer(&staging_buffer_create_info, None)
        }
        .map_err(|e| format!("Failed to create staging buffer: {:?}", e))?;

        let requirements = unsafe {
            device.get_buffer_memory_requirements(staging_buffer)
        };

        let memory_type_index = self.find_memory_type(
            device,
            requirements.memory_type_bits,
            vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
        )?;

        let allocate_info = vk::MemoryAllocateInfo::default()
            .allocation_size(requirements.size)
            .memory_type_index(memory_type_index);

        let staging_memory = unsafe {
            device.allocate_memory(&allocate_info, None)
        }
        .map_err(|e| format!("Failed to allocate staging memory: {:?}", e))?;

        unsafe {
            device.bind_buffer_memory(staging_buffer, staging_memory, 0)
        }
        .map_err(|e| format!("Failed to bind staging buffer memory: {:?}", e))?;

        // 复制图像数据到暂存缓冲
        unsafe {
            let data = device.map_memory(staging_memory, 0, buffer_size, vk::MemoryMapFlags::empty())
                .map_err(|e| format!("Failed to map staging memory: {:?}", e))?;
            std::ptr::copy_nonoverlapping(img_data.as_ptr(), data as *mut u8, img_data.len());
            device.unmap_memory(staging_memory);
        }

        // 创建纹理图像
        let texture_image_create_info = vk::ImageCreateInfo::default()
            .image_type(vk::ImageType::TYPE_2D)
            .format(vk::Format::R8G8B8A8_UNORM)
            .extent(vk::Extent3D {
                width: img_width,
                height: img_height,
                depth: 1,
            })
            .mip_levels(1)
            .array_layers(1)
            .samples(vk::SampleCountFlags::TYPE_1)
            .tiling(vk::ImageTiling::OPTIMAL)
            .usage(vk::ImageUsageFlags::TRANSFER_DST | vk::ImageUsageFlags::SAMPLED)
            .sharing_mode(vk::SharingMode::EXCLUSIVE)
            .initial_layout(vk::ImageLayout::UNDEFINED);

        let texture_image = unsafe {
            device.create_image(&texture_image_create_info, None)
        }
        .map_err(|e| format!("Failed to create texture image: {:?}", e))?;
        self.texture_image = Some(texture_image);

        let tex_requirements = unsafe {
            device.get_image_memory_requirements(texture_image)
        };

        let tex_memory_type_index = self.find_memory_type(
            device,
            tex_requirements.memory_type_bits,
            vk::MemoryPropertyFlags::DEVICE_LOCAL,
        )?;

        let tex_allocate_info = vk::MemoryAllocateInfo::default()
            .allocation_size(tex_requirements.size)
            .memory_type_index(tex_memory_type_index);

        let texture_memory = unsafe {
            device.allocate_memory(&tex_allocate_info, None)
        }
        .map_err(|e| format!("Failed to allocate texture memory: {:?}", e))?;

        unsafe {
            device.bind_image_memory(texture_image, texture_memory, 0)
        }
        .map_err(|e| format!("Failed to bind texture image memory: {:?}", e))?;

        self.texture_image_memory = Some(texture_memory);

        // 创建纹理图像视图
        let texture_view_create_info = vk::ImageViewCreateInfo::default()
            .image(texture_image)
            .view_type(vk::ImageViewType::TYPE_2D)
            .format(vk::Format::R8G8B8A8_UNORM)
            .subresource_range(vk::ImageSubresourceRange {
                aspect_mask: vk::ImageAspectFlags::COLOR,
                base_mip_level: 0,
                level_count: 1,
                base_array_layer: 0,
                layer_count: 1,
            });

        let texture_image_view = unsafe {
            device.create_image_view(&texture_view_create_info, None)
        }
        .map_err(|e| format!("Failed to create texture image view: {:?}", e))?;
        self.texture_image_view = Some(texture_image_view);

        // 创建采样器
        let sampler_create_info = vk::SamplerCreateInfo::default()
            .mag_filter(vk::Filter::LINEAR)
            .min_filter(vk::Filter::LINEAR)
            .address_mode_u(vk::SamplerAddressMode::REPEAT)
            .address_mode_v(vk::SamplerAddressMode::REPEAT)
            .address_mode_w(vk::SamplerAddressMode::REPEAT)
            .anisotropy_enable(true)
            .max_anisotropy(16.0)
            .border_color(vk::BorderColor::INT_OPAQUE_BLACK)
            .unnormalized_coordinates(false)
            .compare_enable(false)
            .mipmap_mode(vk::SamplerMipmapMode::LINEAR);

        let texture_sampler = unsafe {
            device.create_sampler(&sampler_create_info, None)
        }
        .map_err(|e| format!("Failed to create texture sampler: {:?}", e))?;
        self.texture_sampler = Some(texture_sampler);

        // 更新描述符集（绑定纹理图像视图和采样器）
        let descriptor_image_info = vk::DescriptorImageInfo::default()
            .image_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL)
            .image_view(texture_image_view)
            .sampler(texture_sampler);

        let write_descriptor_sets = [vk::WriteDescriptorSet {
            dst_set: self.descriptor_set.unwrap(),
            dst_binding: 0,
            dst_array_element: 0,
            descriptor_count: 1,
            descriptor_type: vk::DescriptorType::COMBINED_IMAGE_SAMPLER,
            p_image_info: &descriptor_image_info,
            ..Default::default()
        }];

        unsafe {
            device.update_descriptor_sets(&write_descriptor_sets, &[]);
        }

        // 清理暂存缓冲
        unsafe {
            device.destroy_buffer(staging_buffer, None);
            device.free_memory(staging_memory, None);
        }

        Ok(())
    }

    /// 渲染一帧 3D 皮肤模型，输出 RGBA 数据到 out_data
    pub fn render_frame(
        &mut self,
        skin_data: &[u8],
        out_data: &mut [u8],
    ) -> Result<usize, String> {
        if !self.initialized {
            return Err("Vulkan renderer not initialized".to_string());
        }

        // 如果有皮肤数据，先上传纹理（需要可变借用 self）
        if !skin_data.is_empty() {
            self.load_texture_from_png(skin_data)?;
        }

        // 获取所有需要的引用（在可变借用之后）
        let device = self.device.as_ref().ok_or("Device not initialized")?;
        let queue = self.queue.ok_or("Queue not initialized")?;
        let command_pool = self.command_pool.ok_or("Command pool not initialized")?;
        let render_pass = self.render_pass.ok_or("Render pass not initialized")?;
        let framebuffer = self.framebuffer.ok_or("Framebuffer not initialized")?;
        let pipeline = self.pipeline.ok_or("Pipeline not initialized")?;
        let pipeline_layout = self.pipeline_layout.ok_or("Pipeline layout not initialized")?;
        let vertex_buffer = self.vertex_buffer.ok_or("Vertex buffer not initialized")?;
        let descriptor_set = self.descriptor_set.ok_or("Descriptor set not initialized")?;
        let command_buffer = self.command_buffer.ok_or("Command buffer not initialized")?;
        let in_flight_fence = self.in_flight_fence.ok_or("Fence not initialized")?;

        // 等待之前的帧完成
        unsafe {
            device.wait_for_fences(&[in_flight_fence], true, u64::MAX)
                .map_err(|e| format!("Failed to wait for fences: {:?}", e))?;
        }

        // 重置栅栏
        unsafe {
            device.reset_fences(&[in_flight_fence])
                .map_err(|e| format!("Failed to reset fences: {:?}", e))?;
        }

        // 开始记录命令缓冲
        unsafe {
            device.reset_command_pool(command_pool, vk::CommandPoolResetFlags::empty())
                .map_err(|e| format!("Failed to reset command pool: {:?}", e))?;
        }

        let cmd_begin_info = vk::CommandBufferBeginInfo::default()
            .flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT);

        unsafe {
            device.begin_command_buffer(command_buffer, &cmd_begin_info)
                .map_err(|e| format!("Failed to begin command buffer: {:?}", e))?;
        }

        // 开始渲染 pass
        let clear_values = [vk::ClearValue {
            color: vk::ClearColorValue { float32: [0.1, 0.1, 0.18, 1.0] },
        }];

        let render_pass_begin_info = vk::RenderPassBeginInfo {
            render_pass,
            framebuffer,
            render_area: vk::Rect2D {
                offset: vk::Offset2D { x: 0, y: 0 },
                extent: vk::Extent2D { width: self.width, height: self.height },
            },
            clear_value_count: 1,
            p_clear_values: clear_values.as_ptr(),
            ..Default::default()
        };

        unsafe {
            device.cmd_begin_render_pass(
                command_buffer,
                &render_pass_begin_info,
                vk::SubpassContents::INLINE,
            );
        }

        // 绑定管线和描述符集
        unsafe {
            device.cmd_bind_pipeline(
                command_buffer,
                vk::PipelineBindPoint::GRAPHICS,
                pipeline,
            );
            device.cmd_bind_descriptor_sets(
                command_buffer,
                vk::PipelineBindPoint::GRAPHICS,
                pipeline_layout,
                0,
                &[descriptor_set],
                &[],
            );
        }

        // 绑定顶点缓冲并绘制
        unsafe {
            device.cmd_bind_vertex_buffers(
                command_buffer,
                0,
                &[vertex_buffer],
                &[0],
            );
            device.cmd_draw(command_buffer, CUBE_VERTICES.len() as u32, 1, 0, 0);
        }

        // 结束渲染 pass
        unsafe {
            device.cmd_end_render_pass(command_buffer);
        }

        // 结束命令缓冲
        unsafe {
            device.end_command_buffer(command_buffer)
                .map_err(|e| format!("Failed to end command buffer: {:?}", e))?;
        }

        // 提交命令缓冲
        let submit_info = vk::SubmitInfo {
            command_buffer_count: 1,
            p_command_buffers: &command_buffer,
            ..Default::default()
        };

        unsafe {
            device.queue_submit(queue, &[submit_info], in_flight_fence)
                .map_err(|e| format!("Failed to submit to queue: {:?}", e))?;
        }

        // 等待渲染完成
        unsafe {
            device.queue_wait_idle(queue)
                .map_err(|e| format!("Failed to wait idle: {:?}", e))?;
        }

        // 从离屏图像读取像素数据到 out_data
        self.read_pixels(device, queue, command_pool, command_buffer, out_data)?;

        Ok((self.width * self.height * 4) as usize)
    }

    /// 从离屏图像读取像素数据
    fn read_pixels(
        &self,
        device: &Device,
        queue: vk::Queue,
        command_pool: vk::CommandPool,
        command_buffer: vk::CommandBuffer,
        out_data: &mut [u8],
    ) -> Result<(), String> {
        let pixel_count = (self.width * self.height) as usize;
        let bytes_needed = pixel_count * 4;

        if out_data.len() < bytes_needed {
            return Err(format!(
                "Output buffer too small: need {}, got {}",
                bytes_needed,
                out_data.len()
            ));
        }

        // 创建暂存缓冲用于读取像素
        let buffer_create_info = vk::BufferCreateInfo::default()
            .size(bytes_needed as u64)
            .usage(vk::BufferUsageFlags::TRANSFER_DST)
            .sharing_mode(vk::SharingMode::EXCLUSIVE);

        let readback_buffer = unsafe {
            device.create_buffer(&buffer_create_info, None)
        }
        .map_err(|e| format!("Failed to create readback buffer: {:?}", e))?;

        let requirements = unsafe {
            device.get_buffer_memory_requirements(readback_buffer)
        };

        let memory_type_index = self.find_memory_type(
            device,
            requirements.memory_type_bits,
            vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
        )?;

        let allocate_info = vk::MemoryAllocateInfo::default()
            .allocation_size(requirements.size)
            .memory_type_index(memory_type_index);

        let readback_memory = unsafe {
            device.allocate_memory(&allocate_info, None)
        }
        .map_err(|e| format!("Failed to allocate readback memory: {:?}", e))?;

        unsafe {
            device.bind_buffer_memory(readback_buffer, readback_memory, 0)
        }
        .map_err(|e| format!("Failed to bind readback buffer memory: {:?}", e))?;

        // 开始命令缓冲（用于图像到缓冲的拷贝）
        unsafe {
            device.reset_command_pool(command_pool, vk::CommandPoolResetFlags::empty())
                .map_err(|e| format!("Failed to reset command pool: {:?}", e))?;
        }

        let cmd_begin_info = vk::CommandBufferBeginInfo::default()
            .flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT);

        unsafe {
            device.begin_command_buffer(command_buffer, &cmd_begin_info)
                .map_err(|e| format!("Failed to begin command buffer: {:?}", e))?;
        }

        // 将离屏图像的布局转换为 TRANSFER_SRC_OPTIMAL
        let image_memory_barrier = vk::ImageMemoryBarrier::default()
            .old_layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL)
            .new_layout(vk::ImageLayout::TRANSFER_SRC_OPTIMAL)
            .src_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
            .dst_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
            .image(self.offscreen_image.unwrap())
            .subresource_range(vk::ImageSubresourceRange {
                aspect_mask: vk::ImageAspectFlags::COLOR,
                base_mip_level: 0,
                level_count: 1,
                base_array_layer: 0,
                layer_count: 1,
            })
            .src_access_mask(vk::AccessFlags::COLOR_ATTACHMENT_WRITE)
            .dst_access_mask(vk::AccessFlags::TRANSFER_READ);

        unsafe {
            device.cmd_pipeline_barrier(
                command_buffer,
                vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
                vk::PipelineStageFlags::TRANSFER,
                vk::DependencyFlags::empty(),
                &[],
                &[],
                &[image_memory_barrier],
            );
        }

        // 拷贝图像到缓冲
        let buffer_image_copy = vk::BufferImageCopy::default()
            .buffer_offset(0)
            .buffer_row_length(0)
            .buffer_image_height(0)
            .image_subresource(vk::ImageSubresourceLayers {
                aspect_mask: vk::ImageAspectFlags::COLOR,
                mip_level: 0,
                base_array_layer: 0,
                layer_count: 1,
            })
            .image_offset(vk::Offset3D { x: 0, y: 0, z: 0 })
            .image_extent(vk::Extent3D {
                width: self.width,
                height: self.height,
                depth: 1,
            });

        unsafe {
            device.cmd_copy_image_to_buffer(
                command_buffer,
                self.offscreen_image.unwrap(),
                vk::ImageLayout::TRANSFER_SRC_OPTIMAL,
                readback_buffer,
                &[buffer_image_copy],
            );
        }

        // 将图像布局转换回 COLOR_ATTACHMENT_OPTIMAL
        let image_memory_barrier_back = vk::ImageMemoryBarrier::default()
            .old_layout(vk::ImageLayout::TRANSFER_SRC_OPTIMAL)
            .new_layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL)
            .src_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
            .dst_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
            .image(self.offscreen_image.unwrap())
            .subresource_range(vk::ImageSubresourceRange {
                aspect_mask: vk::ImageAspectFlags::COLOR,
                base_mip_level: 0,
                level_count: 1,
                base_array_layer: 0,
                layer_count: 1,
            })
            .src_access_mask(vk::AccessFlags::TRANSFER_READ)
            .dst_access_mask(vk::AccessFlags::COLOR_ATTACHMENT_WRITE);

        unsafe {
            device.cmd_pipeline_barrier(
                command_buffer,
                vk::PipelineStageFlags::TRANSFER,
                vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
                vk::DependencyFlags::empty(),
                &[],
                &[],
                &[image_memory_barrier_back],
            );
        }

        unsafe {
            device.end_command_buffer(command_buffer)
                .map_err(|e| format!("Failed to end command buffer: {:?}", e))?;
        }

        // 提交命令缓冲
        let submit_info = vk::SubmitInfo {
            command_buffer_count: 1,
            p_command_buffers: &command_buffer,
            ..Default::default()
        };

        unsafe {
            device.queue_submit(queue, &[submit_info], vk::Fence::null())
                .map_err(|e| format!("Failed to submit to queue: {:?}", e))?;
            device.queue_wait_idle(queue)
                .map_err(|e| format!("Failed to wait idle: {:?}", e))?;
        }

        // 从缓冲读取像素数据
        unsafe {
            let data = device.map_memory(readback_memory, 0, bytes_needed as u64, vk::MemoryMapFlags::empty())
                .map_err(|e| format!("Failed to map readback memory: {:?}", e))?;
            std::ptr::copy_nonoverlapping(data as *const u8, out_data.as_mut_ptr(), bytes_needed);
            device.unmap_memory(readback_memory);
        }

        // 清理暂存缓冲
        unsafe {
            device.destroy_buffer(readback_buffer, None);
            device.free_memory(readback_memory, None);
        }

        Ok(())
    }

    pub fn cleanup(&mut self) {
        let device = match self.device.take() {
            Some(d) => d,
            None => return,
        };

        // 销毁所有资源
        unsafe {
            if let Some(fence) = self.in_flight_fence.take() {
                device.destroy_fence(fence, None);
            }
            if let Some(semaphore) = self.render_finished_semaphore.take() {
                device.destroy_semaphore(semaphore, None);
            }
            if let Some(semaphore) = self.image_available_semaphore.take() {
                device.destroy_semaphore(semaphore, None);
            }
            if let Some(cmd_buf) = self.command_buffer.take() {
                device.free_command_buffers(self.command_pool.unwrap(), &[cmd_buf]);
            }
            if let Some(framebuffer) = self.framebuffer.take() {
                device.destroy_framebuffer(framebuffer, None);
            }
            if let Some(image_view) = self.offscreen_image_view.take() {
                device.destroy_image_view(image_view, None);
            }
            if let Some(image) = self.offscreen_image.take() {
                device.destroy_image(image, None);
            }
            if let Some(memory) = self.offscreen_image_memory.take() {
                device.free_memory(memory, None);
            }
            if let Some(sampler) = self.texture_sampler.take() {
                device.destroy_sampler(sampler, None);
            }
            if let Some(image_view) = self.texture_image_view.take() {
                device.destroy_image_view(image_view, None);
            }
            if let Some(image) = self.texture_image.take() {
                device.destroy_image(image, None);
            }
            if let Some(memory) = self.texture_image_memory.take() {
                device.free_memory(memory, None);
            }
            if let Some(descriptor_pool) = self.descriptor_pool.take() {
                device.destroy_descriptor_pool(descriptor_pool, None);
            }
            if let Some(descriptor_set_layout) = self.descriptor_set_layout.take() {
                device.destroy_descriptor_set_layout(descriptor_set_layout, None);
            }
            if let Some(vertex_buffer) = self.vertex_buffer.take() {
                device.destroy_buffer(vertex_buffer, None);
            }
            if let Some(vertex_buffer_memory) = self.vertex_buffer_memory.take() {
                device.free_memory(vertex_buffer_memory, None);
            }
            if let Some(pipeline) = self.pipeline.take() {
                device.destroy_pipeline(pipeline, None);
            }
            if let Some(pipeline_layout) = self.pipeline_layout.take() {
                device.destroy_pipeline_layout(pipeline_layout, None);
            }
            if let Some(render_pass) = self.render_pass.take() {
                device.destroy_render_pass(render_pass, None);
            }
            if let Some(command_pool) = self.command_pool.take() {
                device.destroy_command_pool(command_pool, None);
            }
            device.destroy_device(None);
        }

        if let Some(instance) = self.instance.take() {
            unsafe { instance.destroy_instance(None); }
        }

        self.initialized = false;
    }
}

impl Drop for VulkanRenderer {
    fn drop(&mut self) {
        self.cleanup();
    }
}

impl Default for VulkanRenderer {
    fn default() -> Self {
        Self::new()
    }
}

/// ==================== C ABI 出口函数 ====================

#[no_mangle]
pub extern "C" fn vulkan_renderer_new() -> *mut VulkanRenderer {
    Box::into_raw(Box::new(VulkanRenderer::new()))
}

#[no_mangle]
pub extern "C" fn vulkan_renderer_init(
    renderer: *mut VulkanRenderer,
    width: c_int,
    height: c_int,
) -> c_int {
    if renderer.is_null() {
        return -1;
    }
    let renderer = unsafe { &mut *renderer };
    match renderer.init(width as u32, height as u32) {
        Ok(()) => 0,
        Err(e) => {
            eprintln!("Vulkan init error: {}", e);
            -1
        }
    }
}

#[no_mangle]
pub extern "C" fn vulkan_renderer_render_frame(
    renderer: *mut VulkanRenderer,
    skin_data: *const u8,
    skin_len: c_int,
    width: c_int,
    height: c_int,
    out_data: *mut u8,
    out_len: c_int,
) -> c_int {
    if renderer.is_null() || skin_data.is_null() || out_data.is_null() {
        return -1;
    }

    let renderer = unsafe { &mut *renderer };
    let skin_slice = unsafe { std::slice::from_raw_parts(skin_data, skin_len as usize) };
    let out_slice = unsafe { std::slice::from_raw_parts_mut(out_data, out_len as usize) };

    match renderer.render_frame(skin_slice, out_slice) {
        Ok(bytes) => bytes as c_int,
        Err(e) => {
            eprintln!("Vulkan render error: {}", e);
            -1
        }
    }
}

#[no_mangle]
pub extern "C" fn vulkan_renderer_free(renderer: *mut VulkanRenderer) {
    if !renderer.is_null() {
        unsafe {
            let _ = Box::from_raw(renderer);
        }
    }
}

#[no_mangle]
pub extern "C" fn vulkan_renderer_version() -> *mut c_char {
    CString::new(env!("CARGO_PKG_VERSION")).unwrap().into_raw()
}

