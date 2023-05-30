use core::slice::Iter;
use std::fmt::Debug;

use graphics::{types::Color, Context};
use opengl_graphics::GlGraphics;
use petgraph::{graph::NodeIndex, prelude::DiGraph, stable_graph::DefaultIx, visit::EdgeRef};
use piston::RenderArgs;
use serde::{Deserialize, Serialize};
use shapes::Point;

use crate::{
    error::RenderResult,
    rendering::{registry::TextureRegistry, Camera, View},
    types::{physbox::PhysBox, BoxEdge, Solidity, NO_SOLIDITY, SOLID},
    HasBox, HasBoxMut, HasSolidity, PhysicsError, PhysicsResult,
};

/// A graph of `Cell`s on the field, with the edges between them being the
/// physical side of the cell where the edge appears, and whether or not
/// that edge is passable by solidity entering/exiting rules
pub type FieldGraphInner = DiGraph<Cell, (BoxEdge, bool)>;

/// A field is a graph of rectangles ([cells](Cell)) that aids in movement within a scene
///
/// The cells have a [solidity](Solidity) field which dictates
/// how standard entities can move between cells.
/// In the construction of a field from the given cells, their solidity is
/// used to construct a graph for determining if inter-cell movement is possible
///
/// For simplicity, it's assumed that intra-cell movement is always possible
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Field {
    graph: FieldGraphInner,
}

impl Field {
    /// Given a list of cells, turns them into a field
    pub fn new(cells: Vec<Cell>) -> PhysicsResult<Self> {
        let mut graph = FieldGraphInner::new();

        for cell in cells {
            let i = graph.add_node(cell);
            graph.node_weight_mut(i).map(|c| c.i = i);
        }

        Field::build_cells(&mut graph)?;

        // TODO: potential validation steps:
        // - ensure that cells dont overlap (probably should be done before the edges are built)
        //     it most likely can't be done exhaustively < O(n^2)
        //     but I don't know if that's the end of the world since this is only done
        //     on level loading
        // - ensure there are no gaps in between cells
        //     this would probably take a very long time unless I can come up with a clever alg

        Ok(Self { graph })
    }

    fn build_cells(graph: &mut FieldGraphInner) -> PhysicsResult<()> {
        // Find the bordering cells along the given edge,
        // and mark the appropriate graph edges
        // c: the current cell
        // test_pos: mut TileVec.
        //     Must be initialized to `TileVec(c.pos.0 + off_x, c.pos.1 + off_y)`
        //     where `off_x` is 0 when `direction` is 0 or c.size.0 otherwise and
        //     `off_y` is 0 when `direction is 1 or c.size.1 otherwise.
        //     This part is not pulled into the macro due to some complications
        //     with typing because TileVec is unsigned.
        // edge: A `BoxEdge` variant
        fn cell_edges(
            this_cell_idx: NodeIndex,
            graph: &mut FieldGraphInner,
            test_pos: Point,
            edge: BoxEdge,
        ) -> PhysicsResult<()> {
            let mut test_pos = test_pos.clone();

            let this_cell_far_axis = Field::cell_at_idx(graph, this_cell_idx)?
                .physbox
                .get_far_axis(edge.parallel_axis());

            // We iterate in the direction is orthogonal to edge's axis
            while edge.parallel_axis().component_of_point(&test_pos) < this_cell_far_axis {
                // Find the cell at the current 'test_pos'. If it exists,
                // add the edge it's on to neighbors along with wither or not the
                // edge is typically passable (the corresponding edge can be entered
                // or exited for each cell)
                // Then set the new test pos to the far end of the neighbor
                let new_normal_component = if let Some(cell_at_test_pos) =
                    Field::cell_at_pos_internal(graph.node_weights(), test_pos)
                {
                    let new_normal_component =
                        cell_at_test_pos.physbox.get_far_axis(edge.parallel_axis());
                    let edge_is_passable = Field::cell_at_idx(graph, this_cell_idx)?
                        .solidity
                        .exit_edge(edge)
                        && cell_at_test_pos.solidity.enter_edge(edge.opposite());

                    // Rust gives a warning about derefing 'graph' in a call to graph
                    // if we put it immediately in the function call
                    let cell_at_test_pos_idx = cell_at_test_pos.i;
                    graph.update_edge(
                        this_cell_idx,
                        cell_at_test_pos_idx,
                        (edge, edge_is_passable),
                    );
                    new_normal_component
                } else {
                    edge.parallel_axis().component_of_point(&test_pos) + 1.0
                };

                test_pos = match edge {
                    BoxEdge::Top | BoxEdge::Bottom => Point {
                        x: new_normal_component,
                        y: test_pos.y,
                    },
                    BoxEdge::Left | BoxEdge::Right => Point {
                        x: test_pos.x,
                        y: new_normal_component,
                    },
                };
            }

            Ok(())
        }

        // Initialize the neighbors and edges
        for cell_idx in graph.node_indices() {
            let physbox = Field::cell_at_idx(graph, cell_idx)?.physbox;
            let mut test_pos;

            // Along the top edge
            test_pos = *physbox.pos() - [0.0, 1.0];
            cell_edges(cell_idx, graph, test_pos, BoxEdge::Top)?;

            // Along the left edge
            test_pos = *physbox.pos() - [1.0, 0.0];
            cell_edges(cell_idx, graph, test_pos, BoxEdge::Left)?;

            // Along the bottom edge
            test_pos = [physbox.left_x(), physbox.bottom_y()].into();
            cell_edges(cell_idx, graph, test_pos, BoxEdge::Bottom)?;

            // Along the right edge
            test_pos = [physbox.right_x(), physbox.top_y()].into();
            cell_edges(cell_idx, graph, test_pos, BoxEdge::Right)?;
        }

        Ok(())
    }

    fn cell_at_idx(graph: &FieldGraphInner, idx: NodeIndex) -> PhysicsResult<&Cell> {
        graph
            .node_weight(idx)
            .ok_or_else(|| PhysicsError::FieldIndex(idx.index()))
    }

    fn cell_at_pos_internal<'a, I: Iterator<Item = &'a Cell>>(
        cells: I,
        pos: Point,
    ) -> Option<&'a Cell> {
        // The real challenge of this will be to try and do it in less than O(n)
        // I could see some sort of spanning tree setup to do this more quickly
        // in as little as O(log(n))
        for c in cells {
            if c.physbox.contains_pos(pos) {
                return Some(c);
            }
        }
        None
    }

    /// Returns the cell at the given point on the field if any exist
    pub fn cell_at_pos(&self, pos: Point) -> Option<&Cell> {
        Field::cell_at_pos_internal(self.graph.node_weights(), pos)
    }

    /// Given a cell on the field and a physbox, returns the neighbors of
    /// the cell that the physbox overlaps.
    pub fn neighbors_of_cell_overlapping_box(
        &self,
        cell: &Cell,
        physbox: &PhysBox,
    ) -> PhysicsResult<CellNeighbors> {
        let mut neighbors = CellNeighbors::default();

        for graph_edge in self.graph.edges(cell.i) {
            let neighbor = Field::cell_at_idx(&self.graph, graph_edge.target())?;
            if physbox.has_overlap(&neighbor.physbox) {
                neighbors.add_neighbor(neighbor, graph_edge.weight().0);
            }
        }

        Ok(neighbors)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
/// Renders a Field, coloring all cells with a color determined by their solidity
/// i.e. all cells of a unique solidity are a single color
pub struct FieldColorView {
    /// The color view for completely solid cells
    pub solid_view: CellColorView,
    /// The color view for completely passable cells
    pub air_view: CellColorView,
    /// The color view for all other cells
    pub default_view: CellColorView,
}

impl FieldColorView {
    fn view_for_cell(&mut self, cell: &Cell) -> &mut CellColorView {
        match cell.solidity {
            SOLID => &mut self.solid_view,
            NO_SOLIDITY => &mut self.air_view,
            _ => &mut self.default_view,
        }
    }
}

impl View for FieldColorView {
    type Viewed = Field;

    fn render(
        &mut self,
        viewed: &Self::Viewed,
        args: &RenderArgs,
        camera: &Camera,
        ctx: Context,
        texture_registry: &TextureRegistry,
        gl: &mut GlGraphics,
    ) -> RenderResult<()> {
        for cell in viewed.graph.node_weights() {
            let cell_view = self.view_for_cell(cell);
            cell_view.render(cell, args, camera, ctx, texture_registry, gl)?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, HasBox, HasBoxMut, HasSolidity)]
/// Represents a static area on a Field that determines the passability for other standard entities
pub struct Cell {
    /// This cell's index in the Field's `Vec<Cell>`
    i: NodeIndex<DefaultIx>,
    #[has_solidity]
    /// Defines how entities can move into/out of this cell
    solidity: Solidity,
    #[has_box]
    /// The upper left corner and width/height of the cell
    physbox: PhysBox,
}

impl Cell {
    /// Creates a new cell with the given Solidity and PhysBox
    pub fn new(solidity: Solidity, physbox: PhysBox) -> Self {
        Self {
            i: NodeIndex::new(0),
            solidity,
            physbox,
        }
    }
}

/// Represents the neighbors of a cell organized by what edge the neighbor is on
#[derive(Debug, Default, Clone, PartialEq)]
pub struct CellNeighbors<'a> {
    top: Vec<&'a Cell>,
    left: Vec<&'a Cell>,
    bottom: Vec<&'a Cell>,
    right: Vec<&'a Cell>,
}

impl<'a> CellNeighbors<'a> {
    /// Adds a neighbor to this cell along the given edge
    pub fn add_neighbor(&mut self, neighbor: &'a Cell, edge: BoxEdge) {
        match edge {
            BoxEdge::Top => self.top.push(neighbor),
            BoxEdge::Left => self.left.push(neighbor),
            BoxEdge::Bottom => self.bottom.push(neighbor),
            BoxEdge::Right => self.right.push(neighbor),
        }
    }

    /// Gets all neighbors of the cell that touch the given edge
    pub fn get_neighbors(&self, edge: BoxEdge) -> &Vec<&'a Cell> {
        match edge {
            BoxEdge::Top => &self.top,
            BoxEdge::Left => &self.left,
            BoxEdge::Bottom => &self.bottom,
            BoxEdge::Right => &self.right,
        }
    }

    /// Iterates across all neighbors of the cell, grouped by the touched edge
    pub fn iter_by_edge(&self) -> CellNeighborsIterByEdge {
        CellNeighborsIterByEdge {
            current_edge: BoxEdge::iter(),
            inner: &self,
        }
    }

    /// Iterates across all neighbors of the cell, not grouped by the touched edge
    pub fn iter_all(&self) -> std::vec::IntoIter<&Cell> {
        let mut all_vec = vec![];
        all_vec.extend_from_slice(&self.top);
        all_vec.extend_from_slice(&self.left);
        all_vec.extend_from_slice(&self.bottom);
        all_vec.extend_from_slice(&self.right);
        all_vec.into_iter()
    }
}

impl<'a> From<Vec<(BoxEdge, &'a Cell)>> for CellNeighbors<'a> {
    fn from(val: Vec<(BoxEdge, &'a Cell)>) -> Self {
        let mut neighbors = CellNeighbors::default();

        for (edge, cell) in val {
            neighbors.add_neighbor(cell, edge);
        }

        neighbors
    }
}

/// Iterates across the neighbors of a cell as grouped by the edge that the neighbors are on
///
/// Each item is a tuple `(edge, neighbors): (BoxEdge, &'a Vec<&'a Cell>)`. Where the BoxEdge is the edge of the source cell,
/// and the `neighbors` is the list of all other [Cell]s that touch the source cell on the source's `edge
pub struct CellNeighborsIterByEdge<'a> {
    current_edge: Iter<'a, BoxEdge>,
    inner: &'a CellNeighbors<'a>,
}

impl<'a> Iterator for CellNeighborsIterByEdge<'a> {
    type Item = (BoxEdge, &'a Vec<&'a Cell>);

    // TODO: there's a chance for slight optimizations here to avoid
    // the cloning of the vec.
    fn next(&mut self) -> Option<Self::Item> {
        self.current_edge
            .next()
            .map(|edge| (edge.to_owned(), self.inner.get_neighbors(*edge)))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
/// Renders the cell by filling its PhysBox with the given color
pub struct CellColorView {
    /// The color to render the cell with
    pub color: Color,
}

impl View for CellColorView {
    type Viewed = Cell;

    fn render(
        &mut self,
        viewed: &Self::Viewed,
        _args: &RenderArgs,
        camera: &Camera,
        ctx: Context,
        _texture_registry: &TextureRegistry,
        gl: &mut GlGraphics,
    ) -> RenderResult<()> {
        if let Some((transform, rect)) = camera.box_renderables(&viewed.physbox, ctx) {
            graphics::rectangle(self.color, rect, transform, gl);
        }

        Ok(())
    }
}

#[cfg(test)]
mod test {
    use crate::types::physbox::HasBox;

    use super::*;

    fn create_test_field() -> (Vec<PhysBox>, Field) {
        let boxes = vec![
            PhysBox::new([0.0, 0.0, 10.0, 20.0]).unwrap(),
            PhysBox::new([10.0, 10.0, 20.0, 30.0]).unwrap(),
            PhysBox::new([10.0, 0.0, 40.0, 10.0]).unwrap(),
            PhysBox::new([30.0, 10.0, 20.0, 50.0]).unwrap(),
            PhysBox::new([0.0, 40.0, 30.0, 20.0]).unwrap(),
            PhysBox::new([0.0, 20.0, 10.0, 20.0]).unwrap(),
            PhysBox::new([50.0, 0.0, 1.0, 60.0]).unwrap(),
            PhysBox::new([0.0, 60.0, 50.0, 1.0]).unwrap(),
            PhysBox::new([0.0, -1.0, 50.0, 1.0]).unwrap(),
            PhysBox::new([-1.0, 0.0, 1.0, 60.0]).unwrap(),
        ];

        let cell0 = Cell::new(SOLID, boxes[0].clone());
        let cell1 = Cell::new(SOLID, boxes[1].clone());
        let cell2 = Cell::new(NO_SOLIDITY, boxes[2].clone());
        let cell3 = Cell::new(NO_SOLIDITY, boxes[3].clone());
        let cell4 = Cell::new(NO_SOLIDITY, boxes[4].clone());
        let cell5 = Cell::new(NO_SOLIDITY, boxes[5].clone());
        let cell6 = Cell::new(SOLID, boxes[6].clone());
        let cell7 = Cell::new(SOLID, boxes[7].clone());
        let cell8 = Cell::new(SOLID, boxes[8].clone());
        let cell9 = Cell::new(SOLID, boxes[9].clone());

        let field = Field::new(vec![
            cell0, cell1, cell2, cell3, cell4, cell5, cell6, cell7, cell8, cell9,
        ])
        .unwrap();

        (boxes, field)
    }

    #[test]
    fn cell_at_pos_works() {
        let (boxes, field) = create_test_field();

        for physbox in &boxes {
            assert_eq!(
                field.cell_at_pos(*physbox.pos()).unwrap().get_box(),
                physbox
            )
        }

        assert_eq!(
            field
                .cell_at_pos(*boxes[0].pos() + [5.0, 5.0])
                .unwrap()
                .get_box(),
            &boxes[0]
        );

        assert_eq!(
            field
                .cell_at_pos(*boxes[1].pos() + [5.0, 5.0])
                .unwrap()
                .get_box(),
            &boxes[1]
        );

        assert!(field.cell_at_pos([-1000.0, -1000.0].into()).is_none())
    }

    #[test]
    fn neighbors_of_cell_overlapping_box_works_with_cell_physboxes() {
        let (boxes, field) = create_test_field();

        for physbox in &boxes {
            let cell_at = field.cell_at_pos(*physbox.pos()).unwrap();
            assert_eq!(cell_at.get_box(), physbox);

            let neighbors = field
                .neighbors_of_cell_overlapping_box(cell_at, physbox)
                .unwrap();
            assert_eq!(neighbors, CellNeighbors::default());
        }
    }

    #[test]
    fn neighbors_of_cell_overlapping_box_works_in_middle_of_cell() {
        let (boxes, field) = create_test_field();

        let testbox = PhysBox::new([31.0, 21.0, 8.0, 8.0]).unwrap();
        let cell_at = field.cell_at_pos(*testbox.pos()).unwrap();
        assert_eq!(cell_at.get_box(), &boxes[3]);

        let neighbors = field
            .neighbors_of_cell_overlapping_box(cell_at, &testbox)
            .unwrap();
        assert_eq!(neighbors, CellNeighbors::default());
    }

    #[test]
    fn neighbors_of_cell_overlapping_box_works_on_right_edge() {
        let (boxes, field) = create_test_field();

        // Shift it over -6 x
        let testbox = PhysBox::new([25.0, 21.0, 8.0, 8.0]).unwrap();
        let cell_at = field.cell_at_pos(*testbox.pos()).unwrap();
        assert_eq!(cell_at.get_box(), &boxes[1]);

        let neighbors = field
            .neighbors_of_cell_overlapping_box(cell_at, &testbox)
            .unwrap();

        // It only has an overlapping neighbor on the new box's right
        assert_eq!(neighbors.get_neighbors(BoxEdge::Top), &Vec::<&Cell>::new());
        assert_eq!(neighbors.get_neighbors(BoxEdge::Left), &Vec::<&Cell>::new());
        assert_eq!(
            neighbors.get_neighbors(BoxEdge::Bottom),
            &Vec::<&Cell>::new()
        );
        assert_eq!(
            neighbors.get_neighbors(BoxEdge::Right),
            &vec![field.cell_at_pos(*boxes[3].pos()).unwrap()]
        );
    }

    #[test]
    fn neighbors_of_cell_overlapping_box_works_on_bottom_edge() {
        let (boxes, field) = create_test_field();

        // Shift it up -13 y
        let testbox = PhysBox::new([31.0, 8.0, 8.0, 8.0]).unwrap();
        let cell_at = field.cell_at_pos(*testbox.pos()).unwrap();
        assert_eq!(cell_at.get_box(), &boxes[2]);

        let neighbors = field
            .neighbors_of_cell_overlapping_box(cell_at, &testbox)
            .unwrap();

        // It only has an overlapping neighbor on the bottom
        assert_eq!(neighbors.get_neighbors(BoxEdge::Top), &Vec::<&Cell>::new());
        assert_eq!(neighbors.get_neighbors(BoxEdge::Left), &Vec::<&Cell>::new());
        assert_eq!(
            neighbors.get_neighbors(BoxEdge::Bottom),
            &vec![field.cell_at_pos(*boxes[3].pos()).unwrap()]
        );
        assert_eq!(
            neighbors.get_neighbors(BoxEdge::Right),
            &Vec::<&Cell>::new()
        );
    }

    #[test]
    fn neighbors_of_cell_overlapping_box_works_on_bottom_right_corner() {
        let (boxes, field) = create_test_field();

        // Shift it left -6 x and down +15 y
        let testbox = PhysBox::new([25.0, 36.0, 8.0, 8.0]).unwrap();
        let cell_at = field.cell_at_pos(*testbox.pos()).unwrap();
        assert_eq!(cell_at.get_box(), &boxes[1]);

        let neighbors = field
            .neighbors_of_cell_overlapping_box(cell_at, &testbox)
            .unwrap();

        // It has an overlapping neighbor on the bottom and right
        assert_eq!(neighbors.get_neighbors(BoxEdge::Top), &Vec::<&Cell>::new());
        assert_eq!(neighbors.get_neighbors(BoxEdge::Left), &Vec::<&Cell>::new());
        assert_eq!(
            neighbors.get_neighbors(BoxEdge::Bottom),
            &vec![field.cell_at_pos(*boxes[4].pos()).unwrap()]
        );
        assert_eq!(
            neighbors.get_neighbors(BoxEdge::Right),
            &vec![field.cell_at_pos(*boxes[3].pos()).unwrap()]
        );
    }
}
