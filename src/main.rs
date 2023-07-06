use core::panic;
use image::GenericImageView;
use std::fs;

#[derive(Clone)]
enum Obj {
	Empty,
	Player,
	Goal,
	Enemy { hp: u32, hp_max: u32 },
	Tower,
	Rock,
	Tree,
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

#[derive(Clone, Copy)]
struct Dimensions {
	w: i32,
	h: i32,
}

impl From<winit::dpi::PhysicalSize<u32>> for Dimensions {
	fn from(size: winit::dpi::PhysicalSize<u32>) -> Dimensions {
		Dimensions { w: size.width as i32, h: size.height as i32 }
	}
}

impl Dimensions {
	fn square(side: i32) -> Dimensions {
		Dimensions { w: side, h: side }
	}

	fn area(self) -> i32 {
		self.w * self.h
	}

	fn contains(self, coords: Coords) -> bool {
		0 <= coords.x && coords.x < self.w && 0 <= coords.y && coords.y < self.h
	}

	fn index_of_coords(self, coords: Coords) -> Option<usize> {
		if self.contains(coords) {
			Some((coords.y * self.w + coords.x) as usize)
		} else {
			None
		}
	}
}

impl Dimensions {
	fn iter(self) -> IterCoordsRect {
		IterCoordsRect::with_rect(Rect { top_left: (0, 0).into(), dims: self })
	}
}

struct IterCoordsRect {
	current: Coords,
	rect: Rect,
}
impl IterCoordsRect {
	fn with_rect(rect: Rect) -> IterCoordsRect {
		IterCoordsRect { current: rect.top_left, rect }
	}
}
impl Iterator for IterCoordsRect {
	type Item = Coords;
	fn next(&mut self) -> Option<Coords> {
		let coords = self.current;
		self.current.x += 1;
		if !self.rect.contains(self.current) {
			self.current.x = self.rect.left();
			self.current.y += 1;
		}
		if self.rect.contains(coords) {
			Some(coords)
		} else {
			None
		}
	}
}

#[derive(Clone)]
struct Grid<T> {
	dims: Dimensions,
	content: Vec<T>,
}

impl<T: Clone> Grid<T> {
	fn new(dims: Dimensions, value: T) -> Grid<T> {
		Grid {
			dims,
			content: std::iter::repeat(value)
				.take(dims.area() as usize)
				.collect(),
		}
	}
}

impl<T> Grid<T> {
	fn get(&self, coords: Coords) -> Option<&T> {
		if let Some(index) = self.dims.index_of_coords(coords) {
			self.content.get(index as usize)
		} else {
			None
		}
	}
	fn get_mut(&mut self, coords: Coords) -> Option<&mut T> {
		if let Some(index) = self.dims.index_of_coords(coords) {
			self.content.get_mut(index as usize)
		} else {
			None
		}
	}
}

struct LevelData {
	init_grid: Grid<Cell>,
	max_towers: Option<u32>,
}

impl LevelData {
	fn new(grid: Grid<Cell>) -> LevelData {
		LevelData { init_grid: grid, max_towers: None }
	}
}
struct LevelState {
	grid: Grid<Cell>,
	towers: u32,
	game_joever: bool,
}

#[derive(Clone, Copy)]
struct Coords {
	x: i32,
	y: i32,
}

#[derive(Clone, Copy)]
struct DxDy {
	dx: i32,
	dy: i32,
}

impl From<(i32, i32)> for Coords {
	fn from((x, y): (i32, i32)) -> Coords {
		Coords { x, y }
	}
}
impl From<(i32, i32)> for DxDy {
	fn from((dx, dy): (i32, i32)) -> DxDy {
		DxDy { dx, dy }
	}
}
impl From<Coords> for DxDy {
	fn from(coords: Coords) -> DxDy {
		DxDy { dx: coords.x, dy: coords.y }
	}
}

impl std::ops::Add<DxDy> for Coords {
	type Output = Coords;
	fn add(self, rhs: DxDy) -> Coords {
		(self.x + rhs.dx, self.y + rhs.dy).into()
	}
}
impl std::ops::AddAssign<DxDy> for Coords {
	fn add_assign(&mut self, rhs: DxDy) {
		*self = *self + rhs;
	}
}

impl DxDy {
	fn the_4_directions() -> impl Iterator<Item = DxDy> {
		[(0, -1), (1, 0), (0, 1), (-1, 0)]
			.into_iter()
			.map(DxDy::from)
	}
}

impl std::fmt::Display for Coords {
	fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
		write!(f, "{}, {}", self.x, self.y)
	}
}

#[derive(Clone, Copy)]
struct Rect {
	top_left: Coords,
	dims: Dimensions,
}

impl Rect {
	fn tile(coords: Coords, tiles_side: i32) -> Rect {
		Rect {
			top_left: Coords { x: coords.x * tiles_side, y: coords.y * tiles_side },
			dims: Dimensions::square(tiles_side),
		}
	}

	fn top(self) -> i32 {
		self.top_left.y
	}
	fn left(self) -> i32 {
		self.top_left.x
	}
	fn bottom_excluded(self) -> i32 {
		self.top_left.y + self.dims.h
	}
	fn right_excluded(self) -> i32 {
		self.top_left.x + self.dims.w
	}

	fn contains(self, coords: Coords) -> bool {
		self.left() <= coords.x
			&& coords.x < self.right_excluded()
			&& self.top() <= coords.y
			&& coords.y < self.bottom_excluded()
	}

	fn iter(self) -> IterCoordsRect {
		IterCoordsRect::with_rect(self)
	}
}

/// Draw a sprite form the given spritesheet to the given pixel buffer.
/// `dst` is the rectangle location of the pixel buffer to draw to,
/// `sprite` is the rectangle location of the spritesheet to copy from.
fn draw_sprite(
	pixel_buffer: &mut pixels::Pixels,
	pixel_buffer_dims: Dimensions,
	dst: Rect,
	spritesheet: &image::DynamicImage,
	sprite: Rect,
) {
	// `coords_dst_dims` is a pixel in the dst rect but with (0, 0) being the top left corner.
	for coords_dst_dims in dst.dims.iter() {
		// `(sx, sy)` is the pixel to read from the spritesheet.
		let sx = (sprite.top_left.x + coords_dst_dims.x * sprite.dims.w / dst.dims.w) as u32;
		let sy = (sprite.top_left.y + coords_dst_dims.y * sprite.dims.h / dst.dims.h) as u32;
		let color = spritesheet.get_pixel(sx, sy).0;
		if color[3] == 0 {
			// Skip transparent pixels.
			continue;
		}
		// `coords_pixel_buffer` is the pixel to write to in the pixel buffer,
		// each of which is visited once.
		let coords_pixel_buffer = coords_dst_dims + dst.top_left.into();
		if let Some(pixel_index) = pixel_buffer_dims.index_of_coords(coords_pixel_buffer) {
			let pixel_byte_index = pixel_index * 4;
			let pixel_bytes = pixel_byte_index..(pixel_byte_index + 4);
			pixel_buffer.frame_mut()[pixel_bytes].copy_from_slice(&color);
		}
	}
}

fn draw_rect(
	pixel_buffer: &mut pixels::Pixels,
	pixel_buffer_dims: Dimensions,
	dst: Rect,
	color: [u8; 4],
) {
	for coords in dst.iter() {
		if let Some(pixel_index) = pixel_buffer_dims.index_of_coords(coords) {
			let pixel_byte_index = pixel_index * 4;
			let pixel_bytes = pixel_byte_index..(pixel_byte_index + 4);
			pixel_buffer.frame_mut()[pixel_bytes].copy_from_slice(&color);
		}
	}
}

fn try_push(grid: &mut Grid<Cell>, coords: Coords, dd: DxDy) {
	if grid.get(coords).is_none() {
		return;
	}
	let obj = grid.get(coords).unwrap().obj.clone();
	if matches!(obj, Obj::Rock | Obj::Tower) {
		let dst_coords = coords + dd;
		try_push(grid, dst_coords, dd);
		if grid
			.get(dst_coords)
			.is_some_and(|cell| matches!(cell.obj, Obj::Empty))
		{
			if !matches!(grid.get(dst_coords).unwrap().groud, Ground::Water) {
				grid.get_mut(dst_coords).unwrap().obj = obj;
			}
			grid.get_mut(coords).unwrap().obj = Obj::Empty;
		}
	}
}

enum PlayerAction {
	Move,
	PlaceTower,
	SkipTurn,
}

fn player_move(grid: &mut Grid<Cell>, dd: DxDy, action: PlayerAction) {
	for coords in grid.dims.iter() {
		if grid
			.get(coords)
			.is_some_and(|cell| matches!(cell.obj, Obj::Player))
		{
			let dst_coords = coords + dd;
			match action {
				PlayerAction::Move => {
					if grid
						.get(dst_coords)
						.is_some_and(|cell| !matches!(cell.groud, Ground::Water))
					{
						if !matches!(grid.get(dst_coords).unwrap().obj, Obj::Empty) {
							try_push(grid, dst_coords, dd);
						}
						if matches!(grid.get(dst_coords).unwrap().obj, Obj::Empty) {
							grid.get_mut(coords).unwrap().obj = Obj::Empty;
							grid.get_mut(dst_coords).unwrap().obj = Obj::Player;
						}
					}
				},
				PlayerAction::PlaceTower => {
					if grid.get(dst_coords).is_some_and(|cell| {
						matches!(cell.obj, Obj::Empty) && !matches!(cell.groud, Ground::Water)
					}) {
						grid.get_mut(dst_coords).unwrap().obj = Obj::Tower;
					}
				},
				PlayerAction::SkipTurn => {},
			}
			return;
		}
	}
}

fn enemies_move(grid: &mut Grid<Cell>) {
	let mut new_grid = grid.clone();
	// In order for enemies to try to move in an efficient way, enemies closer to the goal
	// (in distance on the path) move in priority (so that two adjacent enemies one before the
	// other may both move during one turn, instead of the enemy behind trying to move first but
	// being blocked by the other enemy just in front of it).
	// One way to do that is to iterate in increasing order over all the possible distances
	// that enemies can be to the goal, and for each possible distance we move all the enemies
	// that are at that distance. This is what we do here.
	for dist in 0..grid.dims.area() {
		let mut found_one = false;
		for coords in grid.dims.iter() {
			let dist_to_goal = if let Ground::Path(dist) = grid.get(coords).unwrap().groud {
				found_one = true;
				Some(dist)
			} else {
				None
			};
			if grid
				.get(coords)
				.is_some_and(|cell| matches!(cell.obj, Obj::Enemy { .. }))
			{
				let dist_to_goal = dist_to_goal.expect("we thought we were on a path!? >.<");
				if dist_to_goal != dist {
					continue;
				}
				// We may move. We try to find an adjacent path tile that will get us loser
				// to the goal (so its distance to the goal should be smaller that our
				// current distance) (these distances are stored in the path tiles).
				for dd in DxDy::the_4_directions() {
					let dst_coords = coords + dd;
					if new_grid.get(dst_coords).is_some_and(|cell| {
						matches!(
							cell.groud,
							Ground::Path(neighbor_dist) if neighbor_dist < dist_to_goal
						) && matches!(cell.obj, Obj::Empty | Obj::Goal | Obj::Tower | Obj::Rock)
					}) {
						if matches!(new_grid.get_mut(dst_coords).unwrap().obj, Obj::Rock) {
							try_push(&mut new_grid, dst_coords, dd);
						}
						if !matches!(new_grid.get_mut(dst_coords).unwrap().obj, Obj::Rock) {
							new_grid.get_mut(dst_coords).unwrap().obj =
								std::mem::replace(&mut new_grid.get_mut(coords).unwrap().obj, Obj::Empty);
						}
						break;
					}
				}
			}
		}
		// Didn't find any tile with distance `dist` (so there wont be at any greater distance either),
		// thus we stop iterating.
		if !found_one {
			break;
		}
	}
	*grid = new_grid;
}

fn towers_move(grid: &mut Grid<Cell>) {
	for coords in grid.dims.iter() {
		if grid
			.get(coords)
			.is_some_and(|cell| matches!(cell.obj, Obj::Tower))
		{
			for dd in DxDy::the_4_directions() {
				let mut coords_possible_target = coords;
				loop {
					coords_possible_target += dd;
					if grid
						.get(coords_possible_target)
						.is_some_and(|cell| matches!(cell.obj, Obj::Enemy { .. }))
					{
						// An enemy is in a straight line of sight, we shoot it.
						let is_dead = if let Obj::Enemy { hp, .. } =
							&mut grid.get_mut(coords_possible_target).unwrap().obj
						{
							*hp -= 1;
							*hp == 0
						} else {
							unreachable!()
						};
						if is_dead {
							grid.get_mut(coords_possible_target).unwrap().obj = Obj::Empty;
						}
						break;
					}
					if grid.get(coords_possible_target).is_none()
						|| grid
							.get(coords_possible_target)
							.is_some_and(|cell| !matches!(cell.obj, Obj::Empty))
					{
						// View is blocked by some non-targettable object.
						break;
					}
				}
			}
		}
	}
}

fn load_level(level_file: &str) -> std::io::Result<LevelData> {
	let level_raw_data = fs::read_to_string(level_file)?;
	let filt = |x: &&str| !x.is_empty() && !x.starts_with('@');
	let grid_h = level_raw_data.split('\n').filter(filt).count();
	let grid_w = level_raw_data
		.split('\n')
		.find(filt)
		.unwrap()
		.split(char::is_whitespace)
		.count();
	let dims = Dimensions { w: grid_w as i32, h: grid_h as i32 };
	let mut grid: Grid<Cell> = Grid::new(dims, Cell { obj: Obj::Empty, groud: Ground::Grass });
	let mut cells_info = level_raw_data.split(char::is_whitespace);
	for coords in grid.dims.iter() {
		let current_tile = cells_info.next().unwrap();
		let mut cell = grid.get_mut(coords).unwrap();
		cell.groud = match current_tile.chars().next() {
			Some('O') => Ground::Grass,
			Some('x') => Ground::Water,
			Some('|') => Ground::Path(-1),
			_ => panic!("Ground format incorrect at {coords}"),
		};
		cell.obj = match current_tile.chars().nth(1) {
			Some('-') => Obj::Empty,
			Some('p') => Obj::Player,
			Some('t') => Obj::Tower,
			Some('e') => Obj::Enemy { hp: 3, hp_max: 3 },
			Some('g') => Obj::Goal,
			Some('r') => Obj::Rock,
			Some('T') => Obj::Tree,
			_ => panic!("Object format incorrect at {coords}"),
		};
	}
	let mut level_data = LevelData::new(grid);
	let meta_data = level_raw_data
		.split('\n')
		.filter_map(|x| x.strip_prefix('@'));
	for line in meta_data {
		let mut line = line.split(char::is_whitespace);
		match line.next().unwrap() {
			"maxtower" => level_data.max_towers = Some(line.next().unwrap().parse().unwrap()),
			_ => panic!("Jaaj"),
		}
	}
	println!("maxtower: {x:?}", x = level_data.max_towers);
	Ok(level_data)
}

fn compute_distance(grid: &mut Grid<Cell>) {
	let goal = 'goal_find: {
		for coords in grid.dims.iter() {
			if matches!(grid.get(coords).unwrap().obj, Obj::Goal) {
				break 'goal_find coords;
			}
		}
		println!("Didn't find a goal on the level");
		return;
	};
	fn update_dist(grid: &mut Grid<Cell>, start: Coords, depth: i32) {
		grid.get_mut(start).unwrap().groud = Ground::Path(depth);
		for dd in DxDy::the_4_directions() {
			let dst = start + dd;
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
	update_dist(grid, goal, 0);
}

fn _print_dist(grid: &Grid<Cell>) {
	for y in 0..grid.dims.h {
		for x in 0..grid.dims.w {
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
	for coords in grid.dims.iter() {
		if matches!(grid.get(coords).unwrap().obj, Obj::Goal) {
			return false;
		}
	}
	true
}
fn main() {
	env_logger::init();
	let event_loop = winit::event_loop::EventLoop::new();

	let level_file = if let Some(file_path) = std::env::args().nth(1) {
		file_path
	} else {
		String::from("./levels/test")
	};
	let level_data = match load_level(level_file.as_str()) {
		Ok(grid) => grid,
		Err(jaaj) => match jaaj.kind() {
			std::io::ErrorKind::NotFound => panic!("File not found at {level_file}"),
			_ => panic!("Error while reading level file"),
		},
	};
	let mut grid = level_data.init_grid;
	// _print_dist(&grid);
	compute_distance(&mut grid);
	_print_dist(&grid);

	let cell_pixel_side = 8 * 8;

	let window = winit::window::WindowBuilder::new()
		.with_title("Prototype 7")
		.with_inner_size(winit::dpi::PhysicalSize::new(
			(grid.dims.w * cell_pixel_side) as u32,
			(grid.dims.h * cell_pixel_side) as u32,
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

	let pixel_buffer_dims: Dimensions = window.inner_size().into();
	let mut pixel_buffer = {
		let dims = pixel_buffer_dims;
		let surface_texture = pixels::SurfaceTexture::new(dims.w as u32, dims.h as u32, &window);
		pixels::PixelsBuilder::new(dims.w as u32, dims.h as u32, surface_texture)
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
					| VirtualKeyCode::Space
			) =>
			{
				let mut action = if is_ctrl_pressed {
					PlayerAction::PlaceTower
				} else {
					PlayerAction::Move
				};
				let dxdy = match key {
					VirtualKeyCode::Up => (0, -1),
					VirtualKeyCode::Right => (1, 0),
					VirtualKeyCode::Down => (0, 1),
					VirtualKeyCode::Left => (-1, 0),
					VirtualKeyCode::Space => {
						action = PlayerAction::SkipTurn;
						(0, 0)
					},
					_ => unreachable!(),
				}
				.into();
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

			for coords in grid.dims.iter() {
				let dst = Rect::tile(coords, cell_pixel_side);
				let sprite = match grid.get(coords).unwrap().groud {
					Ground::Grass => (5, 0),
					Ground::Water => (6, 0),
					Ground::Path(_) => (7, 0),
				};
				let sprite_rect = Rect::tile(sprite.into(), 8);
				draw_sprite(
					&mut pixel_buffer,
					pixel_buffer_dims,
					dst.clone(),
					&spritesheet,
					sprite_rect,
				);
				let sprite = match grid.get(coords).unwrap().obj {
					Obj::Empty => None,
					Obj::Player => Some((0, 0)),
					Obj::Goal => Some((1, 0)),
					Obj::Enemy { .. } => Some((2, 0)),
					Obj::Tower => Some((3, 0)),
					Obj::Rock => Some((8, 0)),
					Obj::Tree => Some((9, 0)),
				};
				if let Some(sprite) = sprite {
					let sprite_rect = Rect::tile(sprite.into(), 8);
					draw_sprite(
						&mut pixel_buffer,
						pixel_buffer_dims,
						dst,
						&spritesheet,
						sprite_rect,
					);
				}
				if let Obj::Enemy { hp, hp_max } = grid.get(coords).unwrap().obj {
					// Draw a life bar
					let mut dst = Rect::tile(coords, cell_pixel_side);
					dst.top_left.y += cell_pixel_side / 8;
					dst.dims.h = cell_pixel_side / 8;
					dst.top_left.x += cell_pixel_side / 8;
					dst.dims.w = cell_pixel_side * 6 / 8;
					draw_rect(
						&mut pixel_buffer,
						pixel_buffer_dims,
						dst.clone(),
						[255, 0, 0, 255],
					);
					dst.dims.w = (cell_pixel_side * 6 / 8) * hp as i32 / hp_max as i32;
					draw_rect(&mut pixel_buffer, pixel_buffer_dims, dst, [0, 255, 0, 255]);
				}
			}
			if its_joever {
				let jover_sprite = Rect {
					top_left: Coords { x: 0, y: 8 },
					dims: Dimensions { w: 8 * 7, h: 8 },
				};
				let dst_dims = Dimensions { w: 8 * 7 * 8, h: 8 * 8 };
				let centered_dst = Rect {
					top_left: Coords {
						x: pixel_buffer_dims.w / 2 - dst_dims.w / 2,
						y: pixel_buffer_dims.h / 2 - dst_dims.h / 2,
					},
					dims: dst_dims,
				};
				draw_sprite(
					&mut pixel_buffer,
					pixel_buffer_dims,
					centered_dst,
					&spritesheet,
					jover_sprite,
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
