mod coords;

use coords::*;

use core::panic;
use image::GenericImageView;
use std::collections::HashMap;
use std::fs;

#[derive(Clone)]
enum Obj {
	Empty,
	Player { stunned: bool },
	Goal,
	Enemy { variant: Enemy, hp: u32 },
	Tower { variant: Tower, stunned: bool },
	Bomb { countdown: u32 },
	Flower { variant: Flower },
	Rock,
	Tree,
}

impl Obj {
	fn new_enemy(variant: Enemy) -> Obj {
		let hp = variant.hp_max();
		Obj::Enemy { variant, hp }
	}
}

#[derive(Clone)]
enum Ground {
	Grass,
	Water,
	/// Contains distance (along the path) to the goal.
	Path(i32),
}

#[derive(Clone)]
enum Direction {
	North,
	South,
	East,
	West,
}

#[derive(Clone)]
enum Protection {
	Sides,
	FullStack,
	UniqueFront,
	UniqueBack,
	ThreeFront,
	ThreeBack,
}

#[derive(Clone)]
enum Enemy {
	Basic,
	Tank,
	Protected { direction: Direction, protection: Protection },
	Speeeeed,
	Stuner,
	Eater,
}

impl Enemy {
	fn hp_max(&self) -> u32 {
		match self {
			Enemy::Basic => 5,
			Enemy::Tank => 9,
			Enemy::Protected { .. } => 4,
			Enemy::Speeeeed => 3,
			Enemy::Stuner => 4,
			Enemy::Eater => 4,
		}
	}
}

#[derive(Clone, PartialEq, Eq)]
enum Tower {
	Basic,
	Piercing,
	TotalEnergy,
	Bomber,
	Pusher,
}

#[derive(Clone)]
enum Flower {
	BlueFlower,
	TheOther,
}

#[derive(Clone)]
struct Cell {
	obj: Obj,
	groud: Ground,
	rocky_path: bool,
}

struct LevelData {
	init_grid: Grid<Cell>,
	max_towers: Option<u32>,
	init_events: Vec<GameEvent>,
}

impl LevelData {
	fn new(grid: Grid<Cell>) -> LevelData {
		LevelData { init_grid: grid, max_towers: None, init_events: vec![] }
	}
}

struct LevelState {
	grid: Grid<Cell>,
	remaining_towers: Option<u32>,
	turn: u32,
	events: Vec<GameEvent>,
	game_joever: bool,
}

impl LevelState {
	fn new(level_data: &LevelData) -> LevelState {
		let mut grid = level_data.init_grid.clone();
		compute_distance(&mut grid);
		LevelState {
			grid,
			remaining_towers: level_data.max_towers,
			turn: 0,
			events: level_data.init_events.clone(),
			game_joever: false,
		}
	}
}

#[derive(Clone)]
enum GameEventType {
	EnemySpawn(Coords),
}

#[derive(Clone)]
struct GameEvent {
	turn: u32,
	event_type: GameEventType,
}

impl GameEvent {
	fn new(turn: u32, event_type: GameEventType) -> GameEvent {
		GameEvent { turn, event_type }
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
	if matches!(obj, Obj::Rock | Obj::Tower { .. }) {
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

#[derive(PartialEq, Eq)]
enum PlayerAction {
	Move,
	PlaceTower { variant: Tower },
	SkipTurn,
}

fn player_move(level: &mut LevelState, dd: DxDy, action: PlayerAction) {
	for coords in level.grid.dims.iter() {
		if level
			.grid
			.get(coords)
			.is_some_and(|cell| matches!(cell.obj, Obj::Player { stunned: false }))
		{
			let dst_coords = coords + dd;
			match action {
				PlayerAction::Move => {
					if level
						.grid
						.get(dst_coords)
						.is_some_and(|cell| !matches!(cell.groud, Ground::Water))
					{
						if !matches!(level.grid.get(dst_coords).unwrap().obj, Obj::Empty) {
							try_push(&mut level.grid, dst_coords, dd);
						}
						if matches!(level.grid.get(dst_coords).unwrap().obj, Obj::Empty) {
							level.grid.get_mut(coords).unwrap().obj = Obj::Empty;
							level.grid.get_mut(dst_coords).unwrap().obj = Obj::Player { stunned: false };
						}
					}
				},
				PlayerAction::PlaceTower { variant } => {
					if level.remaining_towers.is_some_and(|count| count == 0) {
						// We can't place a tower if we have no more towers to place.
					} else if level.grid.get(dst_coords).is_some_and(|cell| {
						matches!(cell.obj, Obj::Empty) && !matches!(cell.groud, Ground::Water)
					}) {
						level.grid.get_mut(dst_coords).unwrap().obj =
							Obj::Tower { variant, stunned: false };
						if let Some(count) = &mut level.remaining_towers {
							*count -= 1;
						}
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
						) && matches!(
							cell.obj,
							Obj::Empty | Obj::Goal | Obj::Tower { .. } | Obj::Rock
						)
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
			.is_some_and(|cell| matches!(cell.obj, Obj::Tower { .. }))
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

fn apply_events(level: &mut LevelState) {
	for event in level.events.iter_mut().filter(|e| e.turn == level.turn) {
		match event.event_type {
			GameEventType::EnemySpawn(coords) => {
				if let Some(tile) = level.grid.get_mut(coords) {
					match tile.obj {
						Obj::Empty | Obj::Player { .. } => tile.obj = Obj::new_enemy(Enemy::Basic),
						// Can't place enemy
						_ => event.turn += 1,
					}
				}
			},
		}
	}
}

fn parse_tile(tile_string: [char; 2]) -> Cell {
	let mut cell = Cell { obj: Obj::Empty, groud: Ground::Grass, rocky_path: false };
	cell.groud = match tile_string[0] {
		'O' => Ground::Grass,
		'x' => Ground::Water,
		'|' => Ground::Path(-1),
		_ => panic!(
			"Gwound fowmat '{}{}' incowect >w<",
			tile_string[0], tile_string[1]
		),
	};
	cell.obj = match tile_string[1] {
		'-' => Obj::Empty,
		'p' => Obj::Player { stunned: false },
		't' => Obj::Tower { variant: Tower::Basic, stunned: false },
		'e' => Obj::new_enemy(Enemy::Basic),
		'g' => Obj::Goal,
		'r' => Obj::Rock,
		'T' => Obj::Tree,
		_ => panic!(
			"Obwect fowmat '{}{}' incowect >w<",
			tile_string[0], tile_string[1]
		),
	};
	cell
}

fn load_level(level_file: &str) -> std::io::Result<LevelData> {
	let level_raw_data = fs::read_to_string(level_file)?;
	let filt = |x: &&str| !x.is_empty() && !x.starts_with('@') && !x.starts_with('~');
	let grid_h = level_raw_data.split('\n').filter(filt).count();
	let grid_w = level_raw_data
		.split('\n')
		.find(filt)
		.unwrap()
		.split(char::is_whitespace)
		.count();
	let dims = Dimensions { w: grid_w as i32, h: grid_h as i32 };
	let mut grid: Grid<Cell> = Grid::new(
		dims,
		Cell { obj: Obj::Empty, groud: Ground::Grass, rocky_path: false },
	);
	let mut cells_info = level_raw_data.split(char::is_whitespace);
	let mut h: HashMap<char, Coords> = HashMap::new();
	for coords in grid.dims.iter() {
		let current_tile = cells_info.next().unwrap();
		if current_tile.is_empty() {
			panic!("Tile empty, may have a blank space at the end of line or two spaces");
		}
		let cell = grid.get_mut(coords).unwrap();
		if current_tile.starts_with('?') {
			h.insert(current_tile.chars().nth(1).unwrap(), coords);
		} else {
			let mut tile = current_tile.chars();
			let c1 = tile.next().unwrap();
			let c2 = tile.next().unwrap();
			*cell = parse_tile([c1, c2]);
		}
	}
	let mut level_data = LevelData::new(grid);
	let meta_data = level_raw_data
		.split('\n')
		.filter_map(|x| x.strip_prefix('@'));
	for line in meta_data {
		let mut line = line.split(char::is_whitespace);
		match line.next().unwrap() {
			"max_towers" => level_data.max_towers = Some(line.next().unwrap().parse().unwrap()),
			"tile" => {
				let name = line.next().unwrap();
				let coords = h.get(&name.chars().next().unwrap()).unwrap();
				let mut tile = line.next().unwrap().chars();
				let c1 = tile.next().unwrap();
				let c2 = tile.next().unwrap();
				*level_data.init_grid.get_mut(*coords).unwrap() = parse_tile([c1, c2]);
			},
			"event" => match line.next().unwrap() {
				"spawn" => match line.next().unwrap() {
					"enemy" => {
						let tile_name = line.next().unwrap().chars().next().unwrap();
						let tile_coords = h.get(&tile_name).unwrap();
						let turn: u32 = line.next().unwrap().parse().unwrap();
						level_data.init_events.push(GameEvent::new(
							turn,
							GameEventType::EnemySpawn(*tile_coords),
						));
						// println!("OH THE MISERY Everybody wants to be my enemy");
					},
					creature => panic!("UwU, trying to spawn {creature} but it doesn't exist"),
				},
				other_event => panic!("Nyoooo unknown event {other_event}"),
			},
			unknown_meta_data_name => panic!("Jaaj {unknown_meta_data_name}??"),
		}
	}
	println!("max_towers: {x:?}", x = level_data.max_towers);
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
	let mut level = LevelState::new(&level_data);
	_print_dist(&level.grid);

	let cell_pixel_side = 8 * 8;

	let window = winit::window::WindowBuilder::new()
		.with_title("Prototype 7")
		.with_inner_size(winit::dpi::PhysicalSize::new(
			(level.grid.dims.w * cell_pixel_side) as u32,
			(level.grid.dims.h * cell_pixel_side) as u32,
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
					PlayerAction::PlaceTower { variant: Tower::Basic }
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
				player_move(&mut level, dxdy, action);
				if !level.game_joever {
					enemies_move(&mut level.grid);
					level.game_joever = is_game_joever(&level.grid);
					if level.game_joever {
						return;
					}
					towers_move(&mut level.grid);
					level.turn += 1;
					apply_events(&mut level);
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

			for coords in level.grid.dims.iter() {
				let dst = Rect::tile(coords, cell_pixel_side);
				let sprite = match level.grid.get(coords).unwrap().groud {
					Ground::Grass => (5, 0),
					Ground::Water => (6, 0),
					Ground::Path(_) => (7, 0),
				};
				let sprite_rect = Rect::tile(sprite.into(), 8);
				draw_sprite(
					&mut pixel_buffer,
					pixel_buffer_dims,
					dst,
					&spritesheet,
					sprite_rect,
				);
				let sprite = match level.grid.get(coords).unwrap().obj {
					Obj::Empty => None,
					Obj::Player { .. } => Some((0, 2)),
					Obj::Goal => Some((1, 2)),
					Obj::Enemy { variant: Enemy::Basic, .. } => Some((2, 2)),
					Obj::Enemy { variant: Enemy::Tank, .. } => Some((2, 3)),
					Obj::Enemy { variant: Enemy::Speeeeed, .. } => Some((2, 4)),
					Obj::Enemy { variant: Enemy::Stuner, .. } => Some((2, 5)),
					Obj::Enemy { variant: Enemy::Eater, .. } => Some((2, 6)),
					Obj::Enemy { variant: Enemy::Protected { .. }, .. } => todo!("Aaaa"),
					Obj::Tower { variant: Tower::Basic, .. } => Some((3, 2)),
					Obj::Tower { variant: Tower::Piercing, .. } => Some((3, 3)),
					Obj::Tower { variant: Tower::TotalEnergy, .. } => Some((3, 4)),
					Obj::Tower { variant: Tower::Bomber, .. } => Some((3, 5)),
					Obj::Tower { variant: Tower::Pusher, .. } => Some((3, 6)),
					Obj::Bomb { .. } => Some((4, 5)),
					Obj::Flower { variant: Flower::BlueFlower } => Some((6, 2)),
					Obj::Flower { variant: Flower::TheOther } => Some((7, 2)),
					Obj::Rock => Some((8, 2)),
					Obj::Tree => Some((9, 2)),
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
				if let Obj::Enemy { variant, hp, .. } = &level.grid.get(coords).unwrap().obj {
					// Draw a life bar
					let mut dst = Rect::tile(coords, cell_pixel_side);
					dst.top_left.y += cell_pixel_side / 8;
					dst.dims.h = cell_pixel_side / 8;
					dst.top_left.x += cell_pixel_side / 8;
					dst.dims.w = cell_pixel_side * 6 / 8;
					draw_rect(&mut pixel_buffer, pixel_buffer_dims, dst, [255, 0, 0, 255]);
					dst.dims.w = (cell_pixel_side * 6 / 8) * *hp as i32 / variant.hp_max() as i32;
					draw_rect(&mut pixel_buffer, pixel_buffer_dims, dst, [0, 255, 0, 255]);
				}
			}

			if level.game_joever {
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
