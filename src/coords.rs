//! Everything purely related to coordinates, grids, and such stuff.

#[derive(Clone, Copy)]
pub struct Dimensions {
	pub w: i32,
	pub h: i32,
}

impl From<winit::dpi::PhysicalSize<u32>> for Dimensions {
	fn from(size: winit::dpi::PhysicalSize<u32>) -> Dimensions {
		Dimensions { w: size.width as i32, h: size.height as i32 }
	}
}

impl Dimensions {
	pub fn square(side: i32) -> Dimensions {
		Dimensions { w: side, h: side }
	}

	pub fn area(self) -> i32 {
		self.w * self.h
	}

	pub fn contains(self, coords: Coords) -> bool {
		0 <= coords.x && coords.x < self.w && 0 <= coords.y && coords.y < self.h
	}

	pub fn index_of_coords(self, coords: Coords) -> Option<usize> {
		if self.contains(coords) {
			Some((coords.y * self.w + coords.x) as usize)
		} else {
			None
		}
	}
}

impl Dimensions {
	pub fn iter(self) -> IterCoordsRect {
		IterCoordsRect::with_rect(Rect { top_left: (0, 0).into(), dims: self })
	}
}

pub struct IterCoordsRect {
	current: Coords,
	rect: Rect,
}
impl IterCoordsRect {
	pub fn with_rect(rect: Rect) -> IterCoordsRect {
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
pub struct Grid<T> {
	pub dims: Dimensions,
	content: Vec<T>,
}

impl<T: Clone> Grid<T> {
	pub fn new(dims: Dimensions, value: T) -> Grid<T> {
		Grid {
			dims,
			content: std::iter::repeat(value)
				.take(dims.area() as usize)
				.collect(),
		}
	}
}

impl<T> Grid<T> {
	pub fn get(&self, coords: Coords) -> Option<&T> {
		if let Some(index) = self.dims.index_of_coords(coords) {
			self.content.get(index)
		} else {
			None
		}
	}
	pub fn get_mut(&mut self, coords: Coords) -> Option<&mut T> {
		if let Some(index) = self.dims.index_of_coords(coords) {
			self.content.get_mut(index)
		} else {
			None
		}
	}
}

#[derive(Clone, Copy)]
pub struct Coords {
	pub x: i32,
	pub y: i32,
}

#[derive(Clone, Copy)]
pub struct DxDy {
	pub dx: i32,
	pub dy: i32,
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
	pub fn the_4_directions() -> impl Iterator<Item = DxDy> {
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
pub struct Rect {
	pub top_left: Coords,
	pub dims: Dimensions,
}

impl Rect {
	pub fn tile(coords: Coords, tiles_side: i32) -> Rect {
		Rect {
			top_left: Coords { x: coords.x * tiles_side, y: coords.y * tiles_side },
			dims: Dimensions::square(tiles_side),
		}
	}

	pub fn top(self) -> i32 {
		self.top_left.y
	}
	pub fn left(self) -> i32 {
		self.top_left.x
	}
	pub fn bottom_excluded(self) -> i32 {
		self.top_left.y + self.dims.h
	}
	pub fn right_excluded(self) -> i32 {
		self.top_left.x + self.dims.w
	}

	pub fn contains(self, coords: Coords) -> bool {
		self.left() <= coords.x
			&& coords.x < self.right_excluded()
			&& self.top() <= coords.y
			&& coords.y < self.bottom_excluded()
	}

	pub fn iter(self) -> IterCoordsRect {
		IterCoordsRect::with_rect(self)
	}
}
