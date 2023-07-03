use std::fs;

use image::GenericImageView;

#[derive(Clone)]
enum Obj {
	Empty,
	Player,
	Goal,
	Enemy { hp: u32, hp_max: u32 },
	Tower,
	Rock,
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

#[derive(Clone)]
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

#[derive(Clone, Copy)]
struct Coords {
	x: i32,
	y: i32,
}

impl From<(i32, i32)> for Coords {
	fn from((x, y): (i32, i32)) -> Coords {
		Coords { x, y }
	}
}

impl std::ops::Add for Coords {
	type Output = Coords;
	fn add(self, rhs: Self) -> Self::Output {
		(self.x + rhs.x, self.y + rhs.y).into()
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

fn draw_rect(
	pixel_buffer: &mut pixels::Pixels,
	pixel_buffer_size: winit::dpi::PhysicalSize<u32>,
	dst: Rect,
	color: [u8; 4],
) {
	for y in dst.y..(dst.y + dst.h) {
		for x in dst.x..(dst.x + dst.w) {
			if 0 <= x
				&& x < pixel_buffer_size.width as i32
				&& 0 <= y && y < pixel_buffer_size.height as i32
			{
				let pixel_index = (y * pixel_buffer_size.width as i32 + x) as usize * 4;
				pixel_buffer.frame_mut()[pixel_index..(pixel_index + 4)].copy_from_slice(&color);
			}
		}
	}
}

enum PlayerAction {
	Move,
	PlaceTower,
}

fn player_move(grid: &mut Grid<Cell>, (dx, dy): (i32, i32), action: PlayerAction) {
	for y in 0..grid.h {
		for x in 0..grid.w {
			if grid
				.get((x, y).into())
				.is_some_and(|cell| matches!(cell.obj, Obj::Player))
			{
				if grid.get((x + dx, y + dy).into()).is_some_and(|cell| {
					matches!(cell.obj, Obj::Empty) && !matches!(cell.groud, Ground::Water)
				}) {
					match action {
						PlayerAction::Move => {
							grid.get_mut((x, y).into()).unwrap().obj = Obj::Empty;
							grid.get_mut((x + dx, y + dy).into()).unwrap().obj = Obj::Player;
						},
						PlayerAction::PlaceTower => {
							grid.get_mut((x + dx, y + dy).into()).unwrap().obj = Obj::Tower;
						},
					}
				}
				return;
			}
		}
	}
}

fn enemies_move(grid: &mut Grid<Cell>) {
	let mut new_grid = grid.clone();

	for dist in 0..(grid.h * grid.w) {
		let mut found_one = false;
		for y in 0..grid.h {
			for x in 0..grid.w {
				let dist_to_goal = if let Ground::Path(dist) = grid.get((x, y).into()).unwrap().groud {
					found_one = true;
					Some(dist)
				} else {
					None
				};
				if grid
					.get((x, y).into())
					.is_some_and(|cell| matches!(cell.obj, Obj::Enemy { .. }))
				{
					let dist_to_goal = dist_to_goal.expect("we thought we were on a path!? >.<");
					if dist_to_goal != dist {
						continue;
					}
					for (dx, dy) in [(0, -1), (1, 0), (0, 1), (-1, 0)] {
						if new_grid.get((x + dx, y + dy).into()).is_some_and(|cell| {
							matches!(
								cell.groud,
								Ground::Path(neighbor_dist) if neighbor_dist < dist_to_goal
							) && matches!(cell.obj, Obj::Empty | Obj::Goal)
						}) {
							new_grid.get_mut((x + dx, y + dy).into()).unwrap().obj = std::mem::replace(
								&mut new_grid.get_mut((x, y).into()).unwrap().obj,
								Obj::Empty,
							);
						}
					}
				}
			}
		}
		// Didn't find any tile with distance d, stops iterating
		if !found_one {
			break;
		}
	}
	*grid = new_grid;
}

fn towers_move(grid: &mut Grid<Cell>) {
	for y in 0..grid.h {
		for x in 0..grid.w {
			if grid
				.get((x, y).into())
				.is_some_and(|cell| matches!(cell.obj, Obj::Tower))
			{
				for (dx, dy) in [(0, -1), (1, 0), (0, 1), (-1, 0)] {
					let mut sx = x;
					let mut sy = y;
					loop {
						sx += dx;
						sy += dy;
						if grid.get((sx, sy).into()).is_none()
							|| grid.get((sx, sy).into()).is_some_and(|cell| {
								matches!(cell.obj, Obj::Player | Obj::Goal | Obj::Tower | Obj::Rock)
							}) {
							break;
						}
						if grid
							.get((sx, sy).into())
							.is_some_and(|cell| matches!(cell.obj, Obj::Enemy { .. }))
						{
							let is_dead = if let Obj::Enemy { hp, .. } =
								&mut grid.get_mut((sx, sy).into()).unwrap().obj
							{
								*hp -= 1;
								*hp == 0
							} else {
								unreachable!()
							};
							if is_dead {
								grid.get_mut((sx, sy).into()).unwrap().obj = Obj::Empty;
							}
						}
					}
				}
			}
		}
	}
}

fn load_level(level_file: &str) -> std::io::Result<Grid<Cell>> {
	let level_data = fs::read_to_string(level_file)?;
	let grid_h = level_data.split("\n").filter(|x| !x.is_empty()).count();
	let grid_w = level_data
		.split("\n")
		.filter(|x| !x.is_empty())
		.next()
		.unwrap()
		.split(char::is_whitespace)
		.count();
	let mut grid: Grid<Cell> = Grid::new(
		grid_w as i32,
		grid_h as i32,
		Cell { obj: Obj::Empty, groud: Ground::Grass },
	);
	let mut cells_info = level_data.split(char::is_whitespace);
	for y in 0..grid.h {
		for x in 0..grid.w {
			let hh = cells_info.next().unwrap();
			let mut cell = grid.get_mut((x, y).into()).unwrap();
			cell.groud = match hh.chars().nth(0) {
				Some('O') => Ground::Grass,
				Some('x') => Ground::Water,
				Some('|') => Ground::Path(-1),
				_ => panic!("Ground format incorrect at {x}, {y}"),
			};
			cell.obj = match hh.chars().nth(1) {
				Some('-') => Obj::Empty,
				Some('p') => Obj::Player,
				Some('t') => Obj::Tower,
				Some('e') => Obj::Enemy { hp: 3, hp_max: 3 },
				Some('g') => Obj::Goal,
				Some('r') => Obj::Rock,
				_ => panic!("Object format incorrect at {x}, {y}"),
			};
		}
	}
	Ok(grid)
}

fn compute_distance(grid: &mut Grid<Cell>) {
	let goal = 'goal_find: {
		for x in 0..grid.w {
			for y in 0..grid.h {
				if matches!(grid.get((x, y).into()).unwrap().obj, Obj::Goal) {
					break 'goal_find (x, y);
				}
			}
		}
		println!("Didn't find a goal on the level");
		return;
	};
	fn update_dist(grid: &mut Grid<Cell>, start: Coords, depth: i32) {
		let dirs = [(-1, 0), (0, 1), (1, 0), (0, -1)];
		grid.get_mut(start).unwrap().groud = Ground::Path(depth);
		for d in dirs {
			let dst = start + d.into();
			if grid.get(dst).is_none() {
				continue;
			}
			if let Ground::Path(dist) = grid.get(dst).unwrap().groud {
				if dist == -1 || dist > depth {
					update_dist(grid, dst, depth + 1);
				}
			}
		}
	}
	update_dist(grid, goal.into(), 0);
}

fn _print_dist(grid: &Grid<Cell>) {
	for y in 0..grid.h {
		for x in 0..grid.w {
			match grid.get((x, y).into()).unwrap().groud {
				Ground::Path(d) => print!("{d:2} "),
				_ => print!(" - "),
			}
		}
		println!();
	}
	println!();
}

fn is_game_joever(grid: &Grid<Cell>) -> bool {
	for x in 0..grid.w {
		for y in 0..grid.h {
			if matches!(grid.get((x, y).into()).unwrap().obj, Obj::Goal) {
				return false;
			}
		}
	}
	true
}
fn main() {
	env_logger::init();
	let event_loop = winit::event_loop::EventLoop::new();

	let level_file = "./levels/test";
	let mut grid = match load_level(level_file) {
		Ok(grid) => grid,
		Err(jaaj) => match jaaj.kind() {
			std::io::ErrorKind::NotFound => panic!("File not found at {level_file}"),
			_ => panic!("Error while reading level file"),
		},
	};
	// _print_dist(&grid);
	compute_distance(&mut grid);
	_print_dist(&grid);

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

	let pixel_buffer_size = window.inner_size();
	let mut pixel_buffer = {
		let size = pixel_buffer_size;
		let surface_texture = pixels::SurfaceTexture::new(size.width, size.height, &window);
		pixels::PixelsBuilder::new(size.width, size.height, surface_texture)
			.clear_color(clear_color_wgpu)
			.build()
			.unwrap()
	};

	let spritesheet = image::load_from_memory(include_bytes!("../assets/spritesheet.png")).unwrap();

	let mut is_ctrl_pressed = false;
	let mut its_joever = false;
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

			WindowEvent::ModifiersChanged(modifiers) => {
				is_ctrl_pressed = (*modifiers & ModifiersState::CTRL) == ModifiersState::CTRL;
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
				let action = if is_ctrl_pressed {
					PlayerAction::PlaceTower
				} else {
					PlayerAction::Move
				};
				player_move(&mut grid, dxdy, action);
				if !its_joever {
					enemies_move(&mut grid);
					its_joever = is_game_joever(&grid);
					towers_move(&mut grid);
				}
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
						Obj::Enemy { .. } => Some((2, 0)),
						Obj::Tower => Some((3, 0)),
						Obj::Rock => Some((8, 0)),
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
					if let Obj::Enemy { hp, hp_max } = grid.get((x, y).into()).unwrap().obj {
						let mut dst = Rect::tile((x, y).into(), cell_pixel_side);
						dst.y += cell_pixel_side / 8;
						dst.h = cell_pixel_side / 8;
						dst.x += cell_pixel_side / 8;
						dst.w = cell_pixel_side * 6 / 8;
						draw_rect(
							&mut pixel_buffer,
							pixel_buffer_size,
							dst.clone(),
							[255, 0, 0, 255],
						);
						dst.w = (cell_pixel_side * 6 / 8) * hp as i32 / hp_max as i32;
						draw_rect(&mut pixel_buffer, pixel_buffer_size, dst, [0, 255, 0, 255]);
					}
				}
			}
			if its_joever {
				draw_sprite(
					&mut pixel_buffer,
					pixel_buffer_size,
					Rect { x: 1 * 8 * 8, y: 2 * 8 * 8, w: 8 * 7 * 8, h: 8 * 8 },
					&spritesheet,
					Rect { x: 0, y: 8, w: 8 * 7, h: 8 },
				);
			}
			window.request_redraw();
		},

		Event::RedrawRequested(_) => {
			pixel_buffer.render().unwrap();
		},

		_ => {},
	});
}
