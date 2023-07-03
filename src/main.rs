use image::GenericImageView;

#[derive(Clone)]
enum Obj {
	Empty,
	Player,
	Goal,
	Enemy,
	Tower,
}

#[derive(Clone)]
enum Ground {
	Grass,
	Water,
	Path(i32), // contains distance to objective
}

#[derive(Clone)]
struct Cell {
	obj: Obj,
	groud: Ground,
}

struct Grid<T> {
	w: i32,
	h: i32,
	content: Vec<T>,
}

impl<T: Clone> Grid<T> {
	fn new(w: i32, h: i32, value: T) -> Grid<T> {
		Grid {
			w,
			h,
			content: std::iter::repeat(value).take((w * h) as usize).collect(),
		}
	}
}

impl<T> Grid<T> {
	fn get(&self, coords: Coords) -> Option<&T> {
		let index = coords.y * self.w + coords.x;
		if 0 <= coords.x && coords.x < self.w && 0 <= coords.y && coords.y < self.h {
			self.content.get(index as usize)
		} else {
			None
		}
	}
	fn get_mut(&mut self, coords: Coords) -> Option<&mut T> {
		let index = coords.y * self.w + coords.x;
		if 0 <= coords.x && coords.x < self.w && 0 <= coords.y && coords.y < self.h {
			self.content.get_mut(index as usize)
		} else {
			None
		}
	}
}

struct Coords {
	x: i32,
	y: i32,
}

impl From<(i32, i32)> for Coords {
	fn from((x, y): (i32, i32)) -> Coords {
		Coords { x, y }
	}
}

#[derive(Clone)]
struct Rect {
	x: i32,
	y: i32,
	w: i32,
	h: i32,
}

impl Rect {
	fn tile(coords: Coords, tiles_side: i32) -> Rect {
		Rect {
			x: coords.x * tiles_side,
			y: coords.y * tiles_side,
			w: tiles_side,
			h: tiles_side,
		}
	}
}

fn draw_sprite(
	pixel_buffer: &mut pixels::Pixels,
	pixel_buffer_size: winit::dpi::PhysicalSize<u32>,
	dst: Rect,
	spritesheet: &image::DynamicImage,
	sprite: Rect,
) {
	// (rx, ry) is a pixel in the dst rect but with (0, 0) being the top left corner
	for ry in 0..dst.h {
		for rx in 0..dst.w {
			// (sx, sy) is the pixel to read from the spritesheet
			let sx = sprite.x as u32 + rx as u32 * sprite.w as u32 / dst.w as u32;
			let sy = sprite.y as u32 + ry as u32 * sprite.h as u32 / dst.h as u32;
			let color = spritesheet.get_pixel(sx, sy).0;
			if color[3] == 0 {
				// transparent
				continue;
			}
			// (px, py) is the pixel to write to in the pixel buffer
			// each of which is visited once
			let px = rx + dst.x;
			let py = ry + dst.y;
			if 0 <= px
				&& px < pixel_buffer_size.width as i32
				&& 0 <= py && py < pixel_buffer_size.height as i32
			{
				let pixel_index = (py * pixel_buffer_size.width as i32 + px) as usize * 4;
				pixel_buffer.frame_mut()[pixel_index..(pixel_index + 4)].copy_from_slice(&color);
			}
		}
	}
}

fn player_move(grid: &mut Grid<Cell>, (dx, dy): (i32, i32)) {
	for y in 0..grid.h {
		for x in 0..grid.w {
			if grid
				.get((x, y).into())
				.is_some_and(|cell| matches!(cell.obj, Obj::Player))
			{
				if grid
					.get((x + dx, y + dy).into())
					.is_some_and(|cell| matches!(cell.obj, Obj::Empty))
				{
					grid.get_mut((x, y).into()).unwrap().obj = Obj::Empty;
					grid.get_mut((x + dx, y + dy).into()).unwrap().obj = Obj::Player;
				}
				return;
			}
		}
	}
}

fn enemies_move(grid: &mut Grid<Cell>) {
	for y in 0..grid.h {
		for x in 0..grid.w {
			if grid
				.get((x, y).into())
				.is_some_and(|cell| matches!(cell.obj, Obj::Enemy))
			{
				let dist_to_goal = if let Ground::Path(dist) = grid.get((x, y).into()).unwrap().groud {
					dist
				} else {
					panic!("we thought we were on a path!? >.<")
				};
				for (dx, dy) in [(0, -1), (1, 0), (0, 1), (-1, 0)] {
					if grid.get((x + dx, y + dy).into()).is_some_and(|cell| {
						matches!(
							cell.groud,
							Ground::Path(neighbor_dist) if neighbor_dist < dist_to_goal
						) && matches!(cell.obj, Obj::Empty)
					}) {
						grid.get_mut((x, y).into()).unwrap().obj = Obj::Empty;
						grid.get_mut((x + dx, y + dy).into()).unwrap().obj = Obj::Enemy;
					}
				}
				return;
			}
		}
	}
}

fn main() {
	env_logger::init();
	let event_loop = winit::event_loop::EventLoop::new();

	let mut grid: Grid<Cell> = Grid::new(10, 10, Cell { obj: Obj::Empty, groud: Ground::Grass });
	grid.get_mut((4, 2).into()).unwrap().obj = Obj::Player;
	grid.get_mut((4, 4).into()).unwrap().obj = Obj::Goal;
	grid.get_mut((5, 5).into()).unwrap().obj = Obj::Tower;
	grid.get_mut((6, 9).into()).unwrap().obj = Obj::Enemy;
	grid.get_mut((0, 0).into()).unwrap().groud = Ground::Water;
	grid.get_mut((1, 0).into()).unwrap().groud = Ground::Water;
	grid.get_mut((9, 0).into()).unwrap().groud = Ground::Water;

	grid.get_mut((4, 4).into()).unwrap().groud = Ground::Path(0);
	grid.get_mut((4, 5).into()).unwrap().groud = Ground::Path(1);
	grid.get_mut((4, 6).into()).unwrap().groud = Ground::Path(2);
	grid.get_mut((4, 7).into()).unwrap().groud = Ground::Path(3);
	grid.get_mut((5, 7).into()).unwrap().groud = Ground::Path(4);
	grid.get_mut((6, 7).into()).unwrap().groud = Ground::Path(5);
	grid.get_mut((6, 8).into()).unwrap().groud = Ground::Path(6);
	grid.get_mut((6, 9).into()).unwrap().groud = Ground::Path(7);

	let cell_pixel_side = 8 * 8;

	let window = winit::window::WindowBuilder::new()
		.with_title("Prototype 7")
		.with_inner_size(winit::dpi::PhysicalSize::new(
			(grid.w * cell_pixel_side) as u32,
			(grid.h * cell_pixel_side) as u32,
		))
		.build(&event_loop)
		.unwrap();

	// Center the window
	let screen_size = window.available_monitors().next().unwrap().size();
	let window_outer_size = window.outer_size();
	window.set_outer_position(winit::dpi::PhysicalPosition::new(
		screen_size.width / 2 - window_outer_size.width / 2,
		screen_size.height / 2 - window_outer_size.height / 2,
	));

	// Set background and edge color
	let clear_color = [0, 50, 50, 255];
	let clear_color_wgpu = {
		fn conv_srgb_to_linear(x: f64) -> f64 {
			// See https://github.com/gfx-rs/wgpu/issues/2326
			// Stolen from https://github.com/three-rs/three/blob/07e47da5e0673aa9a16526719e16debd59040eec/src/color.rs#L42
			// (licensed MIT, not a substancial portion so not concerned by license obligations)
			// Basically the brightness is adjusted somewhere by wgpu or something due to sRGB stuff,
			// color is hard.
			if x > 0.04045 {
				((x + 0.055) / 1.055).powf(2.4)
			} else {
				x / 12.92
			}
		}
		pixels::wgpu::Color {
			r: conv_srgb_to_linear(clear_color[0] as f64 / 255.0),
			g: conv_srgb_to_linear(clear_color[1] as f64 / 255.0),
			b: conv_srgb_to_linear(clear_color[2] as f64 / 255.0),
			a: conv_srgb_to_linear(clear_color[3] as f64 / 255.0),
		}
	};

	let mut pixel_buffer_size = window.inner_size();
	let mut pixel_buffer = {
		let size = pixel_buffer_size;
		let surface_texture = pixels::SurfaceTexture::new(size.width, size.height, &window);
		pixels::PixelsBuilder::new(size.width, size.height, surface_texture)
			.clear_color(clear_color_wgpu)
			.build()
			.unwrap()
	};

	let spritesheet = image::load_from_memory(include_bytes!("../assets/spritesheet.png")).unwrap();

	use winit::event::*;
	event_loop.run(move |event, _, control_flow| match event {
		Event::WindowEvent { ref event, window_id } if window_id == window.id() => match event {
			WindowEvent::CloseRequested
			| WindowEvent::KeyboardInput {
				input:
					KeyboardInput {
						state: ElementState::Pressed,
						virtual_keycode: Some(VirtualKeyCode::Escape),
						..
					},
				..
			} => {
				*control_flow = winit::event_loop::ControlFlow::Exit;
			},

			WindowEvent::KeyboardInput {
				input: KeyboardInput { state: ElementState::Pressed, virtual_keycode: Some(key), .. },
				..
			} if matches!(
				key,
				VirtualKeyCode::Up
					| VirtualKeyCode::Right
					| VirtualKeyCode::Down
					| VirtualKeyCode::Left
			) =>
			{
				let dxdy = match key {
					VirtualKeyCode::Up => (0, -1),
					VirtualKeyCode::Right => (1, 0),
					VirtualKeyCode::Down => (0, 1),
					VirtualKeyCode::Left => (-1, 0),
					_ => unreachable!(),
				};
				player_move(&mut grid, dxdy);
				enemies_move(&mut grid);
			},

			_ => {},
		},

		Event::MainEventsCleared => {
			std::thread::sleep(std::time::Duration::from_millis(7));
			pixel_buffer
				.frame_mut()
				.chunks_exact_mut(4)
				.for_each(|pixel| pixel.copy_from_slice(&clear_color));

			for y in 0..grid.h {
				for x in 0..grid.w {
					let dst = Rect::tile((x, y).into(), cell_pixel_side);
					let sprite = match grid.get((x, y).into()).unwrap().groud {
						Ground::Grass => (5, 0),
						Ground::Water => (6, 0),
						Ground::Path(_) => (7, 0),
					};
					let sprite_rect = Rect::tile(sprite.into(), 8);
					draw_sprite(
						&mut pixel_buffer,
						pixel_buffer_size,
						dst.clone(),
						&spritesheet,
						sprite_rect,
					);
					let sprite = match grid.get((x, y).into()).unwrap().obj {
						Obj::Empty => None,
						Obj::Player => Some((0, 0)),
						Obj::Goal => Some((1, 0)),
						Obj::Enemy => Some((2, 0)),
						Obj::Tower => Some((3, 0)),
					};
					if let Some(sprite) = sprite {
						let sprite_rect = Rect::tile(sprite.into(), 8);
						draw_sprite(
							&mut pixel_buffer,
							pixel_buffer_size,
							dst,
							&spritesheet,
							sprite_rect,
						);
					}
				}
			}

			window.request_redraw();
		},

		Event::RedrawRequested(_) => {
			pixel_buffer.render().unwrap();
		},

		_ => {},
	});
}
