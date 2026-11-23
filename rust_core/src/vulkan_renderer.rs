//! Vulkan 3D 皮肤模型渲染器
//! 使用 ash crate 绑定 Vulkan API，渲染 Minecraft 玩家皮肤模型

use ash::{vk, Instance, Device};
use std::ffi::CString;
use std::os::raw::c_char;
use std::os::raw::c_int;

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
}

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
        }
    }

    /// 初始化 Vulkan 实例和设备
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
        device_create_info.enabled_extension_count = 0;
        device_create_info.pp_enabled_extension_names = std::ptr::null();

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
        self.initialized = true;

        Ok(())
    }

    /// 渲染一帧 3D 皮肤模型，输出 RGBA 数据到 out_data
    pub fn render_frame(
        &self,
        skin_data: &[u8],
        out_data: &mut [u8],
    ) -> Result<usize, String> {
        if !self.initialized {
            return Err("Vulkan renderer not initialized".to_string());
        }

        let device = self.device.as_ref()
            .ok_or("Device not initialized")?;
        let queue = self.queue.ok_or("Queue not initialized")?;
        let command_pool = self.command_pool.ok_or("Command pool not initialized")?;

        // 分配命令缓冲区
        let mut cmd_buf_alloc_info = vk::CommandBufferAllocateInfo::default();
        cmd_buf_alloc_info.command_pool = command_pool;
        cmd_buf_alloc_info.level = vk::CommandBufferLevel::PRIMARY;
        cmd_buf_alloc_info.command_buffer_count = 1;

        let cmd_buffers = unsafe {
            device.allocate_command_buffers(&cmd_buf_alloc_info)
        }
        .map_err(|e| format!("Failed to allocate command buffers: {:?}", e))?;

        let cmd_buf = cmd_buffers[0];

        // 开始记录命令缓冲区
        let mut cmd_begin_info = vk::CommandBufferBeginInfo::default();
        cmd_begin_info.flags = vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT;

        unsafe {
            device.begin_command_buffer(cmd_buf, &cmd_begin_info)
        }
        .map_err(|e| format!("Failed to begin command buffer: {:?}", e))?;

        // 简化实现：直接填充颜色数据（实际需要完整的渲染管线）
        let pixel_count = (self.width * self.height) as usize;
        let bytes_needed = pixel_count * 4;

        if out_data.len() < bytes_needed {
            return Err(format!(
                "Output buffer too small: need {}, got {}",
                bytes_needed,
                out_data.len()
            ));
        }

        // 填充占位颜色（深蓝紫色背景，对应我们项目的渐变色）
        for i in (0..bytes_needed).step_by(4) {
            out_data[i] = 0x1a;     // R
            out_data[i + 1] = 0x1a; // G
            out_data[i + 2] = 0x2e; // B
            out_data[i + 3] = 0xff; // A
        }

        // 提交命令缓冲区
        let mut submit_info = vk::SubmitInfo::default();
        submit_info.command_buffer_count = 1;
        submit_info.p_command_buffers = &cmd_buf;

        unsafe {
            device.end_command_buffer(cmd_buf)
        }
        .map_err(|e| format!("Failed to end command buffer: {:?}", e))?;

        unsafe {
            device.queue_submit(queue, std::slice::from_ref(&submit_info), vk::Fence::null())
        }
        .map_err(|e| format!("Failed to submit to queue: {:?}", e))?;

        unsafe {
            device.queue_wait_idle(queue)
        }
        .map_err(|e| format!("Failed to wait idle: {:?}", e))?;

        Ok(bytes_needed)
    }

    pub fn cleanup(&mut self) {
        if let Some(device) = self.device.take() {
            if let Some(pool) = self.command_pool.take() {
                unsafe { device.destroy_command_pool(pool, None); }
            }
            unsafe { device.destroy_device(None); }
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

/// 创建 Vulkan 渲染器实例，返回指针（供 FFI 调用）
#[no_mangle]
pub extern "C" fn vulkan_renderer_new() -> *mut VulkanRenderer {
    Box::into_raw(Box::new(VulkanRenderer::new()))
}

/// 初始化 Vulkan 渲染器
/// 返回：0 成功，-1 失败
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
        Ok(_) => 0,
        Err(_) => -1,
    }
}

/// 渲染一帧 3D 皮肤模型
/// skin_data: 皮肤 PNG 数据（可用于纹理）
/// skin_len: 皮肤数据长度
/// width/height: 渲染尺寸
/// out_data: 输出 RGBA 缓冲区
/// out_len: 输出缓冲区大小
/// 返回：写入的字节数，失败返回 -1
#[no_mangle]
pub extern "C" fn vulkan_renderer_render_frame(
    renderer: *mut VulkanRenderer,
    skin_data: *const u8,
    skin_len: c_int,
    _width: c_int,
    _height: c_int,
    out_data: *mut u8,
    out_len: c_int,
) -> c_int {
    if renderer.is_null() || out_data.is_null() {
        return -1;
    }
    let renderer = unsafe { &mut *renderer };

    let skin_slice = if !skin_data.is_null() {
        unsafe { std::slice::from_raw_parts(skin_data, skin_len.max(0) as usize) }
    } else {
        &[]
    };

    let out_slice = unsafe { std::slice::from_raw_parts_mut(out_data, out_len.max(0) as usize) };

    match renderer.render_frame(skin_slice, out_slice) {
        Ok(bytes) => bytes as c_int,
        Err(_) => -1,
    }
}

/// 销毁 Vulkan 渲染器
#[no_mangle]
pub extern "C" fn vulkan_renderer_free(renderer: *mut VulkanRenderer) {
    if !renderer.is_null() {
        unsafe {
            let _ = Box::from_raw(renderer);
        }
    }
}

/// 获取 Vulkan 渲染器版本字符串
#[no_mangle]
pub extern "C" fn vulkan_renderer_version() -> *mut c_char {
    CString::new(env!("CARGO_PKG_VERSION"))
        .unwrap()
        .into_raw()
}