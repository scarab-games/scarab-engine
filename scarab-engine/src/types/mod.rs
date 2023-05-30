use core::ops::{Add, BitAnd, BitOr, Mul, Not, Sub};

use graphics::types::{Scalar, Vec2d};
use lazy_static::lazy_static;
use serde::{Deserialize, Serialize};
use shapes::Point;
pub use uuid::Uuid;

/// Stuff for rectangular physics items
pub mod physbox;

lazy_static! {
    /// Pre-calculate the square root of 2
    pub static ref ROOT_2: f64 = f64::sqrt(2.0);
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
/// Represents the edges of a rectangular game object
pub enum BoxEdge {
    /// The top edge of a game object (negative y)
    Top,
    /// The left edge of a game object (positive x)
    Left,
    /// The bottom edge of a game object (positive y)
    Bottom,
    /// The right edge of a game object (negative x)
    Right,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(usize)]
/// The axis system where game objects exist
pub enum Axis {
    /// The x-axis
    X,
    /// The y-axis
    Y,
}

impl Axis {
    /// Gets the component of the given point along this axis
    pub fn component_of_point(&self, point: &Point) -> Scalar {
        match self {
            Axis::X => point.x,
            Axis::Y => point.y,
        }
    }
}

impl BoxEdge {
    /// Returns the BoxEdge opposite to this one
    pub fn opposite(&self) -> BoxEdge {
        match self {
            Self::Top => Self::Bottom,
            Self::Left => Self::Right,
            Self::Bottom => Self::Top,
            Self::Right => Self::Left,
        }
    }

    /// Retuns the vector that is perpendicular to this edge.
    /// This may also be defined as the vector from the center of a 2-unit square to the respective edge
    pub fn normal_vector(&self) -> [Scalar; 2] {
        match self {
            BoxEdge::Top => [0.0, -1.0],
            BoxEdge::Left => [-1.0, 0.0],
            BoxEdge::Bottom => [0.0, 1.0],
            BoxEdge::Right => [1.0, 0.0],
        }
    }

    /// The axis that runs perpendicular to this edge
    pub fn perpendicular_axis(&self) -> Axis {
        match self {
            BoxEdge::Top | BoxEdge::Bottom => Axis::Y,
            BoxEdge::Left | BoxEdge::Right => Axis::X,
        }
    }

    /// The axis that runs parallel to this edge
    pub fn parallel_axis(&self) -> Axis {
        match self {
            BoxEdge::Top | BoxEdge::Bottom => Axis::X,
            BoxEdge::Left | BoxEdge::Right => Axis::Y,
        }
    }

    /// An iterator over each of the four edges
    pub fn iter() -> core::slice::Iter<'static, BoxEdge> {
        static EDGES: [BoxEdge; 4] = [BoxEdge::Top, BoxEdge::Left, BoxEdge::Bottom, BoxEdge::Right];
        EDGES.iter()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
/// The velocity of a game object
pub struct Velocity {
    /// The x component of the velocity
    pub x: Scalar,
    /// The y component of the velocity
    pub y: Scalar,
}

impl Velocity {
    /// Returns a new velocity in the same direction as this velocity, but with a magnitude of 1
    pub fn normalize(self) -> Self {
        let mag = self.magnitude();
        if mag == 0.0 {
            Self { x: 0.0, y: 0.0 }
        } else {
            Self {
                x: self.x / mag,
                y: self.y / mag,
            }
        }
    }

    /// The magnitude of this velocity squared
    pub fn magnitude_sq(&self) -> Scalar {
        self.x * self.x + self.y * self.y
    }

    /// The magnitude of this velocity
    pub fn magnitude(&self) -> Scalar {
        f64::sqrt(self.x * self.x + self.y * self.y)
    }

    /// Whether this velocity would be reduced by colliding with a game object on the given edge
    /// i.e. is the dot product of the velocity and the edge's normal negative
    pub fn is_reduced_by_edge(&self, edge: BoxEdge) -> bool {
        match edge {
            BoxEdge::Top => self.y < 0.0,
            BoxEdge::Left => self.x < 0.0,
            BoxEdge::Bottom => self.y > 0.0,
            BoxEdge::Right => self.x > 0.0,
        }
    }

    /// Gives the angle of the velocity vector in radians
    /// with positive radians being from +x to +y. This would be clockwise on
    /// the display because +y is down.
    pub fn angle(&self) -> Scalar {
        f64::atan2(self.y, self.x)
    }
}

impl<T: Into<Velocity>> Add<T> for Velocity {
    type Output = Velocity;

    fn add(self, rhs: T) -> Self::Output {
        let rhs = rhs.into();
        Self {
            x: self.x + rhs.x,
            y: self.y + rhs.y,
        }
    }
}

impl<T: Into<Velocity>> Sub<T> for Velocity {
    type Output = Velocity;

    fn sub(self, rhs: T) -> Self::Output {
        let rhs = rhs.into();
        Self {
            x: self.x - rhs.x,
            y: self.y - rhs.y,
        }
    }
}

impl Add<Velocity> for Point {
    type Output = Point;

    fn add(self, vel: Velocity) -> Self::Output {
        Self {
            x: self.x + vel.x,
            y: self.y + vel.y,
        }
    }
}

impl Mul<Scalar> for Velocity {
    type Output = Velocity;

    fn mul(self, rhs: Scalar) -> Self::Output {
        Self {
            x: self.x * rhs,
            y: self.y * rhs,
        }
    }
}

impl From<Vec2d> for Velocity {
    fn from(v: Vec2d) -> Self {
        Self { x: v[0], y: v[1] }
    }
}

/// A trait for a gameobject that has a unique identifier
pub trait HasUuid {
    /// The object's unique identifier
    fn uuid(&self) -> Uuid;
}

impl HasUuid for Uuid {
    fn uuid(&self) -> Uuid {
        *self
    }
}

/// Represents whether the typical entity can enter/exit a cell from each side
///
/// The structure of a solidity is as follows:
/// The most signicant 4 bits represent the "exitability" for the game object
/// while the least significant 4 bits represent the "enterability".
/// Within each octet the bits from most to least significant are: left, right, top, bottom.
/// When a bit is 1 that means the edge can be entered/exited
///
/// However, it is likely easier add together the given constants in some way.
/// ```
/// use scarab_engine::types::{SOLID, ENTER_LEFT, ENTER_RIGHT, ENTER_TOP, ENTER_BOTTOM};
/// // A solidity that can do anything except enter on the left
/// let cant_enter_left = !ENTER_LEFT;
/// assert!(!cant_enter_left.enter_left());
/// assert!(cant_enter_left.exit_left());
///
/// // A solidity that can be entered from any edge, but can't exit any edge
/// let enter_all_cant_exit = ENTER_LEFT | ENTER_RIGHT | ENTER_TOP | ENTER_BOTTOM;
/// assert!(enter_all_cant_exit.enter_left());
/// assert!(enter_all_cant_exit.enter_right());
/// assert!(enter_all_cant_exit.enter_top());
/// assert!(enter_all_cant_exit.enter_bottom());
/// assert!(!enter_all_cant_exit.exit_left());
/// assert!(!enter_all_cant_exit.exit_right());
/// assert!(!enter_all_cant_exit.exit_top());
/// assert!(!enter_all_cant_exit.exit_bottom());
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Solidity(pub u8);

/// Solidity for game objects that can't be entered or exited from any side
pub const SOLID: Solidity = Solidity(0);
/// Solidity for game objects that can be entered or exited from any side
pub const NO_SOLIDITY: Solidity = Solidity(255);
/// Bitmask for solidities that can enter on the left
pub const ENTER_LEFT: Solidity = Solidity(0b0000_1000);
/// Bitmask for solidities that can enter on the left
pub const EXIT_LEFT: Solidity = Solidity(0b1000_0000);
/// Bitmask for solidities that can exit on the right
pub const ENTER_RIGHT: Solidity = Solidity(0b0000_0100);
/// Bitmask for solidities that can enter on the right
pub const EXIT_RIGHT: Solidity = Solidity(0b0100_0000);
/// Bitmask for solidities that can exit on the top
pub const ENTER_TOP: Solidity = Solidity(0b0000_0010);
/// Bitmask for solidities that can exit on the top
pub const EXIT_TOP: Solidity = Solidity(0b0010_0000);
/// Bitmask for solidities that can enter on the bottom
pub const ENTER_BOTTOM: Solidity = Solidity(0b0000_0001);
/// Bitmask for solidities that can exit on the bottom
pub const EXIT_BOTTOM: Solidity = Solidity(0b0001_0000);

/// For each function true means that edge can be passed
impl Solidity {
    /// Whether or not the left side of the attached object can be entered.
    #[inline]
    pub fn enter_left(&self) -> bool {
        self.0 & ENTER_LEFT.0 != 0
    }

    /// Whether or not the left side of the attached object can be exited.
    #[inline]
    pub fn exit_left(&self) -> bool {
        self.0 & EXIT_LEFT.0 != 0
    }

    /// Whether or not the right side of the attached object can be entered.
    #[inline]
    pub fn enter_right(&self) -> bool {
        self.0 & ENTER_RIGHT.0 != 0
    }

    /// Whether or not the right side of the attached object can be exited.
    #[inline]
    pub fn exit_right(&self) -> bool {
        self.0 & EXIT_RIGHT.0 != 0
    }

    /// Whether or not the top side of the attached object can be entered.
    #[inline]
    pub fn enter_top(&self) -> bool {
        self.0 & ENTER_TOP.0 != 0
    }

    /// Whether or not the top side of the attached object can be exited.
    #[inline]
    pub fn exit_top(&self) -> bool {
        self.0 & EXIT_TOP.0 != 0
    }

    /// Whether or not the bottom side of the attached object can be entered.
    #[inline]
    pub fn enter_bottom(&self) -> bool {
        self.0 & ENTER_BOTTOM.0 != 0
    }

    /// Whether or not the bottom side of the attached object can be exited.
    #[inline]
    pub fn exit_bottom(&self) -> bool {
        self.0 & EXIT_BOTTOM.0 != 0
    }

    /// Whether the given edge of the attached object can be entered.
    pub fn enter_edge(&self, edge: BoxEdge) -> bool {
        match edge {
            BoxEdge::Top => self.enter_top(),
            BoxEdge::Left => self.enter_left(),
            BoxEdge::Bottom => self.enter_bottom(),
            BoxEdge::Right => self.enter_right(),
        }
    }

    /// Whether the given edge of the attached object can be exited.
    pub fn exit_edge(&self, edge: BoxEdge) -> bool {
        match edge {
            BoxEdge::Top => self.exit_top(),
            BoxEdge::Left => self.exit_left(),
            BoxEdge::Bottom => self.exit_bottom(),
            BoxEdge::Right => self.exit_right(),
        }
    }

    /// Returns true if there is any edge that can't be entered or exited
    pub fn has_solidity(&self) -> bool {
        self != &NO_SOLIDITY
    }
}

impl BitAnd<Solidity> for Solidity {
    type Output = Solidity;
    fn bitand(self, rhs: Solidity) -> Self::Output {
        Solidity(self.0 & rhs.0)
    }
}

impl BitOr<Solidity> for Solidity {
    type Output = Solidity;
    fn bitor(self, rhs: Solidity) -> Self::Output {
        Solidity(self.0 | rhs.0)
    }
}

impl Not for Solidity {
    type Output = Solidity;
    fn not(self) -> Self::Output {
        Solidity(!self.0)
    }
}

/// A trait for gameobjects that have a solidity component
pub trait HasSolidity {
    /// The game object's solidity component
    fn get_solidity(&self) -> &Solidity;
}

impl HasSolidity for Solidity {
    fn get_solidity(&self) -> &Solidity {
        self
    }
}

#[derive(Debug, Serialize, Deserialize)]
/// The health of a game object
pub struct Health {
    curr: Scalar,
    max: Scalar,
}

impl Health {
    /// Creates a new health, with the current value initialized at max.
    pub fn new(max: Scalar) -> Self {
        Self {
            curr: max,
            max: max,
        }
    }

    /// Apply a raw amount of damage.
    pub fn raw_damage(&mut self, amt: Scalar) {
        self.curr -= amt;
    }

    /// The current health value
    pub fn current(&self) -> Scalar {
        self.curr
    }

    /// The maximum health
    pub fn max(&self) -> Scalar {
        self.max
    }

    /// The current health as a fraction of max health
    pub fn fraction(&self) -> Scalar {
        self.curr / self.max
    }
}

/// A trait for gameobjects that have a health component
pub trait HasHealth {
    /// A reference to the game object's internal health
    fn get_health(&self) -> &Health;

    /// A mutable reference to the game object's internal health
    fn get_health_mut(&mut self) -> &mut Health;
}

impl HasHealth for Health {
    fn get_health(&self) -> &Health {
        self
    }

    fn get_health_mut(&mut self) -> &mut Health {
        self
    }
}
