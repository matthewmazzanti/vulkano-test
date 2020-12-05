use vulkano::{
    buffer::{ BufferUsage, CpuAccessibleBuffer as CpuBuf },
    command_buffer::{ AutoCommandBufferBuilder,DynamicState },
    device::{ Device, DeviceExtensions, Queue },
    descriptor::PipelineLayoutAbstract,
    framebuffer::{
        Framebuffer,
        FramebufferAbstract,
        RenderPassAbstract,
        Subpass
    },
    image::{ ImageUsage, SwapchainImage },
    instance::{ Instance, PhysicalDevice },
    pipeline::{
        viewport::Viewport,
        GraphicsPipeline,
        vertex::OneVertexOneInstanceDefinition,
    },
    swapchain,
    swapchain::{
        AcquireError,
        ColorSpace,
        FullscreenExclusive,
        PresentMode,
        Surface,
        SurfaceTransform,
        Swapchain,
        SwapchainCreationError,
    },
    sync,
    sync::{FlushError, GpuFuture},
};

use vulkano_win::VkSurfaceBuild;
use winit::{
    event_loop::EventLoop,
    window::{Window, WindowBuilder},
};

use std::sync::Arc;


mod vs {
    vulkano_shaders::shader! {
        ty: "vertex",
        path: "src/shaders/vertex.glsl"
    }
}

mod fs {
    vulkano_shaders::shader! {
        ty: "fragment",
        path: "src/shaders/fragment.glsl"
    }
}

fn mk_shaders(device: Arc<Device>) -> (vs::Shader, fs::Shader) {
    let vs = vs::Shader::load(device.clone()).unwrap();
    let fs = fs::Shader::load(device).unwrap();

    (vs, fs)
}

#[derive(Default, Debug, Clone)]
pub struct Vertex {
    pos: [f32; 2],
}

vulkano::impl_vertex!(Vertex, pos);

fn mk_vert_buf(device: Arc<Device>) -> Arc<CpuBuf<[Vertex]>> {
    let vertices = [
        Vertex { pos: [ 0.0, -0.5] },
        Vertex { pos: [ 0.0,  0.0] },
        Vertex { pos: [-0.5,  0.5] },
        Vertex { pos: [ 0.0, -0.5] },
        Vertex { pos: [ 0.0,  0.0] },
        Vertex { pos: [ 0.5,  0.5] },
    ].iter().cloned();

    CpuBuf::from_iter(
        device,
        BufferUsage::all(),
        false,
        vertices
    ).unwrap()
}


#[derive(Default, Debug, Clone)]
pub struct InstanceData {
    pub pos_offset: [f32; 2],
    pub angle: f32,
    pub scale: f32,
}

vulkano::impl_vertex!(InstanceData, pos_offset, angle, scale);

pub fn mk_inst_buf(device: Arc<Device>, data: Vec<InstanceData>) ->
    Arc<CpuBuf<[InstanceData]>>
{
    CpuBuf::from_iter(
        device,
        BufferUsage::all(),
        false,
        data.iter().cloned(),
    ).unwrap()
}

pub struct Renderer {
    pub instance: Arc<Instance>,
    pub phy_index: usize,
    pub device: Arc<Device>,
    pub queue: Arc<Queue>,
    pub surface: Arc<Surface<Window>>,
    pub swapchain: Arc<Swapchain<Window>>,
    pub images: Vec<Arc<SwapchainImage<Window>>>,
    pub render_pass: Arc<dyn RenderPassAbstract + Send + Sync>,
    pub vert_buf: Arc<CpuBuf<[Vertex]>>,
    pub vs: vs::Shader,
    pub fs: fs::Shader,
    pub pipeline: MyPipeline,
    pub dynamic_state: DynamicState,
    pub framebuffers: Vec<Arc<dyn FramebufferAbstract + Send + Sync>>,
    pub recreate_swapchain: bool,
    pub previous_frame_end: Option<Box<dyn GpuFuture>>,
}

impl Renderer {
    pub fn new(event_loop: &EventLoop<()>) -> Self {
        let instance = {
            let extensions = vulkano_win::required_extensions();
            Instance::new(None, &extensions, None).unwrap()
        };

        let physical = PhysicalDevice::enumerate(&instance)
            .next()
            .expect("No available device");

        let surface = WindowBuilder::new()
            .build_vk_surface(event_loop, instance.clone())
            .unwrap();

        let (device, queue) = mk_device(physical, &surface);

        // Before we can draw on the surface, we have to create what is called a
        // swapchain. Creating a swapchain allocates the color buffers that will
        // contain the image that will ultimately be visible on the screen.
        // These images are returned alongside with the swapchain.
        let (swapchain, images) = mk_swapchain(
            physical,
            surface.clone(),
            device.clone(),
            queue.clone()
        );

        // At this point, OpenGL initialization would be finished. However in
        // Vulkan it is not. OpenGL implicitly does a lot of computation
        // whenever you draw.  In Vulkan, you have to do all this manually.

        // The next step is to create a *render pass*, which is an object that
        // describes where the output of the graphics pipeline will go. It
        // describes the layout of the images where the colors, depth and/or
        // stencil information will be written.
        let render_pass = mk_render_pass(device.clone(), swapchain.clone());

        let vert_buf = mk_vert_buf(device.clone());

        let (vs, fs) = mk_shaders(device.clone());

        // Before we draw we have to create what is called a pipeline. This is
        // similar to an OpenGL program, but much more specific.
        let pipeline = mk_pipeline(
            device.clone(),
            render_pass.clone(),
            &vs,
            &fs
        );

        // Dynamic viewports allow us to recreate just the viewport when the
        // window is resized, otherwise we would have to recreate the whole
        // pipeline.
        let mut dynamic_state = DynamicState::none();

        // The render pass we created above only describes the layout of our
        // framebuffers. Before we can draw we also need to create the actual
        // framebuffers.
        //
        // Since we need to draw to multiple images, we are going to create a
        // different framebuffer for each image.
        let framebuffers = window_size_dependent_setup(
            &images,
            render_pass.clone(),
            &mut dynamic_state
        );

        let phy_index = physical.index();
        let recreate_swapchain = false;
        let previous_frame_end = Some(sync::now(device.clone()).boxed());

        Renderer {
            instance,
            phy_index,
            device,
            queue,
            surface,
            swapchain,
            images,
            render_pass,
            vert_buf,
            vs,
            fs,
            pipeline,
            dynamic_state,
            framebuffers,
            recreate_swapchain,
            previous_frame_end,
        }
    }

    #[allow(dead_code)]
    pub fn physical(&self) -> PhysicalDevice<'_> {
        PhysicalDevice::from_index(&self.instance, self.phy_index)
            .expect("Unable to find physical device")
    }

    pub fn recreate_swapchain(&mut self) {
        // Get the new dimensions of the window.
        let dimensions: [u32; 2] = self.surface.window().inner_size().into();

        let (new_swapchain, new_images) =
            match self.swapchain.recreate_with_dimensions(dimensions) {
                Ok(r) => r,
                // This error tends to happen when the user is manually resizing
                // the window.  Simply restarting the loop is the easiest way to
                // fix this issue.
                Err(SwapchainCreationError::UnsupportedDimensions) => return,
                Err(e) => panic!("Failed to recreate swapchain: {:?}", e),
            };

        self.swapchain = new_swapchain;

        // Because framebuffers contains an Arc on the old swapchain, we need to
        // recreate framebuffers as well.
        self.framebuffers = window_size_dependent_setup(
            &new_images,
            self.render_pass.clone(),
            &mut self.dynamic_state,
        );

        self.recreate_swapchain = false;
    }

    pub fn redraw(&mut self, data: Vec<InstanceData>) {
        // It is important to call this function from time to time, otherwise
        // resources will keep accumulating and you will eventually reach an out
        // of memory error.  Calling this function polls various fences in order
        // to determine what the GPU has already processed, and frees the
        // resources that are no longer needed.
        self.previous_frame_end.as_mut().unwrap().cleanup_finished();

        // Whenever the window resizes we need to recreate everything dependent
        // on the window size.  In this example that includes the swapchain, the
        // framebuffers and the dynamic state viewport.
        if self.recreate_swapchain {
            // Get the new dimensions of the window.
            self.recreate_swapchain();
        }

        // Before we can draw on the output, we have to *acquire* an image from
        // the swapchain. If no image is available (which happens if you submit
        // draw commands too quickly), then the function will block.  This
        // operation returns the index of the image that we are allowed to draw
        // upon.
        //
        // This function can block if no image is available. The parameter is an
        // optional timeout after which the function call will return an error.
        let (image_num, suboptimal, acquire_future) =
            match swapchain::acquire_next_image(self.swapchain.clone(), None) {
                Ok(r) => r,
                Err(AcquireError::OutOfDate) => {
                    self.recreate_swapchain = true;
                    return;
                }
                Err(e) => panic!("Failed to acquire next image: {:?}", e),
            };

        // acquire_next_image can be successful, but suboptimal. This means that
        // the swapchain image will still work, but it may not display
        // correctly. With some drivers this can be when the window resizes, but
        // it may not cause the swapchain to become out of date.
        if suboptimal {
            self.recreate_swapchain = true;
        }

        // Specify the color to clear the framebuffer with i.e. blue
        let clear_values = vec![[0.0, 0.0, 0.0, 1.0].into()];

        // In order to draw, we have to build a *command buffer*. The command
        // buffer object holds the list of commands that are going to be
        // executed.
        //
        // Building a command buffer is an expensive operation (usually a few
        // hundred microseconds), but it is known to be a hot path in the driver
        // and is expected to be optimized.
        //
        // Note that we have to pass a queue family when we create the command
        // buffer. The command buffer will only be executable on that given
        // queue family.
        let mut builder = AutoCommandBufferBuilder::primary_one_time_submit(
            self.device.clone(),
            self.queue.family(),
        ).unwrap();

        builder
            // Before we can draw, we have to *enter a render pass*. There are
            // two methods to do this: `draw_inline` and `draw_secondary`. The
            // latter is a bit more advanced and is not covered here.
            //
            // The third parameter builds the list of values to clear the
            // attachments with. The API is similar to the list of attachments
            // when building the framebuffers, except that only the attachments
            // that use `load: Clear` appear in the list.
            .begin_render_pass(
                self.framebuffers[image_num].clone(),
                false,
                clear_values,
            ).unwrap()
            // We are now inside the first subpass of the render pass. We add a
            // draw command.
            //
            // The last two parameters contain the list of resources to pass to
            // the shaders.  Since we used an `EmptyPipeline` object, the
            // objects have to be `()`.
            .draw(
                self.pipeline.clone(),
                &self.dynamic_state,
                (self.vert_buf.clone(), mk_inst_buf(self.device.clone(), data)),
                (),
                (),
            ).unwrap()
            // We leave the render pass by calling `draw_end`. Note that if we
            // had multiple subpasses we could have called `next_inline` (or
            // `next_secondary`) to jump to the next subpass.
            .end_render_pass().unwrap();

        // Finish building the command buffer by calling `build`.
        let command_buffer = builder.build().unwrap();

        let future = self.previous_frame_end
            .take().unwrap()
            .join(acquire_future)
            .then_execute(self.queue.clone(), command_buffer).unwrap()
            // The color output is now expected to contain our triangle. But in
            // order to show it on the screen, we have to *present* the image by
            // calling `present`.
            //
            // This function does not actually present the image immediately.
            // Instead it submits a present command at the end of the queue.
            // This means that it will only be presented once the GPU has
            // finished executing the command buffer that draws the triangle.
            .then_swapchain_present(
                self.queue.clone(),
                self.swapchain.clone(),
                image_num
            )
            .then_signal_fence_and_flush();

        self.previous_frame_end = match future {
            Ok(future) => Some(future.boxed()),
            Err(FlushError::OutOfDate) => {
                self.recreate_swapchain = true;
                Some(sync::now(self.device.clone()).boxed())
            }
            Err(e) => {
                println!("Failed to flush future: {:?}", e);
                Some(sync::now(self.device.clone()).boxed())
            }
        };
    }
}

fn mk_device(
    physical: PhysicalDevice<'_>,
    surface: &Arc<Surface<Window>>
) -> (
    Arc<Device>,
    Arc<Queue>
) {
    let family = physical
        .queue_families()
        .find(|&q| {
            q.supports_graphics()
                && surface.is_supported(q).unwrap_or(false)
        }).unwrap();

    let device_ext = DeviceExtensions {
        khr_swapchain: true,
        ..DeviceExtensions::none()
    };

    let (device, mut queues) = Device::new(
        physical,
        physical.supported_features(),
        &device_ext,
        [(family, 0.5)].iter().cloned(),
    ).unwrap();

    (device, queues.next().unwrap())
}


fn mk_swapchain(
    physical: PhysicalDevice<'_>,
    surface: Arc<Surface<Window>>,
    device: Arc<Device>,
    queue: Arc<Queue>,
) -> (
    Arc<Swapchain<Window>>,
    Vec<Arc<SwapchainImage<Window>>>
) {
    // Querying the capabilities of the surface. When we create the
    // swapchain we can only pass values that are allowed by the
    // capabilities.
    let caps = surface.capabilities(physical).unwrap();

    // The alpha mode indicates how the alpha value of the final image will
    // behave. For example you can choose whether the window will be opaque
    // or transparent.
    let alpha = caps.supported_composite_alpha.iter().next().unwrap();

    // Choosing the internal format that the images will have.
    let format = caps.supported_formats[0].0;

    // The dimensions of the window, only used to initially setup the
    // swapchain.  NOTE: On some drivers the swapchain dimensions are
    // specified by `caps.current_extent` and the swapchain size must use
    // these dimensions.  These dimensions are always the same as the window
    // dimensions
    //
    // However other drivers dont specify a value i.e. `caps.current_extent`
    // is `None` These drivers will allow anything but the only sensible
    // value is the window dimensions.
    //
    // Because for both of these cases, the swapchain needs to be the window
    // dimensions, we just use that.
    let dimensions: [u32; 2] = surface.window().inner_size().into();

    // Please take a look at the docs for the meaning of the parameters we
    // didn't mention.
    Swapchain::new(
        device,
        surface,
        caps.min_image_count,
        format,
        dimensions,
        1,
        ImageUsage::color_attachment(),
        &queue,
        SurfaceTransform::Identity,
        alpha,
        PresentMode::Fifo,
        FullscreenExclusive::Default,
        true,
        ColorSpace::SrgbNonLinear,
    ).unwrap()
}

fn mk_render_pass(device: Arc<Device>, swapchain: Arc<Swapchain<Window>>) ->
    Arc<dyn RenderPassAbstract + Send + Sync>
{
    Arc::new(
        vulkano::single_pass_renderpass!(
            device,
            attachments: {
                color: {
                    load: Clear,
                    store: Store,
                    format: swapchain.format(),
                    samples: 1,
                }
            },
            pass: {
                color: [color],
                depth_stencil: {}
            }
        )
        .unwrap(),
    )
}

type MyPipeline = Arc<GraphicsPipeline<
    OneVertexOneInstanceDefinition<Vertex, InstanceData>,
    Box<dyn PipelineLayoutAbstract + Send + Sync>,
    Arc<dyn RenderPassAbstract + Send + Sync>>
>;

fn mk_pipeline(
    device: Arc<Device>,
    render_pass: Arc<dyn RenderPassAbstract + Send + Sync>,
    vs: &vs::Shader,
    fs: &fs::Shader
) -> MyPipeline {
    Arc::new(
        GraphicsPipeline::start()
        // We need to indicate the layout of the vertices.  The type
        // `SingleBufferDefinition` actually contains a template parameter
        // corresponding to the type of each vertex. But in this code it is
        // automatically inferred.
        .vertex_input(
            OneVertexOneInstanceDefinition::<Vertex, InstanceData>::new()
        )
        // A Vulkan shader can in theory contain multiple entry points, so
        // we have to specify which one. The `main` word of
        // `main_entry_point` actually corresponds to the name of the entry
        // point.
        .vertex_shader(vs.main_entry_point(), ())
        // The content of the vertex buffer describes a list of triangles.
        .triangle_list()
        // Use a resizable viewport set to draw over the entire window
        .viewports_dynamic_scissors_irrelevant(1)
        // See `vertex_shader`.
        .fragment_shader(fs.main_entry_point(), ())
        // We have to indicate which subpass of which render pass this
        // pipeline is going to be used in. The pipeline will only be usable
        // from this particular subpass.
        .render_pass(Subpass::from(render_pass, 0).unwrap())
        // Now that our builder is filled, we call `build()` to obtain an
        // actual pipeline.
        .build(device).unwrap(),
    )
}


/// This method is called once during initialization, then again whenever the
/// window is resized
pub fn window_size_dependent_setup(
    images: &[Arc<SwapchainImage<Window>>],
    render_pass: Arc<dyn RenderPassAbstract + Send + Sync>,
    dynamic_state: &mut DynamicState,
) -> Vec<Arc<dyn FramebufferAbstract + Send + Sync>> {
    let dimensions = images[0].dimensions();

    let viewport = Viewport {
        origin: [0.0, 0.0],
        dimensions: [dimensions[0] as f32, dimensions[1] as f32],
        depth_range: 0.0..1.0,
    };
    dynamic_state.viewports = Some(vec![viewport]);

    images
        .iter()
        .map(|image| {
            Arc::new(
                Framebuffer::start(render_pass.clone())
                    .add(image.clone())
                    .unwrap()
                    .build()
                    .unwrap(),
            ) as Arc<dyn FramebufferAbstract + Send + Sync>
        })
        .collect::<Vec<_>>()
}
